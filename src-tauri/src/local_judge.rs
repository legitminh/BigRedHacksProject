//! On-device on-task classifier via a small quantized model (Ollama / Llama).
//! Lock-in never calls Gemini — local provider only.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api;
use crate::config::AppConfig;
use crate::gemini::CoachVisionResult;

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
    let url = format!("{}/api/tags", cfg.base_url.trim_end_matches('/'));
    let res = match api::coach_authed_raw(
        app,
        reqwest::Method::GET,
        &url,
        None,
        Duration::from_secs(2),
    )
    .await
    {
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

/// Strip query/fragment before cloud coach payloads (tokens in URLs stay local).
fn url_for_judge(url: &str) -> String {
    let base = url.split('#').next().unwrap_or(url);
    base.split('?').next().unwrap_or(base).to_string()
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
    let safe_url = url_for_judge(url);
    let goal_hint = goals_snippet(goals, 48);
    let prompt = format!(
        r#"You are a strict study lock-in classifier. Decide if the student is ON TASK for their goals.
Reply with ONLY compact JSON, no markdown:
{{"on_task":true|false,"confidence":0.0-1.0,"distraction":null|"youtube","coach_line":"Close Discord and get back to {goal_hint}."}}

Rules:
- Decide from title, url, and page_text — NOT from the kind label alone, and NOT from tab-group names like "School".
- on_task=true only if this clearly advances the goals (coursework, lecture, tutorial matching goals).
- YouTube/video are CONTEXTUAL: lecture/tutorial matching goals → on_task=true; music/rap/artist tracks/"no music"/vlogs/memes/entertainment → false, distraction="youtube". Gameplay matching game-design/playtest goals → may be true.
- Discord is CONTEXTUAL: study/homework help matching goals → may be true; meme spam → false; gaming channels matching playtest/game goals → may be true, else false, distraction="discord".
- Reading/papers/PDFs/scholar/arxiv are CONTEXTUAL: on_task=true ONLY if the paper/title advances the goals — being academic is not enough; off-topic papers → false, distraction="reading".
- Gaming/Steam/Epic are CONTEXTUAL: on_task=true ONLY if goals clearly include that game/playtest; otherwise false, distraction="gaming".
- Instagram, shopping, email, texting-class → ALWAYS on_task=false (never study).
- If unsure, confidence < 0.5 and lean on_task=false for entertainment / off-topic content.
- coach_line MUST name the distraction AND reuse words from the goals field only — never invent other courses, assignments, or discussion posts that are not in goals.
- When off-task, coach_line must tell them to leave the distraction and refocus on goals — NEVER suggest taking a break, resting, or stepping away (breaks are only for stress/tiredness).
- Never output meta text like "one short sentence" or "short nudge".

goals: {goals}
kind: {kind}
app: {app}
title: {title}
url: {safe_url}
page_text: {excerpt}"#
    );

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

    let res = api::coach_authed_raw(
        cfg,
        reqwest::Method::POST,
        &url_api,
        Some(&body),
        Duration::from_secs(20),
    )
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
    // Prefer OS/URL kind evidence over model distraction guesses (never YouTube→Instagram).
    let distraction = if on_task {
        None
    } else {
        let model_d = judged.distraction.filter(|s| !s.is_empty());
        let kind_l = kind.trim().to_lowercase();
        let evidence = DISTRACTION_LABELS.iter().any(|l| *l == kind_l);
        if evidence {
            Some(kind_l)
        } else {
            model_d.or_else(|| {
                if kind.is_empty() {
                    None
                } else {
                    Some(kind.to_string())
                }
            })
        }
    };
    let d_for_line = distraction.as_deref().unwrap_or(kind);
    let raw_line = judged.coach_line.unwrap_or_default();
    let coach_line = if on_task {
        let line = raw_line.trim();
        if line.is_empty() || is_placeholder_coach_line(line) {
            format!(
                "This looks on track for {} — keep going.",
                goals_snippet(goals, 48)
            )
        } else {
            sanitize_coach_line(line, "on_task", "on_task", goals)
        }
    } else {
        sanitize_coach_line(raw_line.trim(), "distracted", d_for_line, goals)
    };

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

/// Known distraction surface names (spoken / overlay).
const DISTRACTION_LABELS: &[&str] = &[
    "instagram",
    "discord",
    "youtube",
    "tiktok",
    "reddit",
    "twitter",
    "shopping",
    "email",
    "texting",
    "slack",
    "phone",
    "facebook",
    "snapchat",
    "netflix",
    "twitch",
];

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
        "do not reuse",
        "never invent",
        "return only that sentence",
        "max 22 words",
        "warm, direct",
        "mission goals:",
        "intensity:",
        "kind:",
    ];
    if PHRASES.iter().any(|p| t.contains(p)) {
        return true;
    }
    // Pipe-joined avoid-list / template dumps (e.g. "… | Quick check: instagram p").
    if coach_line_has_prompt_leak(text) {
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
            DISTRACTION_LABELS.iter().any(|l| *w == *l)
                || w.chars().any(|c| c.is_ascii_digit())
        });
        if !concrete {
            return true;
        }
    }
    // Compose prompt asks for ≤22 words; longer blobs are almost always concatenations.
    if words.len() > 28 {
        return true;
    }
    false
}

