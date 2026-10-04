//! Persistent study log + rolling stats. Gemini is used only after a session ends
//! (and for Copilot chat) — never for live lock-in vision.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api;
use crate::auth;
use crate::config::AppConfig;
use crate::session::SessionSummary;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StudyStats {
    pub total_sessions: u32,
    pub total_flight_minutes: u32,
    pub total_on_task_minutes: f64,
    pub avg_on_task_ratio: f64,
    pub longest_flight_minutes: u32,
    pub stress_spikes_total: u32,
    /// Aggregated distraction labels → counts
    pub distraction_totals: Vec<(String, u32)>,
    pub last_goals: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsolidatedMemory {
    /// Short narrative Gemini (or local fallback) keeps for Copilot context.
    pub narrative: String,
    pub stats: StudyStats,
    pub updated_at: String,
}

fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join("study_session_log.jsonl")
}

fn consolidated_path(data_dir: &Path) -> PathBuf {
    data_dir.join("study_memory.json")
}

pub fn load_consolidated(data_dir: &Path) -> ConsolidatedMemory {
    let path = consolidated_path(data_dir);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => ConsolidatedMemory::default(),
    }
}

fn save_consolidated(data_dir: &Path, mem: &ConsolidatedMemory) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let path = consolidated_path(data_dir);
    let raw = serde_json::to_string_pretty(mem).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn append_session_log(data_dir: &Path, entry: &serde_json::Value) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let path = log_path(data_dir);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(entry).map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())
}

fn bump_distraction(totals: &mut Vec<(String, u32)>, label: &str, n: u32) {
    let key = label
        .split('×')
        .next()
        .unwrap_or(label)
        .trim()
        .to_string();
    if key.is_empty() {
        return;
    }
    if let Some((_, count)) = totals.iter_mut().find(|(k, _)| k == &key) {
        *count = count.saturating_add(n);
    } else {
        totals.push((key, n));
    }
}

fn update_stats(stats: &mut StudyStats, summary: &SessionSummary) {
    // duration_secs is already active (non-paused) flight time from summarize().
    let flight_mins = ((summary.duration_secs as f64) / 60.0).round() as u32;
    let on_task_mins = flight_mins as f64 * summary.on_task_ratio.clamp(0.0, 1.0);

    let prev_n = stats.total_sessions;
    stats.total_sessions = prev_n.saturating_add(1);
    stats.total_flight_minutes = stats.total_flight_minutes.saturating_add(flight_mins);
    stats.total_on_task_minutes += on_task_mins;
    stats.longest_flight_minutes = stats.longest_flight_minutes.max(flight_mins);
    stats.stress_spikes_total = stats
        .stress_spikes_total
        .saturating_add(summary.stress_spikes);
    stats.last_goals = summary.goals.clone();

    // Running average of on-task ratio
    let n = stats.total_sessions as f64;
    stats.avg_on_task_ratio =
        ((stats.avg_on_task_ratio * (n - 1.0)) + summary.on_task_ratio.clamp(0.0, 1.0)) / n;

    for d in &summary.top_distractions {
        // "instagram ×3" → add 3
        let mut parts = d.split('×');
        let label = parts.next().unwrap_or(d).trim();
        let count = parts
            .next()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(1);
        bump_distraction(&mut stats.distraction_totals, label, count);
    }
    stats
        .distraction_totals
        .sort_by(|a, b| b.1.cmp(&a.1));
    if stats.distraction_totals.len() > 12 {
        stats.distraction_totals.truncate(12);
    }
    stats.updated_at = chrono::Utc::now().to_rfc3339();
}

