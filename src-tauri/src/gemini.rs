use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::AppConfig;

/// Space out all Gemini calls so lock-in + chat don't stampede the free-tier quota.
static LAST_CALL: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Clone, Copy)]
enum CallKind {
    Chat,
    Vision,
}

fn friendly_gemini_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("503") || lower.contains("unavailable") || lower.contains("high demand") {
        "Gemini is busy right now — still watching your screen, retrying shortly.".into()
    } else if lower.contains("429") || lower.contains("resource_exhausted") {
        "Gemini rate limit hit — pausing to stay under quota, then continuing.".into()
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "Gemini timed out — trying again on the next tick.".into()
    } else {
        let short: String = raw.chars().take(160).collect();
        format!("Screen coach hiccup: {short}")
    }
}

fn is_rate_limited(raw: &str) -> bool {
    let lower = raw.to_lowercase();
    lower.contains("429")
        || lower.contains("resource_exhausted")
        || lower.contains("rate limit")
}

async fn wait_for_quota_slot(min_gap: Duration) {
    let wait_for = {
        let guard = LAST_CALL.lock().unwrap_or_else(|e| e.into_inner());
        guard.and_then(|last| {
            let elapsed = last.elapsed();
            if elapsed < min_gap {
                Some(min_gap - elapsed)
            } else {
                None
            }
        })
    };
    if let Some(delay) = wait_for {
        tokio::time::sleep(delay).await;
    }
}

fn mark_call_finished() {
    if let Ok(mut guard) = LAST_CALL.lock() {
        *guard = Some(Instant::now());
    }
}

fn retry_delay_from_error(raw: &str, fallback_ms: u64) -> u64 {
    if let Some(cap) = regex_retry_delay_ms(raw) {
        return cap.clamp(5_000, 120_000);
    }
    fallback_ms
}