/// Detect model dumps of avoid-lists, duplicated templates, or truncated meta crumbs.
fn coach_line_has_prompt_leak(text: &str) -> bool {
    let raw = text.trim();
    if raw.is_empty() {
        return false;
    }
    // Pipe-joined recent-line dumps from compose prompts.
    if raw.contains('|') {
        return true;
    }
    let lower = normalize_line(raw);
    // Duplicated fallback / intensity crumbs.
    let quick_check_hits = lower.matches("quick check").count();
    if quick_check_hits >= 2 {
        return true;
    }
    // Glued draft + fallback template (even with a single "Quick check").
    if (lower.contains("close ") || lower.contains("heads up"))
        && lower.contains("quick check")
    {
        return true;
    }
    // Truncated tails like "instagram p" / "coding proje".
    if looks_truncated_coach_tail(raw) {
        return true;
    }
    // Classic prompt-example echoes (hard-coded in older prompts).
    if lower.contains("close instagram and finish your biomg")
        || lower.contains("finish your biomg quiz")
    {
        return true;
    }
    false
}

fn looks_truncated_coach_tail(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    // Ends mid-word after a space + 1–2 letters (e.g. "instagram p").
    let bytes = t.as_bytes();
    if let Some(last_space) = t.rfind(' ') {
        let tail = t[last_space + 1..]
            .trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        if (1..=2).contains(&tail.len()) && tail.chars().all(|c| c.is_ascii_alphabetic()) {
            return true;
        }
    }
    // Trailing pipe / dangling punctuation without a sentence end.
    if matches!(bytes.last(), Some(b'|') | Some(b':') | Some(b',')) {
        return true;
    }
    false
}

/// Pull `DEPT` / `DEPT 1230` style tokens from text (ASCII uppercase scan).
/// Also captures bare dept codes (BIOMG, ENGL) when 3–6 letters — common model hallucinations.
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
            // Bare dept-looking codes (BIOMG, ENGL, CS) — skip common English/tech words.
            if (3..=6).contains(&letters.len()) && looks_like_dept_code(letters) {
                out.push(letters.to_string());
            }
        }
        i += 1;
    }
    out
}

