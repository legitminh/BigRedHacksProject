//! In-mission study companion — thin client helpers only.
//! Live voice goes Frontend → Waypoint API `/v1/companion/live` → Gemini Live.
//! Typed fallback uses `POST /v1/companion/chat`. Gemini keys never leave the API host.
//! Screencaps for Live are captured locally and streamed to the API (never to Google from the app).

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::api;
use crate::auth;
use crate::capture;
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

const SCREEN_SHARING_OFF: &str = "Screen sharing is off, so I can't look at your screen.";

/// Live WebSocket subprotocol marker the API selects (never echoes the JWT).
const LIVE_SUBPROTOCOL: &str = "waypoint.live.v1";
const LIVE_BEARER_PREFIX: &str = "bearer.";

/// Connection info for the thin Live client — API WebSocket URL + auth subprotocols.
/// The JWT is delivered as `Sec-WebSocket-Protocol: bearer.<jwt>` (never in the URL query).
/// Does not mint or return Gemini credentials.
#[derive(Debug, Clone, Serialize)]
pub struct CompanionLiveInfo {
    /// Plain `/v1/companion/live` URL — no query string, no credentials.
    pub ws_url: String,
    /// Pass as the `protocols` argument to `new WebSocket(ws_url, protocols)`.
    pub protocols: Vec<String>,
}

/// RFC 6455 subprotocol names must be HTTP tokens (no spaces, separators, or control chars).
fn is_valid_subprotocol_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^'
                        | b'_' | b'`' | b'|' | b'~'
                )
        })
}

#[tauri::command]
pub fn companion_live_info(state: State<'_, AppState>) -> Result<CompanionLiveInfo, String> {
    let cfg = state.config.lock().clone();
    let tokens = auth::load_tokens(&cfg).ok_or_else(|| {
        "Sign in with Google to talk with your companion.".to_string()
    })?;
    // Live is allowed from Copilot or an active lock-in — JWT only; Gemini key stays on the API.
    let http = cfg.api_base().trim_end_matches('/');
    let ws = if let Some(rest) = http.strip_prefix("https://") {
        format!("wss://{rest}/v1/companion/live")
    } else if let Some(rest) = http.strip_prefix("http://") {
        format!("ws://{rest}/v1/companion/live")
    } else {
        format!("ws://{http}/v1/companion/live")
    };
    let bearer = format!("{LIVE_BEARER_PREFIX}{}", tokens.access_token.trim());
    if !is_valid_subprotocol_token(&bearer) {
        return Err("Your session token can't be used for Live voice. Sign in again.".into());
    }
    Ok(CompanionLiveInfo {
        ws_url: ws,
        protocols: vec![LIVE_SUBPROTOCOL.to_string(), bearer],
    })
}

/// Session teardown helper — stops in-flight local speak without changing TTS provider.
#[tauri::command]
pub fn voice_stop() {
    waypoint_voice::stop_speaking();
}

/// Capture a desktop JPEG for the Live companion. Base64 stays on the device until
/// the thin client posts it to Waypoint API `/v1/companion/live` (API → Gemini).
///
/// Consent gate: refuses (no capture at all) unless the UI reports screen sharing is on
/// (`screen_consent`, from the `wp-setting-screen-sharing` pref) AND — when a lock-in
/// session is running — that session was launched with `screen_enabled`.
#[tauri::command]
pub async fn companion_grab_screencap(
    state: State<'_, AppState>,
    screen_consent: Option<bool>,
) -> Result<String, String> {
    if !screen_consent.unwrap_or(false) {
        return Err(SCREEN_SHARING_OFF.into());
    }
    let session_allows = state
        .session
        .lock()
        .as_ref()
        .map(|s| s.screen_enabled)
        .unwrap_or(true);
    if !session_allows {
        return Err(SCREEN_SHARING_OFF.into());
    }
    let jpeg = tokio::task::spawn_blocking(capture::screen::grab_desktop_jpeg)
        .await
        .map_err(|e| format!("Screen capture task failed: {e}"))?
        .map_err(|e| e)?;
    if jpeg.len() > 2_500_000 {
        return Err("Screenshot too large to send.".into());
    }
    Ok(B64.encode(jpeg))
}
