//! In-mission study companion — thin client helpers only.
//! Live voice goes Frontend → Waypoint API `/v1/companion/live` → Gemini Live.
//! Typed fallback uses `POST /v1/companion/chat`. Gemini keys never leave the API host.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::api;
use crate::auth;
use crate::gemini::ChatMessage;
use crate::AppState;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanionContext {
    pub goals: Option<String>,
    pub notes: Option<String>,
    pub modality: Option<String>,
    pub duration_mins: Option<f64>,
    pub remaining_mins: Option<f64>,
    pub next_step_secs: Option<f64>,
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionReply {
    pub role: String,
    pub content: String,
}

pub fn clear_history(state: &AppState) {
    state.companion_history.lock().clear();
}

fn merge_session_context(state: &AppState, mut ctx: CompanionContext) -> CompanionContext {
    if let Some(session) = state.session.lock().as_ref() {
        if ctx.goals.as_ref().map(|g| g.trim().is_empty()).unwrap_or(true) {
            ctx.goals = Some(session.goals.clone());
        }
        if ctx.modality.as_ref().map(|m| m.trim().is_empty()).unwrap_or(true) {
            ctx.modality = Some(session.modality.clone());
        }
        if ctx.duration_mins.is_none() {
            ctx.duration_mins = Some(session.duration_secs as f64 / 60.0);
        }
        if ctx.remaining_mins.is_none() {
            if let Ok(ends) = chrono::DateTime::parse_from_rfc3339(&session.ends_at) {
                let rem = (ends.with_timezone(&chrono::Utc) - chrono::Utc::now())
                    .num_seconds()
                    .max(0) as f64
                    / 60.0;
                ctx.remaining_mins = Some(rem);
            }
        }
        if ctx.paused.is_none() {
            ctx.paused = Some(session.paused);
        }
    }
    ctx
}

#[tauri::command]
pub async fn companion_send(
    state: State<'_, AppState>,
    message: String,
    context: Option<CompanionContext>,
) -> Result<CompanionReply, String> {
    let cfg = state.config.lock().clone();
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Enter a message.".into());
    }
    if auth::load_tokens(&cfg).is_none() {
        return Err("Sign in with Google to talk with your companion.".into());
    }
    if state.session.lock().is_none() {
        return Err("Start a study session to talk with your companion.".into());
    }

    let ctx = merge_session_context(&state, context.unwrap_or_default());
    let history = state.companion_history.lock().clone();
    let history_json: Vec<serde_json::Value> = history
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content,
            })
        })
        .collect();

    #[derive(Deserialize)]
    struct ApiReply {
        content: String,
    }

    let body = serde_json::json!({
        "message": message,
        "history": history_json,
        "context": {
            "goals": ctx.goals,
            "notes": ctx.notes,
            "modality": ctx.modality,
            "duration_mins": ctx.duration_mins,
            "remaining_mins": ctx.remaining_mins,
            "next_step_secs": ctx.next_step_secs,
            "paused": ctx.paused,
        },
    });

    let out: ApiReply =
        api::authed_json(&cfg, reqwest::Method::POST, "/v1/companion/chat", Some(&body)).await?;

    let user = ChatMessage {
        role: "user".into(),
        content: message,
        study_suggestion: None,
    };
    let assistant = ChatMessage {
        role: "assistant".into(),
        content: out.content.clone(),
        study_suggestion: None,
    };
    {
        let mut hist = state.companion_history.lock();
        hist.push(user);
        hist.push(assistant);
        if hist.len() > 40 {
            let drain = hist.len() - 40;
            hist.drain(0..drain);
        }
    }

    Ok(CompanionReply {
        role: "assistant".into(),
        content: out.content,
    })
}

#[tauri::command]
pub fn companion_clear(state: State<'_, AppState>) {
    clear_history(&state);
}

/// Connection info for the thin Live client — JWT + API WebSocket URL only.
/// Does not mint or return Gemini credentials.
#[derive(Debug, Clone, Serialize)]
pub struct CompanionLiveInfo {
    pub ws_url: String,
    pub access_token: String,
}

#[tauri::command]
pub fn companion_live_info(state: State<'_, AppState>) -> Result<CompanionLiveInfo, String> {
    let cfg = state.config.lock().clone();
    let tokens = auth::load_tokens(&cfg).ok_or_else(|| {
        "Sign in with Google to talk with your companion.".to_string()
    })?;
    if state.session.lock().is_none() {
        return Err("Start a study session to talk with your companion.".into());
    }
    let http = cfg.api_base().trim_end_matches('/');
    let ws = if let Some(rest) = http.strip_prefix("https://") {
        format!("wss://{rest}/v1/companion/live")
    } else if let Some(rest) = http.strip_prefix("http://") {
        format!("ws://{rest}/v1/companion/live")
    } else {
        format!("ws://{http}/v1/companion/live")
    };
    Ok(CompanionLiveInfo {
        ws_url: ws,
        access_token: tokens.access_token,
    })
}

/// Session teardown helper — stops in-flight local speak without changing TTS provider.
#[tauri::command]
pub fn voice_stop() {
    waypoint_voice::stop_speaking();
}
