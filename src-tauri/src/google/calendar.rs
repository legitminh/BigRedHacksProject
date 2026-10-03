use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::Value;

use crate::config::AppConfig;
use crate::google::oauth;

pub async fn upcoming_events_summary(cfg: &AppConfig, limit: usize) -> Result<String, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let now = Utc::now();
    let end = now + Duration::days(14);
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&maxResults={}&timeMin={}&timeMax={}",
        limit,
        urlencoding::encode(&now.to_rfc3339()),
        urlencoding::encode(&end.to_rfc3339())
    );

    let res = Client::new()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!(
            "Calendar API error: {} {}",
            res.status(),
            res.text().await.unwrap_or_default()
        ));
    }
    let data: Value = res.json().await.map_err(|e| e.to_string())?;
    let items = data["items"].as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        return Ok("No upcoming calendar events in the next 14 days.".into());
    }

    let mut lines = Vec::new();
    for item in items {
        let summary = item["summary"].as_str().unwrap_or("(untitled)");
        let start = item["start"]["dateTime"]
            .as_str()
            .or_else(|| item["start"]["date"].as_str())
            .unwrap_or("?");
        lines.push(format!("- {start}: {summary}"));
    }
    Ok(format!("Upcoming calendar:\n{}", lines.join("\n")))
}
