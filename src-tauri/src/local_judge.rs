//! On-device on-task classifier via a small quantized model (Ollama / Llama).
//! Lock-in never calls Gemini — local provider only.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::gemini::CoachVisionResult;

fn with_coach_auth(req: reqwest::RequestBuilder, cfg: &AppConfig) -> reqwest::RequestBuilder {
    if let Some((name, value)) = cfg.coach_auth_header() {
        req.header(name, value)
    } else {
        req
    }
}

const MIN_CONFIDENCE: f32 = 0.55;
const PROBE_TTL_SECS: u64 = 45;

static PROBE_CACHE: Mutex<Option<(Instant, bool, String)>> = Mutex::new(None);
/// Recent spoken/overlay lines — avoid boring the user with repeats.
static RECENT_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
const RECENT_LINE_CAP: usize = 8;

#[derive(Debug, Clone)]
pub struct LocalJudgeConfig {
    pub base_url: String,
    pub model: String,
}

impl LocalJudgeConfig {
    pub fn from_app(cfg: &AppConfig) -> Self {
        Self {
            base_url: cfg.local_llm_base.clone(),
            model: cfg.local_llm_model.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JudgeJson {
    on_task: bool,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    distraction: Option<String>,
    #[serde(default)]
    coach_line: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LocalJudgment {
    pub result: CoachVisionResult,
    pub confidence: f32,
    pub model: String,
}

pub fn min_confidence() -> f32 {
    MIN_CONFIDENCE
}

/// Cached probe — is Ollama up with a usable model?
pub async fn is_available(cfg: &AppConfig) -> bool {
    if !cfg.local_llm_enabled {
        return false;
    }
    {
        let guard = PROBE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, ok, _)) = guard.as_ref() {
            if at.elapsed() < Duration::from_secs(PROBE_TTL_SECS) {
                return *ok;
            }
        }
    }
    let local = LocalJudgeConfig::from_app(cfg);
    let (ok, detail) = probe(cfg, &local).await;
    if let Ok(mut guard) = PROBE_CACHE.lock() {
        *guard = Some((Instant::now(), ok, detail));
    }
    ok
}

pub async fn status_line(cfg: &AppConfig) -> String {
    if !cfg.local_llm_enabled {
        return "Coach off".into();
    }
    let local = LocalJudgeConfig::from_app(cfg);
    let (ok, detail) = probe(cfg, &local).await;
    if let Ok(mut guard) = PROBE_CACHE.lock() {
        *guard = Some((Instant::now(), ok, detail.clone()));
    }
    if ok {
        format!("Coach ready · {} · via API", local.model)
    } else {
        format!("Coach unavailable · {detail}")
    }
}

async fn probe(app: &AppConfig, cfg: &LocalJudgeConfig) -> (bool, String) {
    let client = match Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (false, e.to_string()),
    };
    let url = format!("{}/api/tags", cfg.base_url.trim_end_matches('/'));
    let res = match with_coach_auth(client.get(&url), app).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                false,
                "start Waypoint API".into(),
            );
        }
    };
    if !res.status().is_success() {
        let code = res.status().as_u16();
        let msg = if code == 401 || code == 403 {
            "check coach API token".into()
        } else {
            format!("API returned {code}")
        };
        return (false, msg);
    }
    let body: Value = match res.json().await {
        Ok(v) => v,
        Err(e) => return (false, e.to_string()),
    };
    let models = body["models"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if models.is_empty() {
        return (
            false,
            format!("no coach models yet — pull {}", cfg.model),
        );
    }
    let want = cfg.model.to_lowercase();
    let want_base = want.split(':').next().unwrap_or(&want);
    let found = models.iter().any(|m| {
        let name = m["name"].as_str().unwrap_or("").to_lowercase();
        name == want
            || name.starts_with(&format!("{want}:"))
            || name.starts_with(&format!("{want_base}:"))
            || name == want_base
    });
    if found {
        (true, cfg.model.clone())
    } else {
        let have: Vec<&str> = models
            .iter()
            .filter_map(|m| m["name"].as_str())
            .take(4)
            .collect();
        (
            false,
            format!(
                "need model {} (have: {})",
                cfg.model,
                have.join(", ")
            ),
        )
    }
}

