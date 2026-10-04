pub mod calendar;
pub mod drive;
pub mod oauth;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleContext {
    pub connected: bool,
    pub calendar_summary: String,
    pub drive_summary: String,
    /// Calendar returned a summary. False when the tool still needs Settings → Tools.
    #[serde(default)]
    pub calendar_connected: bool,
    /// Drive returned a summary. False when the tool still needs Settings → Tools.
    #[serde(default)]
    pub drive_connected: bool,
}