fn looks_like_dept_code(letters: &str) -> bool {
    // Common non-course uppercase words / tech acronyms that appear in coding missions.
    const SKIP: &[&str] = &[
        "THE", "AND", "FOR", "YOU", "YOUR", "NOT", "GET", "BACK", "OFF", "VIA", "THAT",
        "THIS", "WITH", "FROM", "INTO", "HAVE", "WILL", "JUST", "WANT", "NEED", "GOAL",
        "LOCK", "WORK", "CODE", "HTML", "CSS", "JSON", "HTTP", "API", "URL", "TTS",
        "OCR", "LLM", "VLM", "IDE", "CLI", "APP", "TAB", "PDF", "DOC", "ZIP", "PNG",
        "JPG", "JPEG", "SVG", "GIT", "NPM", "SQL", "CPU", "GPU", "RAM", "OSX", "IOS",
        "MAC", "WIN", "WEB", "UI", "UX", "AI", "ML", "NLP", "SDK", "CDN", "DNS",
        "ONE", "TWO", "ALL", "ANY", "OUT", "NOW", "STILL", "LAST", "NEXT", "OVER",
        "HEADS", "NICE", "KEEP", "STAY", "RIDE", "LEAVE", "CLOSE", "SWITCH", "RESET",
        "PULL", "CLICK", "FINISH", "RETURN", "MISSION", "DRIFT", "CHECK", "QUICK",
    ];
    if SKIP.iter().any(|s| *s == letters) {
        return false;
    }
    // Dept codes are usually consonant-heavy (BIOMG, ENGL, MATH, CS, CHEM).
    let vowels = letters.chars().filter(|c| matches!(c, 'A' | 'E' | 'I' | 'O' | 'U')).count();
    let consonants = letters.len().saturating_sub(vowels);
    consonants >= vowels || letters.len() <= 3
}

/// Course codes mentioned in a nudge but absent from the active mission goals.
fn foreign_course_tokens(text: &str, goals: &str) -> bool {
    let goals_up = goals.to_uppercase();
    for token in course_like_tokens(text) {
        let compact = token.replace(' ', "");
        if goals_up.contains(&token) || goals_up.contains(&compact) {
            continue;
        }
        // Bare dept: also accept if goals contain the letters as a code prefix.
        if !token.contains(' ') && goals_up.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == token) {
            continue;
        }
        return true;
    }
    false
}

/// Assignment nouns invented by the model (quiz/exam/…) that aren't in active goals.
fn foreign_assignment_nouns(text: &str, goals: &str) -> bool {
    const NOUNS: &[&str] = &[
        "quiz",
        "exam",
        "midterm",
        "final exam",
        "essay",
        "homework",
        "problem set",
        "pset",
        "discussion post",
        "lab report",
        "prelab",
        "pre-lab",
        "worksheet",
    ];
    let line = normalize_line(text);
    let goals_l = normalize_line(goals);
    NOUNS.iter().any(|n| line.contains(n) && !goals_l.contains(n))
}

/// True when a model nudge drifts off the active mission (e.g. ENGL while goals say coding).
fn coach_line_off_mission(text: &str, goals: &str) -> bool {
    let goals = goals.trim();
    if goals.is_empty() || text.trim().is_empty() {
        return false;
    }
    // Foreign courses/assignments always win — even if the blob also mentions goal words
    // (concatenated dumps often append a correct fallback after a BIOMG hallucination).
    if foreign_course_tokens(text, goals) || foreign_assignment_nouns(text, goals) {
        return true;
    }
    // Significant goal tokens (≥4 letters/digits) — require at least one overlap so
    // "finish your ENGL discussion" can't survive a coding mission.
    let goal_tokens: Vec<String> = normalize_line(goals)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 4)
        .filter(|t| {
            !matches!(
                *t,
                "with" | "this" | "that" | "from" | "study" | "work" | "lock" | "into" | "about"
                    | "have" | "will" | "just" | "want" | "need" | "goal" | "goals" | "session"
                    | "please" | "your"
            )
        })
        .map(|t| t.to_string())
        .collect();
    if goal_tokens.is_empty() {
        return false;
    }
    let line = normalize_line(text);
    !goal_tokens.iter().any(|t| line.contains(t.as_str()))
}

/// Off-task / distraction coaching context — must never invite a break.
fn is_distraction_coach_context(kind: &str, distraction: &str) -> bool {
    const OFF: &[&str] = &[
        "distracted",
        "offtask",
        "off_task",
        "off-task",
        "phone",
        "instagram",
        "discord",
        "youtube",
        "shopping",
        "email",
        "texting",
        "tiktok",
        "reddit",
        "twitter",
        "slack",
        "other",
        "social",
    ];
    let k = kind.to_lowercase();
    let d = distraction.to_lowercase();
    OFF.iter().any(|x| k == *x || d == *x)
}