fn local_narrative_fallback(stats: &StudyStats, summary: &SessionSummary) -> String {
    let top = stats
        .distraction_totals
        .iter()
        .take(3)
        .map(|(k, v)| format!("{k}×{v}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Student has completed {} flights ({} min total, ~{:.0} min on-task, avg focus {:.0}%). \
         Latest goals: {}. Closing: {}. Top distractions: {}.",
        stats.total_sessions,
        stats.total_flight_minutes,
        stats.total_on_task_minutes,
        stats.avg_on_task_ratio * 100.0,
        summary.goals,
        summary.closing_note,
        if top.is_empty() {
            "none yet".into()
        } else {
            top
        }
    )
}

/// Append session to jsonl, update stats, optionally ask Gemini to refresh the consolidated narrative for Copilot.
pub async fn record_session_end(
    cfg: &AppConfig,
    summary: &SessionSummary,
) -> Result<ConsolidatedMemory, String> {
    let data_dir = &cfg.data_dir;
    let now = chrono::Utc::now().to_rfc3339();

    let entry = serde_json::json!({
        "at": now,
        "goals": summary.goals,
        "duration_secs": summary.duration_secs,
        "on_task_ratio": summary.on_task_ratio,
        "screen_checks": summary.screen_checks,
        "top_distractions": summary.top_distractions,
        "stress_spikes": summary.stress_spikes,
        "closing_note": summary.closing_note,
        "vitals_summary": summary.vitals_summary,
        "prompt_count": summary.prompts.len(),
    });
    append_session_log(data_dir, &entry)?;

    let mut mem = load_consolidated(data_dir);
    update_stats(&mut mem.stats, summary);

    let narrative = match consolidate_with_gemini(cfg, &mem, summary).await {
        Ok(text) if !text.trim().is_empty() => text.trim().to_string(),
        Ok(_) => local_narrative_fallback(&mem.stats, summary),
        Err(e) => {
            tracing::warn!("study memory Gemini consolidate: {e}");
            local_narrative_fallback(&mem.stats, summary)
        }
    };
    mem.narrative = narrative;
    mem.updated_at = now;
    save_consolidated(data_dir, &mem)?;
    Ok(mem)
}

async fn consolidate_with_gemini(
    cfg: &AppConfig,
    mem: &ConsolidatedMemory,
    summary: &SessionSummary,
) -> Result<String, String> {
    if auth::load_tokens(cfg).is_none() {
        return Err("Sign in required for cloud consolidation.".into());
    }
    let stats_json = serde_json::to_string_pretty(&mem.stats).unwrap_or_else(|_| "{}".into());
    let session_json = serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".into());
    let system = "You maintain a compact student study memory for Waypoint Copilot.\n\
         Given prior consolidated notes, rolling stats, and the latest lock-in session summary,\n\
         write an updated consolidation (max 180 words). Cover: recurring goals, focus trends,\n\
         distraction patterns, what helps, and one concrete next-session tip.\n\
         No markdown fences. No inventing courses or deadlines not present in the inputs.\n\
         This is for later chat context — not spoken aloud.";
    let user = format!(
        "PRIOR CONSOLIDATED NOTES:\n{}\n\nROLLING STATS:\n{}\n\nLATEST SESSION:\n{}",
        if mem.narrative.is_empty() {
            "(none yet)"
        } else {
            &mem.narrative
        },
        stats_json,
        session_json
    );
    #[derive(serde::Deserialize)]
    struct ChatReply {
        content: String,
    }
    let body = serde_json::json!({
        "message": user,
        "system": system,
        "history": [],
    });
    let out: ChatReply =
        api::authed_json(cfg, reqwest::Method::POST, "/v1/gemini/chat", Some(&body)).await?;
    Ok(out.content)
}

/// Wipe study logs, consolidated memory, sign-in, Google tokens, and settings.
/// Does not touch secrets.toml / baked API keys.
pub fn delete_all_user_data(cfg: &AppConfig) -> Result<Vec<String>, String> {
    let mut removed = Vec::new();
    let paths = [
        ("study session log", log_path(&cfg.data_dir)),
        ("study memory", consolidated_path(&cfg.data_dir)),
        ("Waypoint sign-in", cfg.data_dir.join("waypoint_session.json")),
        ("Waypoint API tokens", cfg.data_dir.join("waypoint_tokens.json")),
        ("Google tokens", cfg.google_token_path()),
        ("settings", cfg.data_dir.join("settings.json")),
    ];
    for (label, path) in paths {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Couldn’t delete {label}: {e}"))?;
            removed.push(label.to_string());
        }
    }
    Ok(removed)
}