/// Classify focus from text only (goals + app/title/url/excerpt). No screenshot.
pub async fn judge_on_task(
    cfg: &AppConfig,
    goals: &str,
    kind: &str,
    app: &str,
    title: &str,
    url: &str,
    page_text: &str,
) -> Result<LocalJudgment, String> {
    if !cfg.local_llm_enabled {
        return Err("local LLM disabled".into());
    }
    let local = LocalJudgeConfig::from_app(cfg);
    let excerpt: String = page_text.chars().take(1200).collect();
    let goal_hint = goals_snippet(goals, 48);
    let prompt = format!(
        r#"You are a strict study lock-in classifier. Decide if the student is ON TASK for their goals.
Reply with ONLY compact JSON, no markdown:
{{"on_task":true|false,"confidence":0.0-1.0,"distraction":null|"youtube","coach_line":"Close Discord and get back to {goal_hint}."}}

Rules:
- Decide from title, url, and page_text — NOT from the kind label alone, and NOT from tab-group names like "School".
- on_task=true only if this clearly advances the goals (coursework, lecture, tutorial matching goals).
- YouTube/video are CONTEXTUAL: lecture/tutorial matching goals → on_task=true; music/rap/artist tracks/"no music"/gaming/vlogs/memes/entertainment → false, distraction="youtube".
- Discord is CONTEXTUAL: study/homework help matching goals → may be true; meme/gaming spam → false, distraction="discord".
- Instagram, shopping, email, texting-class → ALWAYS on_task=false (never study).
- If unsure, confidence < 0.5 and lean on_task=false for entertainment content.
- coach_line MUST name the distraction AND reuse words from the goals field only — never invent other courses, assignments, or discussion posts that are not in goals.
- Never output meta text like "one short sentence" or "short nudge".

goals: {goals}
kind: {kind}
app: {app}
title: {title}
url: {url}
page_text: {excerpt}"#
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let url_api = format!("{}/api/generate", local.base_url.trim_end_matches('/'));
    let body = json!({
        "model": local.model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "options": {
            "temperature": 0.1,
            "num_predict": 120
        }
    });

    let res = with_coach_auth(client.post(url_api), cfg)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("coach model request failed: {e}"))?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        // Invalidate probe cache so UI can show pull instructions.
        if let Ok(mut guard) = PROBE_CACHE.lock() {
            *guard = None;
        }
        return Err(format!("local model HTTP {status}: {text}"));
    }

    let wrapped: Value =
        serde_json::from_str(&text).map_err(|e| format!("local model bad envelope: {e}"))?;
    let raw = wrapped["response"]
        .as_str()
        .ok_or_else(|| "local model returned no response text".to_string())?;
    let judged = parse_judge_json(raw)?;
    let confidence = judged.confidence.unwrap_or(0.6).clamp(0.0, 1.0);
    let on_task = judged.on_task;
    let distraction = if on_task {
        None
    } else {
        judged
            .distraction
            .filter(|s| !s.is_empty())
            .or_else(|| Some(kind.to_string()))
    };
    let coach_line = judged
        .coach_line
        .filter(|s| !is_placeholder_coach_line(s) && !coach_line_off_mission(s, goals))
        .unwrap_or_else(|| {
            if on_task {
                format!(
                    "This looks on track for {} — keep going.",
                    goals_snippet(goals, 48)
                )
            } else {
                format!(
                    "{} isn’t your mission — get back to {}.",
                    if kind.is_empty() { "That" } else { kind },
                    goals_snippet(goals, 48)
                )
            }
        });

    Ok(LocalJudgment {
        confidence,
        model: local.model,
        result: CoachVisionResult {
            on_task,
            objects: vec![app.to_string(), title.to_string(), kind.to_string()],
            distraction,
            needs_help: false,
            stress_cue: false,
            coach_line,
            modality: Some("computer".into()),
        },
    })
}

fn parse_judge_json(raw: &str) -> Result<JudgeJson, String> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    // Models sometimes wrap extra text — pull the first {...} block.
    let slice = if let (Some(start), Some(end)) = (cleaned.find('{'), cleaned.rfind('}')) {
        &cleaned[start..=end]
    } else {
        cleaned
    };
    serde_json::from_str::<JudgeJson>(slice)
        .map_err(|e| format!("local model JSON parse: {e}; raw={raw}"))
}

