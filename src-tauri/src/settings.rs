use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    /// When true, lock-in coach overlays still show but spoken TTS is off.
    pub silent_mode: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self { silent_mode: false }
    }
}

fn settings_path(cfg: &AppConfig) -> PathBuf {
    cfg.data_dir.join("settings.json")
}

pub fn load(cfg: &AppConfig) -> UserSettings {
    let path = settings_path(cfg);
    let Ok(raw) = fs::read_to_string(path) else {
        return UserSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(cfg: &AppConfig, settings: &UserSettings) -> Result<(), String> {
    let path = settings_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
