pub mod calendar;
pub mod drive;
pub mod oauth;

use serde::{Deserialize, Serialize};

/// IANA zone name for this machine, so the backend can label "today"/"tomorrow"
/// the way the student sees them. Falls back to a fixed-offset `Etc/GMT±N` zone.
pub fn local_timezone() -> String {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim().trim_start_matches(':');
        if tz.contains('/') {
            return tz.to_string();
        }
    }
    if let Ok(path) = std::fs::read_link("/etc/localtime") {
        let text = path.to_string_lossy();
        if let Some(index) = text.find("zoneinfo/") {
            let name = &text[index + "zoneinfo/".len()..];
            if name.contains('/') {
                return name.to_string();
            }
        }
    }
    // Etc/GMT zones invert the sign: UTC-4 is "Etc/GMT+4".
    use chrono::{Local, Offset};
    let seconds = Local::now().offset().fix().local_minus_utc();
    if seconds % 3600 == 0 {
        let hours = seconds / 3600;
        if hours == 0 {
            return "UTC".into();
        }
        return format!("Etc/GMT{}{}", if hours > 0 { '-' } else { '+' }, hours.abs());
    }
    "UTC".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleContext {
    pub connected: bool,
    pub calendar_summary: String,
    /// Recently touched files, with a few short excerpts.
    pub drive_summary: String,
    /// Compact name/type/folder listing of (nearly) the whole Drive. No contents.
    #[serde(default)]
    pub drive_inventory: String,
    /// Once-daily overview from GET /v1/school-digest (Flash build on API only); empty if unavailable.
    #[serde(default)]
    pub school_digest: String,
    /// Local date label for the cached digest (YYYY-MM-DD), when known.
    #[serde(default)]
    pub school_digest_date: String,
    /// Whether POST /v1/school-digest/refresh is allowed (one manual rebuild / 24h).
    #[serde(default)]
    pub manual_refresh_available: bool,
    /// ISO timestamp of the last manual refresh, when any.
    #[serde(default)]
    pub last_manual_refresh_at: String,
    /// ISO timestamp when the next manual refresh unlocks (empty when available).
    #[serde(default)]
    pub next_manual_refresh_at: String,
}
