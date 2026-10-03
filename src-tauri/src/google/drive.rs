use reqwest::{Client, Response};
use serde_json::Value;
use std::time::Duration;

use crate::config::AppConfig;
use crate::google::oauth;

fn http() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .expect("HTTP client")
}

async fn checked(res: Response) -> Result<Response, String> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    let body: Value = res.json().await.unwrap_or_default();
    let message = body["error"]["message"]
        .as_str()
        .unwrap_or("Request failed");
    let help = match status.as_u16() {
        401 => " Sign out and sign in to Google again.",
        403 => " If permission is missing, sign out and sign in again and allow Drive access. If the API is disabled, enable Google Drive API in the app's Google Cloud project.",
        _ => "",
    };
    Err(format!("Drive ({status}): {message}.{help}"))
}

async fn list_files(token: &str, query: &str, limit: usize) -> Result<Vec<Value>, String> {
    let res = http()
        .get("https://www.googleapis.com/drive/v3/files")
        .bearer_auth(token)
        .query(&[
            ("pageSize", limit.clamp(1, 100).to_string()),
            ("orderBy", "modifiedTime desc".into()),
            ("fields", "files(id,name,mimeType),nextPageToken".into()),
            ("q", query.into()),
            ("includeItemsFromAllDrives", "true".into()),
            ("supportsAllDrives", "true".into()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let data: Value = checked(res)
        .await?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(data["files"].as_array().cloned().unwrap_or_default())
}

pub async fn recent_files_summary(cfg: &AppConfig, limit: usize) -> Result<String, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let files = list_files(
        &token,
        "trashed=false and mimeType != 'application/vnd.google-apps.folder'",
        limit,
    )
    .await?;
    if files.is_empty() {
        return Ok("Drive connected; no files found.".into());
    }
    Ok(format!(
        "Recently modified Drive files (partial listing):\n{}",
        summarize_files(&token, &files).await
    ))
}

fn search_terms(message: &str) -> Vec<String> {
    let stop = [
        "a",
        "an",
        "the",
        "my",
        "me",
        "i",
        "you",
        "your",
        "can",
        "could",
        "would",
        "please",
        "find",
        "show",
        "read",
        "check",
        "open",
        "summarize",
        "summary",
        "about",
        "what",
        "whats",
        "is",
        "are",
        "in",
        "on",
        "of",
        "for",
        "to",
        "and",
        "with",
        "do",
        "does",
        "have",
        "access",
        "google",
        "drive",
        "file",
        "files",
        "document",
        "documents",
    ];
    let mut terms = Vec::new();
    for word in message.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_lowercase();
        if word.len() >= 2 && !stop.contains(&word.as_str()) && !terms.contains(&word) {
            terms.push(word);
        }
    }
    terms.truncate(12);
    terms
}

pub async fn search_files(cfg: &AppConfig, message: &str, limit: usize) -> Result<String, String> {
    let terms = search_terms(message);
    if terms.is_empty() {
        return Ok(
            "No specific file title or topic requested; use the recent file listing.".into(),
        );
    }
    let token = oauth::ensure_access_token(cfg).await?;
    let clauses: Vec<_> = terms
        .iter()
        .map(|term| format!("fullText contains '{term}'"))
        .collect();
    let query = format!(
        "trashed=false and mimeType != 'application/vnd.google-apps.folder' and ({})",
        clauses.join(" and ")
    );
    let mut files = list_files(&token, &query, limit).await?;
    if files.is_empty() && terms.len() > 1 {
        let query = format!(
            "trashed=false and mimeType != 'application/vnd.google-apps.folder' and ({})",
            clauses.join(" or ")
        );
        files = list_files(&token, &query, limit).await?;
    }
    if files.is_empty() {
        return Ok(format!("Drive search succeeded but no files matched these keywords: {}. Ask for an exact file title.", terms.join(", ")));
    }
    Ok(format!(
        "Drive keyword search results (partial):\n{}",
        summarize_files(&token, &files).await
    ))
}

async fn summarize_files(token: &str, files: &[Value]) -> String {
    let mut sections = Vec::new();
    for file in files {
        let name = file["name"].as_str().unwrap_or("(unnamed)");
        let mime = file["mimeType"].as_str().unwrap_or("");
        let id = file["id"].as_str().unwrap_or("");
        let content = match read_text(token, id, mime).await {
            Ok(Some(text)) => format!("Content excerpt (up to 8,000 characters):\n{}", text.chars().take(8000).collect::<String>()),
            Ok(None) => "File is accessible, but this file format cannot yet be read by Waypoint. Only its metadata is available.".into(),
            Err(e) => format!("File found, but its content could not be read: {e}"),
        };
        sections.push(format!("File: {name} ({mime})\n{content}"));
    }
    sections.join("\n\n")
}

async fn read_text(token: &str, id: &str, mime: &str) -> Result<Option<String>, String> {
    let base = format!(
        "https://www.googleapis.com/drive/v3/files/{}",
        urlencoding::encode(id)
    );
    let client = http();
    let request = match mime {
        "application/vnd.google-apps.document" | "application/vnd.google-apps.presentation" => {
            client
                .get(format!("{base}/export"))
                .query(&[("mimeType", "text/plain")])
        }
        "application/vnd.google-apps.spreadsheet" => client
            .get(format!("{base}/export"))
            .query(&[("mimeType", "text/csv")]),
        m if m.starts_with("text/") => client
            .get(&base)
            .query(&[("alt", "media"), ("supportsAllDrives", "true")]),
        _ => return Ok(None),
    };
    let res = request
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut res = checked(res).await?;
    // Bound downloads, including text files with no Content-Length header.
    let mut bytes = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        let remaining = 64 * 1024 - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if bytes.len() == 64 * 1024 {
            break;
        }
    }
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if mime == "application/vnd.google-apps.spreadsheet" {
        text.insert_str(0, "[First sheet only]\n");
    }
    Ok(Some(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_language_search_extracts_topic_instead_of_entire_sentence() {
        assert_eq!(
            search_terms("Can you read my Google Drive files about CHEM 2070?"),
            vec!["chem", "2070"]
        );
        assert!(search_terms("Can you access my Google Drive files?").is_empty());
    }

    #[test]
    fn search_input_cannot_inject_drive_query() {
        let terms = search_terms("biology' \\ or trashed = true");
        assert!(terms.iter().all(|t| t.chars().all(char::is_alphanumeric)));
    }
}
