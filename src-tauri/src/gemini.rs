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

/// User-facing copy only — never echo raw HTTP/JSON bodies.
fn friendly_gemini_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("503") || lower.contains("unavailable") || lower.contains("high demand") {
        "Cloud coach is busy right now. Trying again shortly.".into()
    } else if lower.contains("exceeded your current quota")
        || lower.contains("quota exceeded")
        || lower.contains("quota_metric")
        || lower.contains("free_tier")
    {
        "Cloud coach hit today’s free limit. Local watching continues — try Copilot again later.".into()
    } else if lower.contains("429") || lower.contains("resource_exhausted") {
        "Cloud coach is rate-limited. Pausing briefly, then continuing.".into()
    } else if lower.contains("api_key_invalid")
        || lower.contains("api key not valid")
        || lower.contains("api_key_invalid")
        || (lower.contains("api key") && (lower.contains("invalid") || lower.contains("permission")))
        || lower.contains("consumer_invalid")
    {
        "Cloud coach couldn’t authenticate with the Waypoint API. Sign in again.".into()
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "Cloud coach timed out. Trying again shortly.".into()
    } else if lower.contains("not set") || lower.contains("missing") {
        "Cloud coach isn’t available — check the Waypoint API server.".into()
    } else {
        "Cloud coach hit a snag. Please try again in a moment.".into()
    }
}

/// Live probe for Settings — confirms signed-in JWT can reach the API (Gemini stays server-side).
pub async fn probe_status_via_api(cfg: &AppConfig) -> (bool, String) {
    if crate::auth::load_tokens(cfg).is_none() {
        return (false, "Sign in with Google — cloud coach runs on the Waypoint API.".into());
    }
    match crate::api::authed_json::<serde_json::Value>(
        cfg,
        reqwest::Method::GET,
        "/v1/me",
        None,
    )
    .await
    {
        Ok(_) => (
            true,
            "Cloud coach ready via Waypoint API (Gemini key stays on the server).".into(),
        ),
        Err(e) => (false, format!("Couldn’t reach Waypoint API: {e}")),
    }
}

/// Legacy direct-Gemini probe (unused in shipped builds — keys must not live in the app).
#[allow(dead_code)]
pub async fn probe_status(cfg: &AppConfig) -> (bool, String) {
    let Some(api_key) = cfg.gemini_api_key.as_ref().filter(|k| !k.is_empty()) else {
        return (false, "Cloud coach isn’t configured in this build.".into());
    };
    let client = match Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (false, "Couldn’t reach cloud coach right now.".into()),
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        cfg.gemini_model
    );
    let body = json!({
        "contents": [{ "role": "user", "parts": [{ "text": "ping" }] }],
        "generationConfig": { "maxOutputTokens": 1, "temperature": 0 }
    });
    let res = client
        .post(&url)
        .header("X-goog-api-key", api_key)
        .json(&body)
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => (
            true,
            "Cloud vision + chat ready.".into(),
        ),
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            (
                false,
                friendly_gemini_error(&format!("Gemini error {status}: {text}")),
            )
        }
        Err(_) => (false, "Couldn’t reach cloud coach right now.".into()),
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

/// Copilot lock-in suggestion attached to an assistant `ChatMessage`.
/// Frontend: Accept → `start_lock_in(goals, duration_mins)`; Decline → ignore.
/// JSON shape (field names are a shared contract with the web UI — keep in sync):
/// `{ goals, duration_mins, reason, proposed_start, calendar_checked, calendar_clear, conflict_summary }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySessionSuggestion {
    pub goals: String,
    pub duration_mins: u64,
    pub reason: String,
    /// ISO8601 / RFC3339 when the session would start (usually "now")
    pub proposed_start: String,
    pub calendar_checked: bool,
    /// true when clear to start: Google connected + no overlap, OR Google not connected
    pub calendar_clear: bool,
    pub conflict_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Optional study-session CTA; omitted on older messages / when not suggested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study_suggestion: Option<StudySessionSuggestion>,
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
    /// Direct Gemini calls are disabled for shipped builds (keys must not live in the app).
    /// Prefer `POST /v1/gemini/chat` via the Waypoint API + user JWT.
    #[allow(dead_code)]
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
        // Vision is spaced for free-tier RPD (~250/day). Text context/chat stay snappy.
        let min_gap = match kind {
            CallKind::Chat => Duration::from_secs(2),
            CallKind::Vision => Duration::from_secs(45),
        };
        let max_attempts = match kind {
            CallKind::Chat => 2usize,
            CallKind::Vision => 1usize, // don't multiply failed vision into the daily budget
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
            let fallback = if rate_limited { 90_000 } else { 8_000 };
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

    /// Text-only judgment (YouTube title/page excerpt). Much cheaper/faster than screenshots.
    pub async fn judge_focus_context(
        &self,
        goals: &str,
        kind: &str,
        app: &str,
        title: &str,
        url: &str,
        page_text: &str,
    ) -> Result<CoachVisionResult, String> {
        let excerpt = page_text.chars().take(1800).collect::<String>();
        let body = json!({
            "contents": [{
                "role": "user",
                "parts": [{
                    "text": format!(
                        r#"You are Waypoint, a strict but fair study lock-in coach.
Decide if this focused app/page advances the session goals. Use ONLY the text context (no image).

Session goals: {goals}
Signal kind: {kind}
App: {app}
Window title: {title}
URL: {url}
Page text / metadata (may be truncated):
{excerpt}

RULES:
- YouTube / video: on_task=true ONLY if the video clearly helps the goals (lecture, tutorial,
  course content, exam review, topic explanation matching the goals). Entertainment, music,
  gaming, vlogs, reactions, random feeds → on_task=false, distraction="youtube".
- Instagram/TikTok/Discord/shopping/email/texting are never on-task unless goals explicitly require them.
- If unclear and kind is youtube, lean on_task=false (entertainment is the default risk).
- coach_line: one short direct sentence (name the video/topic if off-task, or affirm if on-task).
- objects: short labels of what they opened.
- needs_help=false, stress_cue=false unless text clearly says otherwise.

Reply ONLY valid JSON with keys:
on_task (bool), objects (string array), distraction (string|null),
needs_help (bool), stress_cue (bool), coach_line (short sentence),
modality (computer|paper|mixed)."#
                    )
                }]
            }],
            "generationConfig": {
                "temperature": 0.15,
                "responseMimeType": "application/json"
            }
        });

        let raw = self.generate(body, CallKind::Chat).await?;
        parse_coach_json(&raw)
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
                r#"You are Waypoint, a strict but fair study lock-in coach.
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
- Hard off-task anywhere → on_task=false: Instagram/TikTok/X/Reddit/Discord, Messages/WhatsApp,
  email inboxes, online shopping, Netflix/Twitch, random social feeds.
- YouTube / video players: INTERPRET CONTEXT. on_task=true if the video clearly supports the goals
  (lecture, tutorial, course, exam review). Music, gaming, vlogs, entertainment → on_task=false,
  distraction="youtube". If the title/topic is unreadable, treat as off-task.
- set distraction to a short label when off-task: youtube, email, shopping, texting, instagram, discord, etc.
- Vague goals still require real productive work — shopping/email/social are never "the work" unless goals say so.
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
        parse_coach_json(&raw)
    }
}

fn parse_coach_json(raw: &str) -> Result<CoachVisionResult, String> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str::<CoachVisionResult>(cleaned)
        .map_err(|e| format!("Failed to parse coach JSON: {e}; raw={raw}"))
}

pub fn error_looks_rate_limited(message: &str) -> bool {
    is_rate_limited(message)
}
