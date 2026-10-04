//! Lightweight local screen understanding: Apple Vision OCR + optional tiny Ollama VLM.
//! Designed to stay cheap on CPU/GPU — OCR is primary; vision model is rare backup.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde_json::{json, Value};

use crate::capture::{ocr, screen};
use crate::config::AppConfig;
use crate::gemini::CoachVisionResult;

fn with_coach_auth(req: reqwest::RequestBuilder, cfg: &AppConfig) -> reqwest::RequestBuilder {
    if let Some((name, value)) = cfg.coach_auth_header() {
        req.header(name, value)
    } else {
        req
    }
}

const OCR_MAX_CHARS: usize = 2200;
const VLM_PROBE_TTL_SECS: u64 = 60;
const MIN_VLM_CONFIDENCE: f32 = 0.55;

static VLM_PROBE: Mutex<Option<(Instant, bool, String)>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct ScreenReading {
    pub ocr_text: String,
    pub labels: Vec<&'static str>,
    #[allow(dead_code)]
    pub source: &'static str,
}

/// Capture a small focused/desktop frame and OCR it (fast Vision .fast).
pub fn read_screen_ocr() -> Result<ScreenReading, String> {
    let jpeg = screen::grab_ocr_jpeg()?;
    let text = ocr::ocr_jpeg_bytes(&jpeg)?;
    let clipped: String = text.chars().take(OCR_MAX_CHARS).collect();
    let labels = ocr::labels_in_text(&clipped);
    Ok(ScreenReading {
        ocr_text: clipped,
        labels,
        source: "vision-ocr",
    })
}

pub async fn vlm_available(cfg: &AppConfig) -> bool {
    if !cfg.local_llm_enabled || cfg.local_vision_model.is_empty() {
        return false;
    }
    {
        let guard = VLM_PROBE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, ok, _)) = guard.as_ref() {
            if at.elapsed() < Duration::from_secs(VLM_PROBE_TTL_SECS) {
                return *ok;
            }
        }
    }
    let (ok, detail) = probe_vlm(cfg).await;
    if let Ok(mut guard) = VLM_PROBE.lock() {
        *guard = Some((Instant::now(), ok, detail));
    }
    ok
}

pub async fn vlm_status_line(cfg: &AppConfig) -> String {
    if cfg.local_vision_model.is_empty() {
        return "Screen coach off".into();
    }
    let (ok, detail) = probe_vlm(cfg).await;
    if let Ok(mut guard) = VLM_PROBE.lock() {
        *guard = Some((Instant::now(), ok, detail.clone()));
    }
    if ok {
        format!("Screen coach ready · {}", cfg.local_vision_model)
    } else {
        format!("Screen coach unavailable · {detail}")
    }
}

async fn probe_vlm(cfg: &AppConfig) -> (bool, String) {
    let client = match Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(c) => c,
        Err(e) => return (false, e.to_string()),
    };
    let url = format!("{}/api/tags", cfg.local_llm_base.trim_end_matches('/'));
    let res = match with_coach_auth(client.get(url), cfg).send().await {
        Ok(r) => r,
        Err(_) => {
            return (
                false,
                "start Waypoint API on this Mac".into(),
            )
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
    let want = cfg.local_vision_model.to_lowercase();
    let want_base = want.split(':').next().unwrap_or(&want);
    let found = body["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["name"].as_str())
        .any(|name| {
            let n = name.to_lowercase();
            n == want || n.starts_with(&format!("{want_base}:")) || n.contains(want_base)
        });
    if found {
        (true, cfg.local_vision_model.clone())
    } else {
        (
            false,
            format!("need model {}", cfg.local_vision_model),
        )
    }
}

/// Tiny local vision model: is this screenshot on-task for the goals?
/// Screen only — never run on webcam frames (presence/phone is Presage / VIDEOINPUT).
/// Keep frames small — caller should pass OCR-sized JPEG.
pub async fn judge_frame(
    cfg: &AppConfig,
    goals: &str,
    jpeg: &[u8],
    ocr_hint: &str,
) -> Result<(CoachVisionResult, f32), String> {
    if !vlm_available(cfg).await {
        return Err("local VLM unavailable".into());
    }
    let hint: String = ocr_hint.chars().take(800).collect();
    let goal_hint: String = goals.chars().take(48).collect();
    let prompt = format!(
        r#"You are a study lock-in classifier. Look at the screenshot (and OCR hint).
Reply ONLY JSON: {{"on_task":true|false,"confidence":0.0-1.0,"distraction":null|"youtube"|"instagram"|"shopping"|"email"|"discord"|"phone"|"other","coach_line":"Leave that tab and get back to {goal_hint}."}}
Rules: on_task only if the visible content advances the goals. YouTube entertainment/music/gaming = false. Lectures matching goals = true.
Set distraction from the visible site/app (youtube.com/Shorts → "youtube", never "instagram"). Use "phone" only if a phone UI/screen is clearly visible.
coach_line must name the real distraction and reuse words from goals only — never invent other courses/quizzes (e.g. BIOMG/ENGL) not in goals. Never meta text like "short" or "one short sentence".
When off-task, tell them to leave the distraction and refocus — NEVER suggest taking a break (breaks are only for stress/tiredness).
goals: {goals}
ocr_hint: {hint}"#
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/api/generate", cfg.local_llm_base.trim_end_matches('/'));
    let body = json!({
        "model": cfg.local_vision_model,
        "prompt": prompt,
        "images": [B64.encode(jpeg)],
        "stream": false,
        "format": "json",
        "options": {
            "temperature": 0.1,
            "num_predict": 100
        }
    });
    let res = with_coach_auth(client.post(url), cfg)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("coach VLM request: {e}"))?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        if let Ok(mut guard) = VLM_PROBE.lock() {
            *guard = None;
        }
        return Err(format!("local VLM HTTP {status}: {text}"));
    }
    let wrapped: Value =
        serde_json::from_str(&text).map_err(|e| format!("local VLM envelope: {e}"))?;
    let raw = wrapped["response"]
        .as_str()
        .ok_or_else(|| "local VLM empty response".to_string())?;
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let slice = if let (Some(a), Some(b)) = (cleaned.find('{'), cleaned.rfind('}')) {
        &cleaned[a..=b]
    } else {
        cleaned
    };
    let parsed: Value =
        serde_json::from_str(slice).map_err(|e| format!("local VLM JSON: {e}; raw={raw}"))?;
    let on_task = parsed["on_task"].as_bool().unwrap_or(false);
    let confidence = parsed["confidence"].as_f64().unwrap_or(0.6) as f32;
    let distraction = parsed["distraction"].as_str().map(|s| s.to_string());
    let raw_line = parsed["coach_line"].as_str().unwrap_or("").trim();
    let d_label = distraction.as_deref().unwrap_or("off-task");
    let coach_line = crate::local_judge::sanitize_coach_line(
        raw_line,
        if on_task { "on_task" } else { "distracted" },
        d_label,
        goals,
    );
    let conf = confidence.clamp(0.0, 1.0);
    if conf < MIN_VLM_CONFIDENCE {
        return Err(format!("local VLM low confidence ({conf:.2})"));
    }
    Ok((
        CoachVisionResult {
            on_task,
            objects: vec!["screen".into()],
            distraction: if on_task { None } else { distraction },
            needs_help: false,
            stress_cue: false,
            coach_line,
            modality: Some("computer".into()),
        },
        conf,
    ))
}

/// Grab the same compact JPEG used for OCR (for rare VLM calls).
pub fn grab_judge_jpeg() -> Result<Vec<u8>, String> {
    screen::grab_ocr_jpeg()
}
