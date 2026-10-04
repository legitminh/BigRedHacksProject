use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

fn default_notes_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    /// When true, lock-in coach overlays still show but spoken TTS is off.
    pub silent_mode: bool,
    /// When true, Waypoint writes a readable note when the lock-in ends.
    /// Function default: a missing key in old settings.json must stay on (`bool` default is false).
    #[serde(default = "default_notes_enabled")]
    pub notes_enabled: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        // Figma 04: Copilot audio starts off (spoken TTS opt-in).
        // Session notes start on, including when settings.json has no key.
        Self {
            silent_mode: true,
            notes_enabled: true,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_default_on_when_key_missing() {
        assert!(UserSettings::default().notes_enabled);
        let parsed: UserSettings =
            serde_json::from_str(r#"{"silent_mode":true}"#).expect("parse");
        assert!(
            parsed.notes_enabled,
            "old settings.json without notes_enabled must stay on"
        );
        let off: UserSettings =
            serde_json::from_str(r#"{"silent_mode":true,"notes_enabled":false}"#).expect("parse");
        assert!(!off.notes_enabled);
    }
}
