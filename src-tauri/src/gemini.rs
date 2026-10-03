//! Shared Copilot / lock-in types. Gemini HTTP calls live on the Waypoint API
//! (`POST /v1/gemini/chat`); this crate must not ship an API key.

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

/// Live probe for Settings — confirms signed-in JWT can reach the API (Gemini stays server-side).
pub async fn probe_status_via_api(cfg: &AppConfig) -> (bool, String) {
    if crate::auth::load_tokens(cfg).is_none() {
        return (
            false,
            "Sign in with Google — cloud coach runs on the Waypoint API.".into(),
        );
    }
    match crate::api::authed_json::<serde_json::Value>(
        cfg,
        reqwest::Method::GET,
        "/v1/me",
        None,
    )
    .await
    {
        Ok(_) => (
            true,
            "Cloud coach ready via Waypoint API (Gemini key stays on the server).".into(),
        ),
        Err(e) => (false, format!("Couldn’t reach Waypoint API: {e}")),
    }
}

/// Copilot lock-in suggestion attached to an assistant `ChatMessage`.
/// Frontend: Accept → `start_lock_in(goals, duration_mins)`; Decline → ignore.
/// JSON shape (field names are a shared contract with the web UI — keep in sync):
/// `{ goals, duration_mins, reason, proposed_start, calendar_checked, calendar_clear, conflict_summary }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySessionSuggestion {
    pub goals: String,
    pub duration_mins: u64,
    pub reason: String,
    /// ISO8601 / RFC3339 when the session would start (usually "now")
    pub proposed_start: String,
    pub calendar_checked: bool,
    /// true when clear to start: Google connected + no overlap, OR Google not connected
    pub calendar_clear: bool,
    pub conflict_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Optional study-session CTA; omitted on older messages / when not suggested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study_suggestion: Option<StudySessionSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachVisionResult {
    pub on_task: bool,
    pub objects: Vec<String>,
    pub distraction: Option<String>,
    pub needs_help: bool,
    pub stress_cue: bool,
    pub coach_line: String,
    pub modality: Option<String>,
}

/// Local heuristic — avoids an extra model call at lock-in start.
pub fn infer_modality(goals: &str) -> String {
    let g = goals.to_lowercase();
    if g.contains("paper")
        || g.contains("notebook")
        || g.contains("handwrit")
        || g.contains("textbook")
    {
        "paper".into()
    } else if g.contains("mixed") || (g.contains("paper") && g.contains("laptop")) {
        "mixed".into()
    } else {
        "computer".into()
    }
}
