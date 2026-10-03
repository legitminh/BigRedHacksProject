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
}

impl LockInSession {
    pub fn start(goals: String, duration_mins: u64, modality: String) -> Self {
        let now = chrono::Utc::now();
        let duration_secs = duration_mins.saturating_mul(60).max(60);
        Self {
            id: Uuid::new_v4().to_string(),
            goals,
            duration_secs,
            started_at: now.to_rfc3339(),
            ends_at: (now + chrono::Duration::seconds(duration_secs as i64)).to_rfc3339(),
            modality,
            status: SessionStatusKind::Idle,
            active: true,
            prompts: Vec::new(),
            vitals: VitalsSnapshot::default(),
            distraction_counts: Vec::new(),
            on_task_ticks: 0,
            total_ticks: 0,
            stress_spikes: 0,
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

        let closing_note = if ratio > 0.8 {
            "Strong lock-in. You stayed with the map most of the session.".into()
        } else if self.stress_spikes > 2 {
            "You pushed through some stress spikes — next time, shorter blocks may help.".into()
        } else {
            "Decent effort. Review the distractions and tighten the next waypoint.".into()
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
        }
    }
}
