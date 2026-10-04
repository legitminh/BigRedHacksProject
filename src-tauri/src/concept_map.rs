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

/// Final-review session summary image. The API owns the Grok Imagine prompt.
#[tauri::command]
pub async fn concept_map(
    state: State<'_, AppState>,
    markdown: String,
) -> Result<ConceptMapImage, String> {
    let cfg = state.config.lock().clone();
    if auth::load_tokens(&cfg).is_none() {
        return Err("Sign in to Waypoint first.".into());
    }
    let body = json!({ "markdown": markdown });
    api::authed_json(
        &cfg,
        reqwest::Method::POST,
        "/v1/concept-map",
        Some(&body),
    )
    .await
}
