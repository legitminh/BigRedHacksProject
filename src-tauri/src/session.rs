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
    pub watching_note: String,
}

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
}

impl LockInSession {
    pub fn start(
        goals: String,
        duration_mins: u64,
        modality: String,
        camera_ready: bool,
        presage_ready: bool,
    ) -> Self {
        let now = chrono::Utc::now();
        let duration_secs = duration_mins.saturating_mul(60).max(60);
        let watching_note = if camera_ready && presage_ready {
            "Watching full screen · wellness later in background".into()
        } else {
            "Watching full screen".into()
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
            watching_note,
        }
    }

    pub fn remaining_secs(&self) -> i64 {
        if let Ok(ends) = chrono::DateTime::parse_from_rfc3339(&self.ends_at) {
            (ends.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_seconds()
        } else {
            0
        }
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
            vitals_summary: if self.vitals.raw_summary.is_empty() {
                "No wellness reading this session.".into()
            } else {
                self.vitals.raw_summary.clone()
            },
        }
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
}