/// Model sometimes invents “take a break” for Instagram/Discord — only stress paths may.
pub fn coach_line_suggests_break(text: &str) -> bool {
    let t = normalize_line(text);
    const PHRASES: &[&str] = &[
        "take a break",
        "take a short break",
        "take a quick break",
        "take a little break",
        "take a five-minute break",
        "take a 5-minute break",
        "take five minutes",
        "take 5 minutes",
        "optional five-minute break",
        "optional 5-minute break",
        "suggest a break",
        "suggested break",
        "time for a break",
        "need a break",
        "deserve a break",
        "have a break",
        "go on a break",
        "go take a break",
        "step away for a",
        "rest for a bit",
        "rest a bit",
        "rest and come back",
    ];
    PHRASES.iter().any(|p| t.contains(p))
}

/// First known distraction surface named in the spoken line, if any.
fn distraction_label_in_line(text: &str) -> Option<&'static str> {
    let t = normalize_line(text);
    DISTRACTION_LABELS.iter().copied().find(|l| t.contains(l))
}

/// All known distraction surfaces named in prose (concat dumps may name several).
fn distractions_named_in_line(text: &str) -> Vec<&'static str> {
    let t = normalize_line(text);
    DISTRACTION_LABELS
        .iter()
        .copied()
        .filter(|l| t.contains(l))
        .collect()
}

/// Prose names a different site than the OS/URL/OCR evidence label.
fn coach_line_wrong_distraction(text: &str, distraction: &str) -> bool {
    let expected = distraction.trim().to_lowercase();
    if expected.is_empty() || is_status_kind_token(&expected) || expected == "other" {
        return false;
    }
    let named = distractions_named_in_line(text);
    if named.is_empty() {
        return false;
    }
    // Any named site that isn't the evidence label → stale model / prompt echo.
    named.iter().any(|n| *n != expected.as_str())
}

fn is_stress_break_invite_kind(kind: &str) -> bool {
    matches!(
        kind.to_lowercase().as_str(),
        "suggest_break" | "stressed"
    )
}

/// Camera accountability + calm coach kinds — never treat the tag as a distraction app.
/// Server/desktop already authored the spoken line; keep it verbatim (except placeholders).
fn is_verbatim_coach_kind(kind: &str) -> bool {
    matches!(
        kind.to_lowercase().as_str(),
        "watching"
            | "encourage"
            | "on_task"
            | "left_desk"
            | "left_desk_pause"
            | "welcome_back"
            | "camera_obstructed"
            | "camera"
            | "suggest_break"
            | "stressed"
    )
}

fn coach_line_needs_rewrite(text: &str, kind: &str, distraction: &str, goals: &str) -> bool {
    if is_placeholder_coach_line(text) {
        return true;
    }
    // Presence / watching / stress invites omit mission words on purpose.
    // Never rewrite them into "left_desk isn't {goal}" distraction templates.
    if is_verbatim_coach_kind(kind) {
        // Exception: a "stressed" line that also names Instagram/etc → refocus, not break.
        if is_stress_break_invite_kind(kind)
            && (is_distraction_coach_context("", distraction)
                || distraction_label_in_line(text).is_some())
            && coach_line_suggests_break(text)
        {
            return true;
        }
        return false;
    }
    // Stress/break invites often omit mission words — don't treat that as off-mission.
    // Still rewrite if the line also names a distraction surface (refocus, not break).
    let pure_stress_invite = is_stress_break_invite_kind(kind)
        && !is_distraction_coach_context("", distraction)
        && distraction_label_in_line(text).is_none();
    if pure_stress_invite {
        return false;
    }
    if coach_line_off_mission(text, goals) {
        return true;
    }
    if coach_line_wrong_distraction(text, distraction) {
        return true;
    }
    // Break language is fine only on explicit stress/break-invite kinds with no distraction label.
    coach_line_suggests_break(text)
        && (is_distraction_coach_context(kind, distraction)
            || distraction_label_in_line(text).is_some())
}