fn regex_retry_delay_ms(raw: &str) -> Option<u64> {
    // Gemini often returns: "retryDelay": "24s"
    let key = "\"retryDelay\"";
    let idx = raw.find(key)?;
    let after = &raw[idx + key.len()..];
    let start = after.find('"')? + 1;
    let rest = &after[start..];
    let end = rest.find('"')?;
    let token = &rest[..end];
    let secs = token.trim_end_matches('s').parse::<f64>().ok()?;
    Some((secs * 1000.0) as u64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachVisionResult {
    pub on_task: bool,
    pub objects: Vec<String>,
    pub distraction: Option<String>,
    pub needs_help: bool,
    pub stress_cue: bool,
    pub coach_line: String,
    pub modality: Option<String>,
}

pub struct GeminiClient {
    http: Client,
    api_key: String,
    model: String,
}

impl GeminiClient {
    pub fn from_config(cfg: &AppConfig) -> Result<Self, String> {
        let api_key = cfg
            .gemini_api_key
            .clone()
            .ok_or_else(|| "GEMINI_API_KEY is not set".to_string())?;
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build().map_err(|e| e.to_string())?,
            api_key,
            model: cfg.gemini_model.clone(),
        })
    }

    async fn generate(&self, body: Value, kind: CallKind) -> Result<String, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );
        // Vision is frequent during lock-in — keep a wider gap and fewer retries.
        let min_gap = match kind {
            CallKind::Chat => Duration::from_secs(2),
            CallKind::Vision => Duration::from_secs(20),
        };
        let max_attempts = match kind {
            CallKind::Chat => 4usize,
            CallKind::Vision => 2usize,
        };

        let mut last_err = String::new();
        for attempt in 0..max_attempts {
            wait_for_quota_slot(min_gap).await;
            let res = self
                .http
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-goog-api-key", &self.api_key)
                .json(&body)
                .send()
                .await;
            mark_call_finished();

            let res = match res {
                Ok(r) => r,
                Err(e) => {
                    last_err = e.to_string();
                    if attempt + 1 == max_attempts {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }
            };
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            if status.is_success() {
                let parsed: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                let content = parsed["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .ok_or_else(|| "Gemini returned no text".to_string())?
                    .to_string();
                return Ok(content);
            }
            last_err = format!("Gemini error {status}: {text}");
            let rate_limited = status.as_u16() == 429 || is_rate_limited(&text);
            let temporary = rate_limited
                || status.is_server_error()
                || text.contains("UNAVAILABLE")
                || text.contains("high demand");
            if !temporary || attempt + 1 == max_attempts {
                break;
            }
            // Respect server-suggested delay; default longer on 429 so we don't dig deeper.
            let fallback = if rate_limited { 45_000 } else { 8_000 };
            let wait_ms = retry_delay_from_error(&text, fallback);
            tracing::warn!(
                "gemini {} retry in {}ms (attempt {}/{}): {}",
                match kind {
                    CallKind::Chat => "chat",
                    CallKind::Vision => "vision",
                },
                wait_ms,
                attempt + 1,
                max_attempts,
                friendly_gemini_error(&last_err)
            );
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }
        Err(friendly_gemini_error(&last_err))
    }

    pub async fn chat(
        &self,
        system: &str,
        history: &[ChatMessage],
        user_message: &str,
    ) -> Result<String, String> {
        let mut contents = Vec::new();
        for msg in history {
            let role = if msg.role == "assistant" {
                "model"
            } else {
                "user"
            };
            contents.push(json!({
                "role": role,
                "parts": [{ "text": msg.content }]
            }));
        }
        contents.push(json!({
            "role": "user",
            "parts": [{ "text": user_message }]
        }));

        let body = json!({
            "systemInstruction": {
                "parts": [{ "text": system }]
            },
            "contents": contents,
            "generationConfig": {
                "temperature": 0.6
            }
        });
        self.generate(body, CallKind::Chat).await
    }

    /// Local heuristic — avoids an extra Gemini call at lock-in start.
    pub fn infer_modality(goals: &str) -> String {
        let g = goals.to_lowercase();
        if g.contains("paper")
            || g.contains("notebook")
            || g.contains("handwrit")
            || g.contains("textbook")
        {
            "paper".into()
        } else if g.contains("mixed") || (g.contains("paper") && g.contains("laptop")) {
            "mixed".into()
        } else {
            "computer".into()
        }
    }

    pub async fn analyze_session(
        &self,
        goals: &str,
        modality: &str,
        vitals_summary: &str,
        screen_jpeg: Option<&[u8]>,
        camera_jpeg: Option<&[u8]>,
        desktop_hints: &str,
    ) -> Result<CoachVisionResult, String> {
        let mut parts = vec![json!({
            "text": format!(
                r#"You are Waypoint, a strict study lock-in coach.
You are given a FULL DESKTOP screenshot (all monitors may be stitched side-by-side).
Scan the ENTIRE image — not just the focused window. Look for side panels, PiP players,
second monitors, browser tabs, dock previews, and anything else visible.

Session goals: {goals}
Expected modality: {modality}
Wellness (informational only): {vitals_summary}
Extra OS hints (may be incomplete; trust the screenshot first):
{desktop_hints}

GOAL-SCOPE RULES:
- on_task=true ONLY if nearly everything visible is advancing the goals (IDE, docs, slides, coursework, relevant research).
- If ANY clearly off-task content is visible anywhere on the desktop, on_task=false.
  Off-task includes: YouTube (any player/tab/PiP), Instagram/TikTok/X/Reddit/Discord,
  Messages/iMessage/WhatsApp, email inboxes (Gmail/Outlook/Mail), online shopping
  (Amazon/eBay/etc), Netflix/Twitch, random social feeds, shopping storefronts.
- set distraction to a short label: youtube, email, shopping, texting, instagram, etc.
- Vague goals still require real productive work — entertainment/email/shopping are never "the work" unless goals explicitly say so.
- Waypoint UI alone is not on-task.
- needs_help=true only for stuck coursework/errors.
- stress_cue only with clear stress / provided wellness stress — no medical claims.
- coach_line: one short direct sentence naming what you see that's off-task (or affirming focus).

Reply ONLY valid JSON with keys:
on_task (bool), objects (string array of visible apps/sites), distraction (string|null),
needs_help (bool), stress_cue (bool), coach_line (short sentence),
modality (computer|paper|mixed)."#
            )
        })];

        if let Some(bytes) = screen_jpeg {
            parts.push(json!({
                "text": "FULL DESKTOP screenshot — inspect every region:"
            }));
            parts.push(json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": B64.encode(bytes)
                }
            }));
        } else {
            parts.push(json!({
                "text": "FULL DESKTOP screenshot unavailable this tick."
            }));
        }

        if let Some(bytes) = camera_jpeg {
            parts.push(json!({
                "text": "OPTIONAL webcam context (face / posture only):"
            }));
            parts.push(json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": B64.encode(bytes)
                }
            }));
        }

        let body = json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": {
                "temperature": 0.2,
                "responseMimeType": "application/json"
            }
        });

        let raw = self.generate(body, CallKind::Vision).await?;
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        serde_json::from_str::<CoachVisionResult>(cleaned).map_err(|e| {
            format!("Failed to parse coach JSON: {e}; raw={raw}")
        })
    }
}

pub fn error_looks_rate_limited(message: &str) -> bool {
    is_rate_limited(message)
}
