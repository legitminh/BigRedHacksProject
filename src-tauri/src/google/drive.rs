use reqwest::Client;
use serde_json::Value;

use crate::config::AppConfig;
use crate::google::oauth;

pub async fn recent_files_summary(cfg: &AppConfig, limit: usize) -> Result<String, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let url = format!(
        "https://www.googleapis.com/drive/v3/files?pageSize={}&orderBy=viewedByMeTime%20desc&fields=files(id,name,mimeType,modifiedTime,viewedByMeTime)&q=trashed=false",
        limit
    );
    let res = Client::new()
        .get(url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!(
            "Drive API error: {} {}",
            res.status(),
            res.text().await.unwrap_or_default()
        ));
    }
    let data: Value = res.json().await.map_err(|e| e.to_string())?;
    let files = data["files"].as_array().cloned().unwrap_or_default();
    if files.is_empty() {
        return Ok("No recent Drive files found.".into());
    }

    let mut lines = Vec::new();
    let mut snippets = Vec::new();
    for file in files.iter().take(limit) {
        let name = file["name"].as_str().unwrap_or("(unnamed)");
        let mime = file["mimeType"].as_str().unwrap_or("");
        let id = file["id"].as_str().unwrap_or("");
        lines.push(format!("- {name} ({mime})"));
        if mime == "application/vnd.google-apps.document" && !id.is_empty() {
            if let Ok(text) = export_doc_text(cfg, id).await {
                let clip: String = text.chars().take(400).collect();
                snippets.push(format!("Doc excerpt [{name}]: {clip}"));
            }
        }
    }

    let mut out = format!("Recent Drive files:\n{}", lines.join("\n"));
    if !snippets.is_empty() {
        out.push_str("\n\n");
        out.push_str(&snippets.join("\n\n"));
    }
    Ok(out)
}

pub async fn search_files(cfg: &AppConfig, query: &str, limit: usize) -> Result<String, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let q = format!("fullText contains '{}' and trashed=false", query.replace('\'', "\\'"));
    let url = format!(
        "https://www.googleapis.com/drive/v3/files?pageSize={}&fields=files(id,name,mimeType)&q={}",
        limit,
        urlencoding::encode(&q)
    );
    let res = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!(
            "Drive search error: {} {}",
            res.status(),
            res.text().await.unwrap_or_default()
        ));
    }
    let data: Value = res.json().await.map_err(|e| e.to_string())?;
    let files = data["files"].as_array().cloned().unwrap_or_default();
    if files.is_empty() {
        return Ok(format!("No Drive files matched '{query}'."));
    }
    let lines: Vec<String> = files
        .iter()
        .map(|f| {
            format!(
                "- {} ({})",
                f["name"].as_str().unwrap_or("?"),
                f["mimeType"].as_str().unwrap_or("?")
            )
        })
        .collect();
    Ok(format!("Drive search for '{query}':\n{}", lines.join("\n")))
}

async fn export_doc_text(cfg: &AppConfig, file_id: &str) -> Result<String, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{file_id}/export?mimeType=text/plain"
    );
    let res = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err("export failed".into());
    }
    res.text().await.map_err(|e| e.to_string())
}