/// Status/kind tokens that must not appear as the named distraction surface.
fn is_status_kind_token(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "distracted"
            | "offtask"
            | "off_task"
            | "off-task"
            | "stressed"
            | "suggest_break"
            | "on_task"
            | "needs_help"
            | "watching"
            | "encourage"
            | "camera"
            | "left_desk"
            | "left_desk_pause"
            | "welcome_back"
            | "camera_obstructed"
    )
}

/// Concrete surface label for refocus fallbacks (never “distracted” / status kinds).
/// Prefer OS/URL/OCR evidence (`distraction`) over site names hallucinated in prose.
fn refocus_distraction_label(kind: &str, distraction: &str, text: &str) -> String {
    let d = distraction.trim();
    if !d.is_empty() && !is_status_kind_token(d) {
        return d.to_string();
    }
    let k = kind.trim();
    if !k.is_empty() && !is_status_kind_token(k) {
        return k.to_string();
    }
    // Never mine brands from prompt-leak / BIOMG-echo dumps — deliver_ephemeral often
    // passes kind=distracted for both args, and mining would revive "Instagram".
    if !coach_line_has_prompt_leak(text) {
        if let Some(label) = distraction_label_in_line(text) {
            return label.to_string();
        }
    }
    String::new() // pick_fallback → “that tab”
}