fn normalize_line(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Short mission fragment for concrete check-in lines.
fn goals_snippet(goals: &str, max_chars: usize) -> String {
    let cleaned = goals
        .split(|c| c == '\n' || c == ';' || c == '|')
        .map(str::trim)
        .find(|p| p.len() >= 4)
        .unwrap_or(goals)
        .trim();
    let cleaned = cleaned.trim_matches(|c: char| c == '"' || c == '\'' || c == '.');
    if cleaned.is_empty() {
        return "your lock-in goal".into();
    }
    let mut out: String = cleaned.chars().take(max_chars).collect();
    if cleaned.chars().count() > max_chars {
        out.push('…');
    }
    out
}

/// Tiny models often echo schema hints (“one short sentence”, “short”) — never show those.
pub fn is_placeholder_coach_line(text: &str) -> bool {
    let t = normalize_line(text)
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == '.')
        .to_string();
    if t.is_empty() {
        return true;
    }
    // Multi-word instruction / schema leaks (substring match).
    const PHRASES: &[&str] = &[
        "one short sentence",
        "short sentence",
        "a short sentence",
        "one sentence",
        "short nudge",
        "gentle nudge",
        "final short nudge",
        "coach line",
        "coaching line",
        "coach_line",
        "your sentence here",
        "insert sentence",
        "example nudge",
        "specific nudge",
        "write one",
        "return only",
        "return the sentence",
        "only the sentence",
        "no quotes",
        "no json",
        "no markdown",
    ];
    if PHRASES.iter().any(|p| t.contains(p)) {
        return true;
    }
    // Single-token schema crumbs — exact match only (don't kill “Last nudge: …”).
    const EXACT: &[&str] = &[
        "short", "label", "sentence", "nudge", "todo", "tbd", "n/a", "none", "null", "true",
        "false",
    ];
    if EXACT.iter().any(|p| t == *p) {
        return true;
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    // Ultra-short lines are almost always schema crumbs unless they name a real target.
    if words.len() < 4 {
        let concrete = words.iter().any(|w| {
            matches!(
                *w,
                "discord"
                    | "instagram"
                    | "youtube"
                    | "email"
                    | "shopping"
                    | "texting"
                    | "phone"
                    | "tiktok"
                    | "reddit"
                    | "twitter"
                    | "slack"
            ) || w.chars().any(|c| c.is_ascii_digit())
        });
        if !concrete {
            return true;
        }
    }
    false
}

/// Pull `DEPT` / `DEPT 1230` style tokens from text (ASCII uppercase scan).
fn course_like_tokens(text: &str) -> Vec<String> {
    let up = text.to_uppercase();
    let bytes = up.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let letters = &up[start..i];
            // Skip spaces/dashes between dept and number.
            let mut j = i;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'-') {
                j += 1;
            }
            let num_start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            // Optional trailing letter (CS 1110A).
            if j < bytes.len() && j > num_start && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            let has_num = j > num_start;
            if has_num && (2..=6).contains(&letters.len()) {
                out.push(format!("{letters} {}", &up[num_start..j]));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Course codes mentioned in a nudge but absent from the active mission goals.
fn foreign_course_tokens(text: &str, goals: &str) -> bool {
    let goals_up = goals.to_uppercase();
    for token in course_like_tokens(text) {
        let compact = token.replace(' ', "");
        if goals_up.contains(&token) || goals_up.contains(&compact) {
            continue;
        }
        return true;
    }
    false
}

/// True when a model nudge drifts off the active mission (e.g. ENGL while goals say coding).
fn coach_line_off_mission(text: &str, goals: &str) -> bool {
    let goals = goals.trim();
    if goals.is_empty() || text.trim().is_empty() {
        return false;
    }
    if foreign_course_tokens(text, goals) {
        return true;
    }
    // Significant goal tokens (≥4 letters/digits) — require at least one overlap so
    // "finish your ENGL discussion" can't survive a coding mission.
    let goal_tokens: Vec<String> = normalize_line(goals)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 4)
        .map(|t| t.to_string())
        .collect();
    if goal_tokens.is_empty() {
        return false;
    }
    let line = normalize_line(text);
    !goal_tokens.iter().any(|t| line.contains(t.as_str()))
}

/// Replace placeholder/meta/off-mission coach text with a concrete mission-aware fallback.
pub fn sanitize_coach_line(text: &str, kind: &str, distraction: &str, goals: &str) -> String {
    let trimmed = text.trim();
    if !is_placeholder_coach_line(trimmed) && !coach_line_off_mission(trimmed, goals) {
        return trimmed.to_string();
    }
    pick_fallback(
        kind,
        distraction,
        goals,
        1,
        &recent_lines_snapshot(),
    )
}

fn recent_lines_snapshot() -> Vec<String> {
    RECENT_LINES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn remember_line(line: &str) {
    let cleaned = line.trim();
    if cleaned.is_empty() {
        return;
    }
    if let Ok(mut guard) = RECENT_LINES.lock() {
        guard.push(cleaned.to_string());
        while guard.len() > RECENT_LINE_CAP {
            guard.remove(0);
        }
    }
}

fn too_similar(candidate: &str, recent: &[String]) -> bool {
    let norm = normalize_line(candidate);
    if norm.is_empty() {
        return true;
    }
    recent.iter().any(|prev| {
        let p = normalize_line(prev);
        p == norm || (p.len() > 12 && norm.contains(&p)) || (norm.len() > 12 && p.contains(&norm))
    })
}

fn fallback_templates(kind: &str, distraction: &str, goals: &str, nag_n: u32) -> Vec<String> {
    let d = if distraction.is_empty() {
        "that tab"
    } else {
        distraction
    };
    let g = goals_snippet(goals, 56);
    match kind {
        "watching" => vec![
            format!("I’m with you on {g} — I’ll nudge you if you drift."),
            format!("Mission’s live for {g}. I’ll keep an eye on your path."),
            format!("Locked in on {g}. I’ll tap you gently if you wander."),
            format!("I’ve got your back while you work on {g}."),
        ],
        "encourage" | "on_task" => vec![
            format!("Nice — this looks on track for {g}. Keep going."),
            format!("This fits {g}. Stay with it a bit longer."),
            format!("Good pullback toward {g}. Ride this focus."),
            format!("You’re on {g} — keep that momentum."),
        ],
        _ => {
            let mut lines = vec![
                format!("Heads up — {d} isn’t {g}. Close it and return."),
                format!("Quick check: {d} pulled you off {g}. Switch back."),
                format!("That’s {d}, not {g}. One click back to the work."),
                format!("Mission drift via {d}. Reset to {g}."),
            ];
            if nag_n >= 2 {
                lines.push(format!("Still on {d}? Close it and finish {g}."));
            }
            if nag_n >= 3 {
                lines.push(format!("Last nudge: leave {d} and finish {g}."));
            }
            lines
        }
    }
}

fn pick_fallback(kind: &str, distraction: &str, goals: &str, nag_n: u32, recent: &[String]) -> String {
    let templates = fallback_templates(kind, distraction, goals, nag_n);
    templates
        .into_iter()
        .find(|t| !too_similar(t, recent))
        .unwrap_or_else(|| {
            format!(
                "Leave {} and get back to {}.",
                if distraction.is_empty() {
                    "that distraction"
                } else {
                    distraction
                },
                goals_snippet(goals, 56)
            )
        })
}

/// Fresh spoken/overlay line from the local model (varied; avoids recent repeats).
pub async fn compose_coach_line(
    cfg: &AppConfig,
    kind: &str,
    goals: &str,
    distraction: &str,
    detail: &str,
    nag_n: u32,
) -> String {
    let recent = recent_lines_snapshot();
    let fallback = pick_fallback(kind, distraction, goals, nag_n, &recent);

    if !cfg.local_llm_enabled || !is_available(cfg).await {
        remember_line(&fallback);
        return fallback;
    }

    let local = LocalJudgeConfig::from_app(cfg);
    let avoid = if recent.is_empty() {
        "(none yet)".into()
    } else {
        recent
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let goals_short = goals_snippet(goals, 120);
    let detail_short: String = detail.chars().take(120).collect();
    let intensity = match nag_n {
        0 | 1 => "first gentle nudge",
        2 => "second reminder — firmer, still kind",
        _ => "final short nudge for this distraction",
    };
    let prompt = format!(
        r#"Write ONE specific check-in sentence for a student mid lock-in.
Return ONLY that sentence — no quotes, no JSON, no markdown, no instructions.
Rules:
- Max 22 words. Warm, direct, slightly playful space/mission tone.
- MUST name the mission goal (use words from: {goals_short}) and, if off-task, the distraction ({distraction}).
- Be concrete (e.g. “Close Instagram and finish your BIOMG quiz.”) — never meta phrases like “one short sentence”.
- Do NOT reuse or paraphrase these recent lines: {avoid}
- Never dump URLs or tech jargon. Vary wording every time.

kind: {kind}
intensity: {intensity}
distraction: {distraction}
context: {detail_short}
mission goals: {goals_short}"#
    );

    let client = match Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            remember_line(&fallback);
            return fallback;
        }
    };
    let url_api = format!("{}/api/generate", local.base_url.trim_end_matches('/'));
    let body = json!({
        "model": local.model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.85,
            "num_predict": 48
        }
    });

    let Ok(res) = with_coach_auth(client.post(url_api), cfg)
        .json(&body)
        .send()
        .await
    else {
        remember_line(&fallback);
        return fallback;
    };
    if !res.status().is_success() {
        remember_line(&fallback);
        return fallback;
    }
    let Ok(wrapped) = res.json::<Value>().await else {
        remember_line(&fallback);
        return fallback;
    };
    let raw = wrapped["response"].as_str().unwrap_or("").trim();
    let line = raw
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
        .lines()
        .next()
        .unwrap_or("")
        .trim();
    let line: String = line.chars().take(160).collect();
    let chosen = if is_placeholder_coach_line(&line)
        || coach_line_off_mission(&line, goals)
        || too_similar(&line, &recent)
    {
        fallback
    } else {
        line
    };
    remember_line(&chosen);
    chosen
}

