pub mod calendar;
pub mod drive;
pub mod oauth;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleContext {
    pub connected: bool,
    pub calendar_summary: String,
    pub drive_summary: String,
}
