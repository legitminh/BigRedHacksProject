use chrono::{DateTime, Duration, Utc};
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

/// Returns `Some(human summary)` if a timed event overlaps `[start, start+duration)`,
/// `None` if the window is clear. All-day events are skipped (do not block).
/// Errors when Google is not connected or the Calendar API fails.
pub async fn proposed_window_conflicts(
    cfg: &AppConfig,
    start: DateTime<Utc>,
    duration_mins: u64,
) -> Result<Option<String>, String> {
    let token = oauth::ensure_access_token(cfg).await?;
    let window_end = start + Duration::minutes(duration_mins as i64);
    // Fetch a bit past the window so events that start during it are included.
    let fetch_end = window_end + Duration::hours(1);
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&maxResults=25&timeMin={}&timeMax={}",
        urlencoding::encode(&start.to_rfc3339()),
        urlencoding::encode(&fetch_end.to_rfc3339())
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

    let mut conflicts = Vec::new();
    for item in items {
        // All-day events use `start.date` without `dateTime` — skip them.
        let Some(ev_start_s) = item["start"]["dateTime"].as_str() else {
            continue;
        };
        let Some(ev_end_s) = item["end"]["dateTime"].as_str() else {
            continue;
        };
        let Ok(ev_start) = DateTime::parse_from_rfc3339(ev_start_s) else {
            continue;
        };
        let Ok(ev_end) = DateTime::parse_from_rfc3339(ev_end_s) else {
            continue;
        };
        let ev_start = ev_start.with_timezone(&Utc);
        let ev_end = ev_end.with_timezone(&Utc);

        // Overlap: event_start < window_end && event_end > window_start
        if ev_start < window_end && ev_end > start {
            let summary = item["summary"].as_str().unwrap_or("(untitled)");
            conflicts.push(format!("{summary} ({ev_start_s} – {ev_end_s})"));
        }
    }

    if conflicts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "Conflicts with: {}",
            conflicts.join("; ")
        )))
    }
}
