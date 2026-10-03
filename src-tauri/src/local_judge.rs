//! On-device on-task classifier via a small quantized model (Ollama).
//! Prefer this over Gemini text/vision whenever it is available and confident.

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
        format!("Coach ready · {}", local.model)
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
                "start Waypoint API on this Mac".into(),
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
    let prompt = format!(
        r#"You are a strict study lock-in classifier. Decide if the student is ON TASK for their goals.
Reply with ONLY compact JSON, no markdown:
{{"on_task":true|false,"confidence":0.0-1.0,"distraction":null|"label","coach_line":"one short sentence"}}

Rules:
- on_task=true only if this clearly advances the goals (coursework, lecture, tutorial matching goals).
- YouTube/video: study/lecture/tutorial matching goals → on_task=true; music/gaming/vlogs/entertainment → false, distraction="youtube".
- Discord, Instagram, shopping, email, texting → usually on_task=false.
- If unsure, confidence < 0.5 and lean on_task=false for entertainment sites.

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
    let coach_line = judged.coach_line.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        if on_task {
            "Looks aligned with your lock-in — keep going.".into()
        } else {
            format!("This {kind} doesn’t look like your lock-in — switch back to the work.")
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

fn fallback_templates(kind: &str, distraction: &str, nag_n: u32) -> Vec<String> {
    let d = if distraction.is_empty() {
        "that"
    } else {
        distraction
    };
    match kind {
        "watching" => vec![
            "I’m with you — I’ll nudge you if you drift.".into(),
            "Mission’s live. I’ll keep an eye on your path.".into(),
            "Locked in. I’ll tap you gently if you wander.".into(),
            "I’ve got your back for this stretch.".into(),
        ],
        "encourage" | "on_task" => vec![
            "Nice — this looks on track. Keep going.".into(),
            "This fits your mission. Stay with it.".into(),
            "Good pullback. Ride this focus a bit longer.".into(),
            "You’re on the work — keep that momentum.".into(),
        ],
        _ => {
            let mut lines = vec![
                format!("Heads up — {d} isn’t the mission. Slide back to your goal."),
                format!("Quick check: {d} pulled you off. Close it and return."),
                format!("That’s {d}, not the work. One click back to your goal."),
                format!("Mission drift via {d}. Reset to what you locked in on."),
            ];
            if nag_n >= 2 {
                lines.push(format!("Still on {d}? Close it so you can finish the mission."));
            }
            if nag_n >= 3 {
                lines.push(format!("Last nudge: leave {d} and finish what you started."));
            }
            lines
        }
    }
}

fn pick_fallback(kind: &str, distraction: &str, nag_n: u32, recent: &[String]) -> String {
    let templates = fallback_templates(kind, distraction, nag_n);
    templates
        .into_iter()
        .find(|t| !too_similar(t, recent))
        .unwrap_or_else(|| {
            format!(
                "Switch back to your mission — skip the repeat on {}.",
                if distraction.is_empty() {
                    "distractions"
                } else {
                    distraction
                }
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
    let fallback = pick_fallback(kind, distraction, nag_n, &recent);

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
    let goals_short: String = goals.chars().take(180).collect();
    let detail_short: String = detail.chars().take(120).collect();
    let intensity = match nag_n {
        0 | 1 => "first gentle nudge",
        2 => "second reminder — firmer, still kind",
        _ => "final short nudge for this distraction",
    };
    let prompt = format!(
        r#"Write ONE fresh coaching line for a student on a focus mission.
Return ONLY the sentence — no quotes, no JSON, no markdown.
Rules:
- Max 18 words. Warm, direct, slightly playful space/mission tone.
- Do NOT reuse or paraphrase these recent lines: {avoid}
- Mention the distraction only if relevant; never dump URLs or tech jargon.
- Vary wording every time.

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
    let chosen = if line.split_whitespace().count() < 3 || too_similar(&line, &recent) {
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
    if existing.split_whitespace().count() >= 3 {
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