/// Prefer a non-empty model line if it isn’t a recent duplicate; else compose a new one.
pub async fn freshen_coach_line(
    cfg: &AppConfig,
    existing: &str,
    kind: &str,
    goals: &str,
    distraction: &str,
    detail: &str,
    nag_n: u32,
) -> String {
    let recent = recent_lines_snapshot();
    let existing = existing.trim();
    if !is_placeholder_coach_line(existing) && !coach_line_off_mission(existing, goals) {
        // Keep a line we just composed (it's already the newest recent entry).
        if recent
            .last()
            .is_some_and(|r| normalize_line(r) == normalize_line(existing))
        {
            return existing.to_string();
        }
        let prior: Vec<String> = recent
            .iter()
            .filter(|r| normalize_line(r) != normalize_line(existing))
            .cloned()
            .collect();
        if !too_similar(existing, &prior) {
            remember_line(existing);
            return existing.to_string();
        }
    }
    compose_coach_line(cfg, kind, goals, distraction, detail, nag_n).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_schema_echo_placeholders() {
        assert!(is_placeholder_coach_line("one short sentence"));
        assert!(is_placeholder_coach_line("One short sentence."));
        assert!(is_placeholder_coach_line("short nudge"));
        assert!(is_placeholder_coach_line("short"));
        assert!(is_placeholder_coach_line("Keep going."));
        assert!(is_placeholder_coach_line(""));
        assert!(!is_placeholder_coach_line(
            "Close Discord and finish your ENGL 1140 discussion post."
        ));
        assert!(!is_placeholder_coach_line(
            "Instagram isn’t BIOMG quiz — switch back."
        ));
        assert!(!is_placeholder_coach_line(
            "Last nudge: leave discord and finish ENGL 1140."
        ));
        let sanitized = sanitize_coach_line(
            "one short sentence",
            "distracted",
            "instagram",
            "ENGL 1140 discussion post",
        );
        assert!(!is_placeholder_coach_line(&sanitized));
        assert!(sanitized.to_lowercase().contains("instagram"));
    }

    #[test]
    fn rejects_off_mission_engl_when_goals_are_coding() {
        let sanitized = sanitize_coach_line(
            "Close Discord and finish your ENGL discussion post.",
            "distracted",
            "discord",
            "work on coding please!",
        );
        assert!(
            !sanitized.to_uppercase().contains("ENGL"),
            "off-mission ENGL leaked: {sanitized}"
        );
        assert!(
            sanitize_coach_line(
                sanitized.as_str(),
                "distracted",
                "discord",
                "work on coding please!",
            )
            .to_lowercase()
            .contains("coding")
                || sanitized.to_lowercase().contains("coding"),
            "expected coding mission in nudge: {sanitized}"
        );
    }

    #[test]
    fn goals_snippet_picks_first_useful_clause() {
        let s = goals_snippet(
            "ENGL 1140 discussion post; BIOMG 1350 pre-lecture quiz",
            80,
        );
        assert!(s.contains("ENGL 1140"));
        assert!(!s.contains(';'));
    }

    #[test]
    fn fallback_names_goal_and_distraction() {
        let line = pick_fallback(
            "distracted",
            "instagram",
            "ENGL 1140 discussion post",
            1,
            &[],
        );
        assert!(line.to_lowercase().contains("instagram"));
        assert!(line.contains("ENGL 1140"));
        assert!(!is_placeholder_coach_line(&line));
    }
}