/// Replace placeholder/meta/off-mission/break-for-distraction text with a refocus fallback.
pub fn sanitize_coach_line(text: &str, kind: &str, distraction: &str, goals: &str) -> String {
    let trimmed = text.trim();
    if !coach_line_needs_rewrite(trimmed, kind, distraction, goals) {
        return trimmed.to_string();
    }
    let d = refocus_distraction_label(kind, distraction, trimmed);
    let k = if is_distraction_coach_context(kind, distraction)
        || distraction_label_in_line(trimmed).is_some()
    {
        "distracted"
    } else {
        kind
    };
    pick_fallback(k, &d, goals, 1, &recent_lines_snapshot())
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
    let d = if distraction.is_empty() || is_status_kind_token(distraction) {
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
        // Camera accountability from Presage face-lost (D1) — never "tag isn't goal", never accuse phone.
        "left_desk" => vec![
            "You've stepped away. Come back to the work when you can.".into(),
            "Still away from the desk — return when you’re ready.".into(),
            "Camera lost you. Come back to the work.".into(),
        ],
        "left_desk_pause" => vec![
            "I’ll stay quiet until you’re back at the desk.".into(),
            "Still away — I’ll pause check-ins until you’re back.".into(),
        ],
        "welcome_back" => vec![
            "Welcome back — good to see you. Let's pick the work back up.".into(),
            "Welcome back. Stay with the work.".into(),
        ],
        "camera_obstructed" | "camera" => vec![
            "I can’t see you clearly. Check the camera or lighting.".into(),
            "Camera’s unclear — fix lighting or uncover the lens.".into(),
        ],
        "suggest_break" => vec![
            "Feeling tense — optional five-minute break?".into(),
        ],
        "stressed" => vec![
            "You seem tense — one slow breath, then back.".into(),
        ],
        _ => {
            // Avoid "Quick check:" here — models echo it into dumps that collide with the UI kicker.
            let mut lines = vec![
                format!("Heads up — {d} isn’t {g}. Close it and return."),
                format!("{d} pulled you off {g}. Switch back."),
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
    // Prefer numbered lines over `|` joins — models echo pipe-joined avoid lists into overlays.
    let avoid = if recent.is_empty() {
        "(none yet)".into()
    } else {
        recent
            .iter()
            .rev()
            .take(5)
            .enumerate()
            .map(|(i, line)| format!("{}. {line}", i + 1))
            .collect::<Vec<_>>()
            .join(" / ")
    };
    let goals_short = goals_snippet(goals, 120);
    let detail_short: String = detail.chars().take(120).collect();
    let intensity = match nag_n {
        0 | 1 => "first gentle nudge",
        2 => "second reminder — firmer, still kind",
        _ => "final firmer nudge for this distraction",
    };
    let off_task = is_distraction_coach_context(kind, distraction);
    let break_rule = if off_task {
        "- OFF-TASK: tell them to leave the distraction and refocus on the mission. NEVER suggest taking a break, resting, or stepping away — breaks are only for stress/tiredness, not distraction."
    } else {
        "- Do not invent a break invite unless this is explicitly a stress/tiredness check-in."
    };
    let example_distraction = if distraction.is_empty() {
        "that tab"
    } else {
        distraction
    };
    let prompt = format!(
        r#"Write ONE specific check-in sentence for a student mid lock-in.
Return ONLY that sentence — no quotes, no JSON, no markdown, no instructions.
Rules:
- Max 22 words. Warm, direct, slightly playful space/mission tone.
- MUST name the mission goal (use words from: {goals_short}) and, if off-task, the distraction ({example_distraction}).
- Be concrete — name ONLY the given distraction and goals. Example shape: “Close {example_distraction} and get back to {goals_short}.”
- Never invent other apps, courses, quizzes, or assignments not listed in goals.
- Never meta phrases like “one short sentence”. Never copy the recent-lines list.
{break_rule}
- Do NOT reuse or paraphrase these recent lines: {avoid}
- Never dump URLs or tech jargon. Vary wording every time.

kind: {kind}
intensity: {intensity}
distraction: {example_distraction}
context: {detail_short}
mission goals: {goals_short}"#
    );

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

    let Ok(res) = api::coach_authed_raw(
        cfg,
        reqwest::Method::POST,
        &url_api,
        Some(&body),
        Duration::from_secs(4),
    )
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
    let chosen = if coach_line_needs_rewrite(&line, kind, distraction, goals)
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
    if !coach_line_needs_rewrite(existing, kind, distraction, goals) {
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
        assert!(!coach_line_suggests_break(&line));
    }

    #[test]
    fn distraction_break_copy_rewritten_to_refocus() {
        let bad = "Kindly, take a break and focus on your coding project. Let's move forward together.";
        assert!(coach_line_suggests_break(bad));
        let sanitized = sanitize_coach_line(bad, "distracted", "instagram", "coding project");
        assert!(
            !coach_line_suggests_break(&sanitized),
            "still suggests break: {sanitized}"
        );
        assert!(
            sanitized.to_lowercase().contains("instagram")
                || sanitized.to_lowercase().contains("coding"),
            "expected refocus line naming distraction or goals: {sanitized}"
        );
    }

    #[test]
    fn stress_break_invite_copy_is_kept() {
        let stress = "Feeling tense — optional five-minute break?";
        assert!(coach_line_suggests_break(stress));
        let kept = sanitize_coach_line(stress, "suggest_break", "suggest_break", "coding project");
        assert_eq!(kept, stress);
        let stressed = sanitize_coach_line(stress, "stressed", "stressed", "coding project");
        assert_eq!(stressed, stress);
    }

    #[test]
    fn camera_presence_kinds_kept_verbatim_not_rewritten_as_distraction() {
        let goals = "work on coding project for Waypoint!";
        let cases: &[(&str, &str)] = &[
            (
                "left_desk",
                "You've stepped away. Come back to the work when you can.",
            ),
            (
                "left_desk",
                "Still away from the desk — return when you're ready.",
            ),
            (
                "left_desk_pause",
                "I'll stay quiet until you're back at the desk.",
            ),
            (
                "welcome_back",
                "Welcome back — good to see you. Let's pick the work back up.",
            ),
            (
                "camera_obstructed",
                "I can't see you clearly. Check the camera or lighting.",
            ),
            (
                "suggest_break",
                "Feeling tense — optional five-minute break?",
            ),
            (
                "stressed",
                "You seem tense — one slow breath, then back.",
            ),
            (
                "watching",
                "You're locked in. I'll check in if you drift.",
            ),
        ];
        for (kind, text) in cases {
            // deliver_ephemeral used to pass kind as distraction → "left_desk isn't coding…".
            let sanitized = sanitize_coach_line(text, kind, kind, goals);
            assert_eq!(
                sanitized, *text,
                "kind={kind} was rewritten into distraction copy: {sanitized}"
            );
            assert!(
                !sanitized.to_lowercase().contains("left_desk"),
                "tag leaked into spoken line for {kind}: {sanitized}"
            );
            assert!(
                !sanitized.contains("isn't") && !sanitized.contains("isn’t"),
                "distraction template leaked for {kind}: {sanitized}"
            );
        }
    }

    #[test]
    fn break_plus_named_distraction_rewritten_even_if_kind_is_stressed() {
        let bad = "Take a break from Instagram and finish your coding project.";
        let sanitized = sanitize_coach_line(bad, "stressed", "stressed", "coding project");
        assert!(
            !coach_line_suggests_break(&sanitized),
            "distraction+break must refocus: {sanitized}"
        );
        assert!(
            sanitized.to_lowercase().contains("instagram"),
            "expected Instagram refocus: {sanitized}"
        );
    }

    #[test]
    fn fallback_templates_never_suggest_break() {
        for nag_n in [1u32, 2, 3] {
            for line in fallback_templates("distracted", "discord", "finish coding lab", nag_n) {
                assert!(
                    !coach_line_suggests_break(&line),
                    "fallback suggested break: {line}"
                );
            }
        }
    }

    #[test]
    fn biomg_quiz_hallucination_sanitized_for_coding_goals() {
        let bad = "Close Instagram and finish your BIOMG quiz.";
        assert!(coach_line_off_mission(bad, "coding project"));
        let sanitized = sanitize_coach_line(bad, "distracted", "youtube", "coding project");
        let lower = sanitized.to_lowercase();
        assert!(
            !lower.contains("biomg") && !lower.contains("quiz") && !lower.contains("instagram"),
            "BIOMG/Instagram leaked: {sanitized}"
        );
        assert!(
            lower.contains("youtube") && lower.contains("coding"),
            "expected youtube + coding refocus: {sanitized}"
        );
    }

    #[test]
    fn prompt_leak_pipe_and_quick_check_rejected() {
        let leak = "Close Instagram and finish your BIOMG quiz. Quick check: instagram pulled you off work on coding project. One click back to the work. | Quick check: instagram p";
        assert!(is_placeholder_coach_line(leak));
        assert!(coach_line_has_prompt_leak(leak));
        let sanitized = sanitize_coach_line(leak, "distracted", "youtube", "coding project");
        assert!(!sanitized.contains('|'), "pipe survived: {sanitized}");
        assert!(
            !normalize_line(&sanitized).contains("quick check"),
            "Quick check crumb survived: {sanitized}"
        );
        assert!(
            !sanitized.to_lowercase().contains("instagram"),
            "Instagram leak survived: {sanitized}"
        );
        assert!(sanitized.to_lowercase().contains("youtube"));

        // deliver_ephemeral often passes kind for both args — must not revive Instagram.
        let via_kind_only = sanitize_coach_line(leak, "distracted", "distracted", "coding project");
        assert!(
            !via_kind_only.to_lowercase().contains("instagram"),
            "kind-only sanitize revived Instagram: {via_kind_only}"
        );
        assert!(
            !via_kind_only.contains('|') && !via_kind_only.to_lowercase().contains("biomg"),
            "kind-only sanitize kept leak crumbs: {via_kind_only}"
        );
    }

    #[test]
    fn youtube_evidence_rejects_instagram_prose() {
        let bad = "Instagram isn’t the goal — close it and get back to coding project.";
        assert!(coach_line_wrong_distraction(bad, "youtube"));
        let sanitized = sanitize_coach_line(bad, "distracted", "youtube", "coding project");
        assert!(
            sanitized.to_lowercase().contains("youtube"),
            "expected youtube label: {sanitized}"
        );
        assert!(
            !sanitized.to_lowercase().contains("instagram"),
            "Instagram prose kept for youtube evidence: {sanitized}"
        );
    }
}
