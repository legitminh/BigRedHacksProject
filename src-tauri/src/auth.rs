//! Waypoint account session — tokens from the backend Google sign-in flow.

use std::fs;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api;
use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user: PublicUser,
}

/// Back-compat shape used by older UI that expects username.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointSession {
    pub username: String,
    pub signed_in_at: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

fn tokens_path(cfg: &AppConfig) -> std::path::PathBuf {
    cfg.data_dir.join("waypoint_tokens.json")
}

fn legacy_session_path(cfg: &AppConfig) -> std::path::PathBuf {
    cfg.data_dir.join("waypoint_session.json")
}

pub fn load_tokens(cfg: &AppConfig) -> Option<AuthTokens> {
    let raw = fs::read_to_string(tokens_path(cfg)).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_tokens(cfg: &AppConfig, tokens: &AuthTokens) -> Result<(), String> {
    let path = tokens_path(cfg);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let raw = serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("Couldn’t save session: {e}"))?;
    // Keep a tiny legacy mirror for UI that still reads username.
    let legacy = WaypointSession {
        username: tokens
            .user
            .name
            .clone()
            .or_else(|| tokens.user.email.clone())
            .unwrap_or_else(|| "Waypoint user".into()),
        signed_in_at: chrono::Utc::now().to_rfc3339(),
        email: tokens.user.email.clone(),
        user_id: Some(tokens.user.id.clone()),
    };
    let _ = fs::write(
        legacy_session_path(cfg),
        serde_json::to_string_pretty(&legacy).unwrap_or_default(),
    );
    Ok(())
}

pub fn clear_tokens(cfg: &AppConfig) {
    let _ = fs::remove_file(tokens_path(cfg));
    let _ = fs::remove_file(legacy_session_path(cfg));
}

pub fn current_username(cfg: &AppConfig) -> Option<String> {
    if let Some(t) = load_tokens(cfg) {
        return Some(
            t.user
                .name
                .or(t.user.email)
                .unwrap_or_else(|| "Waypoint user".into()),
        );
    }
    load_session_legacy(cfg).map(|s| s.username)
}

pub fn load_session(cfg: &AppConfig) -> Option<WaypointSession> {
    if let Some(tokens) = load_tokens(cfg) {
        return Some(WaypointSession {
            username: tokens
                .user
                .name
                .clone()
                .or_else(|| tokens.user.email.clone())
                .unwrap_or_else(|| "Waypoint user".into()),
            signed_in_at: chrono::Utc::now().to_rfc3339(),
            email: tokens.user.email,
            user_id: Some(tokens.user.id),
        });
    }
    load_session_legacy(cfg)
}

#[derive(Debug, Deserialize)]
struct StartBody {
    authorization_url: String,
    poll_token: String,
}

#[derive(Debug, Deserialize)]
struct PollBody {
    status: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    user: Option<PublicUser>,
    error: Option<PollError>,
}

#[derive(Debug, Deserialize)]
struct PollError {
    code: Option<String>,
    message: Option<String>,
}

/// Only one browser OAuth window at a time (guards double-click / stacked UI handlers).
static GOOGLE_SIGN_IN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Open system browser for Google sign-in via the Waypoint API; poll until complete.
pub async fn sign_in_with_google(cfg: &AppConfig) -> Result<WaypointSession, String> {
    let Ok(_guard) = GOOGLE_SIGN_IN.try_lock() else {
        return Err("Sign-in already in progress — use the open Google window.".into());
    };

    let start: StartBody = api::post_json(cfg, "/v1/auth/google/start", &json!({}), None).await?;
    // Single explicit open — avoid any double-launcher quirks.
    std::process::Command::new("/usr/bin/open")
        .arg(&start.authorization_url)
        .status()
        .map_err(|e| format!("Couldn’t open browser: {e}"))?;

    for _ in 0..1200 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let poll: PollBody = api::get_json(
            cfg,
            &format!(
                "/v1/auth/google/poll?poll_token={}",
                urlencoding::encode(&start.poll_token)
            ),
            None,
        )
        .await?;
        match poll.status.as_str() {
            "pending" => continue,
            "error" => {
                let err = poll.error;
                let message = err
                    .as_ref()
                    .and_then(|e| e.message.clone())
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "Google sign-in failed.".into());
                let code = err.and_then(|e| e.code).unwrap_or_default();
                return Err(if code.is_empty() {
                    message
                } else {
                    format!("{message} ({code})")
                });
            }
            "complete" => {
                let access = poll
                    .access_token
                    .ok_or_else(|| "Missing access token.".to_string())?;
                let refresh = poll
                    .refresh_token
                    .ok_or_else(|| "Missing refresh token.".to_string())?;
                let user = poll.user.ok_or_else(|| "Missing user.".to_string())?;
                let expires_in = poll.expires_in.unwrap_or(900);
                let tokens = AuthTokens {
                    access_token: access,
                    refresh_token: refresh,
                    expires_at: chrono::Utc::now().timestamp() + expires_in,
                    user,
                };
                save_tokens(cfg, &tokens)?;
                // Local study_memory.json is a single device-wide cache. Drop whatever the
                // previous account/guest left behind so another user's longest flight (or a
                // planned-length leftover) can never be merged into this account and PUT back.
                clear_local_study_memory(cfg);
                // Pull cloud study memory onto this device after Google → user.id link.
                if let Ok(mem) = api::get_json::<serde_json::Value>(
                    cfg,
                    "/v1/study-memory",
                    Some(&tokens.access_token),
                )
                .await
                {
                    if let Some(blob) = mem.get("study_memory").filter(|v| !v.is_null()) {
                        let path = cfg.data_dir.join("study_memory.json");
                        if let Some(parent) = path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::write(
                            &path,
                            serde_json::to_string_pretty(blob).unwrap_or_default(),
                        );
                    }
                }
                return Ok(load_session(cfg).expect("session just saved"));
            }
            other => return Err(format!("Unexpected poll status: {other}")),
        }
    }
    Err("Google sign-in timed out. Try again.".into())
}

/// Remove the device-wide study memory cache (cloud copy is the source of truth when signed in).
pub fn clear_local_study_memory(cfg: &AppConfig) {
    let _ = fs::remove_file(cfg.data_dir.join("study_memory.json"));
}

pub async fn sign_out_remote(cfg: &AppConfig) -> Result<(), String> {
    if let Some(tokens) = load_tokens(cfg) {
        let body = json!({ "refresh_token": tokens.refresh_token });
        let _ = api::send_empty(
            cfg,
            reqwest::Method::POST,
            "/v1/auth/sign-out",
            Some(&tokens.access_token),
            Some(&body),
        )
        .await;
    }
    clear_tokens(cfg);
    clear_local_study_memory(cfg);
    Ok(())
}

/// Guest local-only session (no backend sync).
pub fn sign_in_guest(cfg: &AppConfig) -> Result<WaypointSession, String> {
    clear_tokens(cfg);
    clear_local_study_memory(cfg);
    let session = WaypointSession {
        username: "Guest".into(),
        signed_in_at: chrono::Utc::now().to_rfc3339(),
        email: None,
        user_id: None,
    };
    let path = legacy_session_path(cfg);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Couldn’t save session: {e}"))?;
    Ok(session)
}

fn load_session_legacy(cfg: &AppConfig) -> Option<WaypointSession> {
    let raw = fs::read_to_string(legacy_session_path(cfg)).ok()?;
    serde_json::from_str(&raw).ok()
}
