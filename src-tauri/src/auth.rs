use serde::{Deserialize, Serialize};
use std::fs;

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointSession {
    pub username: String,
    pub signed_in_at: String,
}

fn session_path(cfg: &AppConfig) -> std::path::PathBuf {
    cfg.data_dir.join("waypoint_session.json")
}

pub fn is_signed_in(cfg: &AppConfig) -> bool {
    load_session(cfg).is_some()
}

pub fn current_username(cfg: &AppConfig) -> Option<String> {
    load_session(cfg).map(|s| s.username)
}

pub fn load_session(cfg: &AppConfig) -> Option<WaypointSession> {
    let raw = fs::read_to_string(session_path(cfg)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Placeholder auth — accepts any non-empty username/password.
pub fn sign_in(cfg: &AppConfig, username: &str, password: &str) -> Result<WaypointSession, String> {
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err("Enter a Waypoint username or email.".into());
    }
    // Password is intentionally ignored for now — any value works, including empty.
    let _ = password;
    let session = WaypointSession {
        username,
        signed_in_at: chrono::Utc::now().to_rfc3339(),
    };
    let path = session_path(cfg);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let raw = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("Couldn’t save session: {e}"))?;
    Ok(session)
}

pub fn sign_out(cfg: &AppConfig) {
    let _ = fs::remove_file(session_path(cfg));
}
