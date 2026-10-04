use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::presage::VitalsSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatusKind {
    OnTask,
    Distracted,
    Stressed,
    NeedsHelp,
    Idle,
    Watching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachPrompt {
    pub id: String,
    pub at: String,
    pub text: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInSession {
    pub id: String,
    pub goals: String,
    pub duration_secs: u64,
    pub started_at: String,
    pub ends_at: String,
    pub modality: String,
    pub status: SessionStatusKind,
    pub active: bool,
    pub paused: bool,
    /// Cumulative seconds spent paused (breaks). Open pause is folded in at end.
    pub paused_accum_secs: u64,
    pub prompts: Vec<CoachPrompt>,
    pub vitals: VitalsSnapshot,
    pub distraction_counts: Vec<(String, u32)>,
    pub on_task_ticks: u32,
    pub total_ticks: u32,
    pub stress_spikes: u32,
    pub camera_ready: bool,
    pub presage_ready: bool,
    /// User consented to screen-based watching (app/title/URL, OCR, local VLM) for this mission.
    #[serde(default)]
    pub screen_enabled: bool,
    /// User consented to camera accountability for this mission.
    #[serde(default)]
    pub camera_enabled: bool,
    pub watching_note: String,
    /// Local screen summaries from this lock-in (app/title/URL, OCR, judge). Text only, newest last.
    #[serde(default)]
    pub screen_log: Vec<String>,
}

pub const SCREEN_LOG_CAP: usize = 40;
pub const SCREEN_LOG_MAX_CHARS: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub goals: String,
    pub duration_secs: u64,
    pub modality: String,
    pub on_task_ratio: f64,
    pub screen_checks: u32,
    pub top_distractions: Vec<String>,
    pub stress_spikes: u32,
    pub prompts: Vec<CoachPrompt>,
    pub closing_note: String,
    pub vitals_summary: String,
    /// Lock-in id, for the session note. Empty on older payloads.
    #[serde(default)]
    pub session_id: String,
    /// When the lock-in started (RFC3339). Empty on older payloads.
    #[serde(default)]
    pub started_at: String,
    /// Readable session note, once written. Omitted until generation finishes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_note: Option<String>,
    /// True while a session note is still being written.
    #[serde(default)]
    pub notes_pending: bool,
}

impl LockInSession {
    pub fn start(
        goals: String,
        duration_mins: u64,
        modality: String,
        screen_enabled: bool,
        camera_enabled: bool,
        camera_ready: bool,
        presage_ready: bool,
    ) -> Self {
        let now = chrono::Utc::now();
        let duration_secs = duration_mins.saturating_mul(60).max(60);
        let camera_ready = camera_enabled && camera_ready;
        let watching_note = if !screen_enabled {
            "Screen sharing off · timer and check-ins only".into()
        } else if !camera_enabled {
            "Watching your screen · camera accountability off".into()
        } else if camera_ready && presage_ready {
            "Watching your screen · camera accountability in background".into()
        } else {
            "Watching your screen · camera accountability unavailable".into()
        };
        Self {
            id: Uuid::new_v4().to_string(),
            goals,
            duration_secs,
            started_at: now.to_rfc3339(),
            ends_at: (now + chrono::Duration::seconds(duration_secs as i64)).to_rfc3339(),
            modality,
            status: SessionStatusKind::Watching,
            active: true,
            paused: false,
            paused_accum_secs: 0,
            prompts: Vec::new(),
            vitals: VitalsSnapshot::default(),
            distraction_counts: Vec::new(),
            on_task_ticks: 0,
            total_ticks: 0,
            stress_spikes: 0,
            camera_ready,
            presage_ready,
            screen_enabled,
            camera_enabled,
            watching_note,
            screen_log: Vec::new(),
        }
    }

    /// Append one local screen summary. Skips empties and exact repeats, caps the log.
    pub fn push_screen_log(&mut self, line: &str) {
        let clipped = clip_screen_line(line, SCREEN_LOG_MAX_CHARS);
        if clipped.is_empty() {
            return;
        }
        if self.screen_log.iter().any(|prev| prev == &clipped) {
            return;
        }
        self.screen_log.push(clipped);
        if self.screen_log.len() > SCREEN_LOG_CAP {
            let extra = self.screen_log.len() - SCREEN_LOG_CAP;
            self.screen_log.drain(0..extra);
        }
    }

    pub fn remaining_secs(&self) -> i64 {
        if let Ok(ends) = chrono::DateTime::parse_from_rfc3339(&self.ends_at) {
            (ends.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_seconds()
        } else {
            0
        }
    }

    /// Remaining time as the student experiences it: frozen while paused. `ends_at` is only
    /// pushed out when the pause is folded in (resume/end), so a paused session whose
    /// wall-clock `ends_at` has passed must not be treated as expired.
    pub fn remaining_secs_with_pause(
        &self,
        pause_started: Option<chrono::DateTime<chrono::Utc>>,
    ) -> i64 {
        let Ok(ends) = chrono::DateTime::parse_from_rfc3339(&self.ends_at) else {
            return 0;
        };
        let ends = ends.with_timezone(&chrono::Utc);
        let reference = if self.paused {
            pause_started.unwrap_or_else(chrono::Utc::now)
        } else {
            chrono::Utc::now()
        };
        (ends - reference).num_seconds()
    }

    /// True only for a natural timer expiry — never while on a break.
    pub fn is_expired(&self, pause_started: Option<chrono::DateTime<chrono::Utc>>) -> bool {
        self.active && self.remaining_secs_with_pause(pause_started) <= 0
    }

    /// Fold an open pause into `paused_accum_secs` and extend `ends_at` so remaining
    /// time does not keep burning while the student is on a break.
    pub fn finalize_open_pause(&mut self, pause_started: Option<chrono::DateTime<chrono::Utc>>) {
        let Some(started) = pause_started else {
            return;
        };
        let paused_secs = (chrono::Utc::now() - started).num_seconds().max(0) as u64;
        if paused_secs == 0 {
            return;
        }
        self.paused_accum_secs = self.paused_accum_secs.saturating_add(paused_secs);
        if let Ok(ends) = chrono::DateTime::parse_from_rfc3339(&self.ends_at) {
            self.ends_at = (ends.with_timezone(&chrono::Utc)
                + chrono::Duration::seconds(paused_secs as i64))
            .to_rfc3339();
        }
    }

    /// Active flight time: wall clock since start minus breaks, capped at planned duration.
    pub fn active_elapsed_secs(&self) -> u64 {
        let Ok(started) = chrono::DateTime::parse_from_rfc3339(&self.started_at) else {
            return 0;
        };
        let wall = (chrono::Utc::now() - started.with_timezone(&chrono::Utc))
            .num_seconds()
            .max(0) as u64;
        wall.saturating_sub(self.paused_accum_secs)
            .min(self.duration_secs)
    }

    pub fn bump_distraction(&mut self, label: &str) {
        if let Some((_, count)) = self
            .distraction_counts
            .iter_mut()
            .find(|(k, _)| k == label)
        {
            *count += 1;
        } else {
            self.distraction_counts.push((label.to_string(), 1));
        }
    }

    pub fn summarize(&self) -> SessionSummary {
        // Never treat "no screen checks" as a perfect score.
        let ratio = if self.total_ticks == 0 {
            0.0
        } else {
            self.on_task_ticks as f64 / self.total_ticks as f64
        };
        let mut distractions = self.distraction_counts.clone();
        distractions.sort_by(|a, b| b.1.cmp(&a.1));
        let top: Vec<String> = distractions
            .into_iter()
            .take(3)
            .map(|(k, v)| format!("{k} ×{v}"))
            .collect();

        let closing_note = if self.total_ticks == 0 {
            "Screen checks didn’t land this session (quota limits or capture issues), so focus couldn’t be verified — that wasn’t a perfect lock-in.".into()
        } else if !top.is_empty() && ratio < 0.6 {
            format!(
                "You drifted to {} — next block, keep only the goal app visible.",
                top[0].split('×').next().unwrap_or("distractions").trim()
            )
        } else if ratio > 0.8 && self.stress_spikes <= 1 {
            "Strong lock-in. You stayed with the work and kept stress mostly steady.".into()
        } else if self.stress_spikes > 2 {
            "You pushed through stress spikes — next time try shorter blocks or a quick reset breath.".into()
        } else if ratio < 0.5 {
            "Focus drifted often. Tighten the next goal to one concrete task on screen.".into()
        } else {
            "Solid effort. Review what pulled you off-screen and set a clearer next waypoint.".into()
        };

        SessionSummary {
            goals: self.goals.clone(),
            // Earned flight time — not the planned block length.
            duration_secs: self.active_elapsed_secs(),
            modality: self.modality.clone(),
            on_task_ratio: ratio,
            screen_checks: self.total_ticks,
            top_distractions: top,
            stress_spikes: self.stress_spikes,
            prompts: self.prompts.clone(),
            closing_note,
            vitals_summary: if !self.camera_enabled {
                "Camera accountability was off for this mission.".into()
            } else if self.vitals.raw_summary.is_empty() {
                "Camera accountability was on; no presence reading this mission.".into()
            } else {
                self.vitals.raw_summary.clone()
            },
            session_id: self.id.clone(),
            started_at: self.started_at.clone(),
            session_note: None,
            notes_pending: false,
        }
    }
}

fn clip_screen_line(s: &str, max_chars: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        flat
    } else {
        let cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_active_elapsed_not_planned_duration() {
        let mut session = LockInSession::start(
            "code".into(),
            25,
            "coding".into(),
            true,
            false,
            false,
            false,
        );
        // Simulate ~90s of work with 10 minutes of break already accounted.
        session.paused_accum_secs = 600;
        let started = chrono::Utc::now() - chrono::Duration::seconds(690);
        session.started_at = started.to_rfc3339();
        let summary = session.summarize();
        assert!(
            summary.duration_secs <= 120,
            "expected ~90s earned, got {}",
            summary.duration_secs
        );
        assert!(summary.duration_secs >= 60);
        assert_ne!(summary.duration_secs, session.duration_secs);
    }

    fn test_session() -> LockInSession {
        LockInSession::start("code".into(), 25, "coding".into(), true, false, false, false)
    }

    #[test]
    fn paused_session_past_ends_at_is_not_expired() {
        let mut session = test_session();
        // Wall-clock deadline passed 5 min ago, but the break began 10 min before it.
        let now = chrono::Utc::now();
        session.ends_at = (now - chrono::Duration::minutes(5)).to_rfc3339();
        session.paused = true;
        let pause_started = Some(now - chrono::Duration::minutes(15));
        assert!(!session.is_expired(pause_started));
        assert!(session.remaining_secs_with_pause(pause_started) > 9 * 60);
        // Same session not paused really is expired.
        session.paused = false;
        assert!(session.is_expired(None));
    }

    #[test]
    fn resume_extends_ends_at_by_pause_length() {
        let mut session = test_session();
        let now = chrono::Utc::now();
        session.ends_at = (now + chrono::Duration::minutes(2)).to_rfc3339();
        session.paused = true;
        let started = now - chrono::Duration::minutes(10);
        session.finalize_open_pause(Some(started));
        session.paused = false;
        assert!(session.paused_accum_secs >= 600);
        let remaining = session.remaining_secs();
        assert!(
            (remaining - 12 * 60).abs() <= 2,
            "expected ~12m remaining after resume, got {remaining}s"
        );
        assert!(!session.is_expired(None));
    }

    #[test]
    fn screen_log_truncates_dedups_and_caps() {
        let mut session = test_session();
        session.push_screen_log("   ");
        session.push_screen_log("  Cursor ·  repo  ");
        session.push_screen_log("Cursor · repo");
        assert_eq!(session.screen_log, vec!["Cursor · repo".to_string()]);
        session.push_screen_log(&"a".repeat(300));
        assert!(session.screen_log[1].chars().count() <= SCREEN_LOG_MAX_CHARS);
        assert!(session.screen_log[1].ends_with('…'));
        for i in 0..50 {
            session.push_screen_log(&format!("line {i}"));
        }
        assert_eq!(session.screen_log.len(), SCREEN_LOG_CAP);
        assert_eq!(
            session.screen_log.last().map(String::as_str),
            Some("line 49")
        );
    }

    #[test]
    fn consent_flags_gate_camera_ready() {
        let s = LockInSession::start("x".into(), 5, "m".into(), false, false, true, true);
        assert!(!s.camera_ready && !s.screen_enabled);
        assert!(!s.camera_enabled);
        assert!(s.watching_note.contains("camera accountability off") || !s.screen_enabled);
        let s = LockInSession::start("x".into(), 5, "m".into(), true, false, true, true);
        assert!(!s.camera_ready && s.screen_enabled && !s.camera_enabled);
        assert!(s.watching_note.contains("camera accountability off"));
        let s = LockInSession::start("x".into(), 5, "m".into(), true, true, true, true);
        assert!(s.camera_ready && s.camera_enabled && s.screen_enabled);
        assert!(s.watching_note.contains("camera accountability in background"));
    }
}
