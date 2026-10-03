use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::AppConfig;

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

    async fn generate(&self, body: Value) -> Result<String, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );
        let res = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Gemini error {status}: {text}"));
        }
        let parsed: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let content = parsed["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| "Gemini returned no text".to_string())?
            .to_string();
        Ok(content)
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
        self.generate(body).await
    }

    pub async fn classify_modality(
        &self,
        goals: &str,
        screen_jpeg: Option<&[u8]>,
    ) -> Result<String, String> {
        let mut parts = vec![json!({
            "text": format!(
                "Classify how this study session will mostly be done.\nGoals:\n{goals}\n\nReply with ONLY one word: computer, paper, or mixed."
            )
        })];
        if let Some(bytes) = screen_jpeg {
            parts.push(json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": B64.encode(bytes)
                }
            }));
        }
        let body = json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": { "temperature": 0.1 }
        });
        let raw = self.generate(body).await?.to_lowercase();
        if raw.contains("paper") {
            Ok("paper".into())
        } else if raw.contains("mixed") {
            Ok("mixed".into())
        } else {
            Ok("computer".into())
        }
    }

    pub async fn analyze_session(
        &self,
        goals: &str,
        modality: &str,
        vitals_summary: &str,
        screen_jpeg: Option<&[u8]>,
        camera_jpeg: Option<&[u8]>,
    ) -> Result<CoachVisionResult, String> {
        let mut parts = vec![json!({
            "text": format!(
                r#"You are Waypoint, a calm study lock-in coach.
Primary job: watch the SCREEN and judge whether it matches the student's goals.
Session goals: {goals}
Expected modality: {modality}
Live wellness signals from Presage / fallback (informational only, not medical): {vitals_summary}

Rules:
- on_task=true only if the visible screen work clearly advances the goals (docs, IDE, problem set, slides, etc.).
- Social feeds, shopping, unrelated videos, messaging, or random browsing → on_task=false and set distraction.
- Phone in a webcam frame is a distraction; a calculator for math is usually OK.
- needs_help=true if they appear stuck on the same problem/error with no progress.
- stress_cue=true if Presage reports stress OR they look tense/overwhelmed; do not invent medical claims.
- coach_line: one short, kind, specific sentence about what you see on screen (or a brief stress reset). No lectures.

Reply ONLY valid JSON with keys:
on_task (bool), objects (string array), distraction (string|null),
needs_help (bool), stress_cue (bool), coach_line (short spoken-style sentence),
modality (computer|paper|mixed)."#
            )
        })];

        if let Some(bytes) = screen_jpeg {
            parts.push(json!({
                "text": "PRIMARY — current screen:"
            }));
            parts.push(json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": B64.encode(bytes)
                }
            }));
        } else {
            parts.push(json!({
                "text": "PRIMARY — screen frame unavailable this tick."
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
                "temperature": 0.3,
                "responseMimeType": "application/json"
            }
        });

        let raw = self.generate(body).await?;
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
