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
            "Watching your screen · webcam ready for Presage stress checks".into()
        } else if camera_ready {
            "Watching your screen · webcam on (Presage key not set — stress from vision only)".into()
        } else if presage_ready {
            "Watching your screen · allow Camera to enable Presage stress checks".into()
        } else {
            "Watching your screen · stress checks need Camera + Presage API key".into()
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
        let ratio = if self.total_ticks == 0 {
            1.0
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

        let closing_note = if ratio > 0.8 && self.stress_spikes <= 1 {
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
            duration_secs: self.duration_secs,
            modality: self.modality.clone(),
            on_task_ratio: ratio,
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
