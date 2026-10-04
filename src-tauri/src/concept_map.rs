use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::api;
use crate::auth;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptMapImage {
    pub content_type: String,
    pub image_base64: String,
}

/// Final-review mission brag-sheet image. The API owns the Grok Imagine prompt.
#[tauri::command]
pub async fn concept_map(
    state: State<'_, AppState>,
    markdown: String,
    locked_in_minutes: Option<u64>,
    mission: Option<String>,
    on_task_percent: Option<u32>,
) -> Result<ConceptMapImage, String> {
    let cfg = state.config.lock().clone();
    if auth::load_tokens(&cfg).is_none() {
        return Err("Sign in to Waypoint first.".into());
    }
    let mut body = json!({ "markdown": markdown });
    if let Some(mins) = locked_in_minutes.filter(|m| *m > 0) {
        body["locked_in_minutes"] = json!(mins);
    }
    if let Some(mission) = mission.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()) {
        body["mission"] = json!(mission);
    }
    if let Some(pct) = on_task_percent.filter(|p| *p > 0) {
        body["on_task_percent"] = json!(pct.min(100));
    }
    api::authed_json(
        &cfg,
        reqwest::Method::POST,
        "/v1/concept-map",
        Some(&body),
    )
    .await
}
