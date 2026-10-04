use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
struct EmbeddedSecrets {
    /// Ignored if present — Gemini keys must never ship in the .app.
    gemini_api_key: String,
    gemini_model: String,
    presage_api_key: String,
    /// Ignored if present — Google OAuth lives on the API server.
    google_client_id: String,
    google_client_secret: String,
    local_llm_base: String,
    local_llm_model: String,
    local_vision_model: String,
    coach_api_token: String,
    waypoint_api_base: String,
}

fn parse_toml_secrets(raw: &str) -> EmbeddedSecrets {
    let mut out = EmbeddedSecrets::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        match key {
            "gemini_api_key" => out.gemini_api_key = value,
            "gemini_model" => out.gemini_model = value,
            "presage_api_key" => out.presage_api_key = value,
            "google_client_id" => out.google_client_id = value,
            "google_client_secret" => out.google_client_secret = value,
            "local_llm_base" => out.local_llm_base = value,
            "local_llm_model" => out.local_llm_model = value,
            "local_vision_model" => out.local_vision_model = value,
            "coach_api_token" => out.coach_api_token = value,
            "waypoint_api_base" => {
                out.waypoint_api_base = value.clone();
                // Convenience: if only API root is set, coach lives at /v1/coach
                if out.local_llm_base.is_empty() {
                    out.local_llm_base = format!("{}/v1/coach", value.trim_end_matches('/'));
                }
            }
            _ => {}
        }
    }
    out
}

fn embedded() -> EmbeddedSecrets {
    // Compiled into the binary from src-tauri/secrets.toml (gitignored).
    parse_toml_secrets(include_str!("../secrets.toml"))
}

fn first_nonempty(values: &[Option<String>]) -> Option<String> {
    values
        .iter()
        .flatten()
        .find(|s| !s.is_empty())
        .cloned()
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub presage_api_key: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    /// Coach LLM base URL (Waypoint API `/v1/coach` proxy, or local Ollama for dev).
    pub local_llm_base: String,
    pub local_llm_model: String,
    /// Tiny multimodal model for rare screenshot checks (e.g. moondream).
    pub local_vision_model: String,
    /// Bearer token for `/v1/coach/*` (matches backend `COACH_API_TOKEN`).
    pub coach_api_token: Option<String>,
    /// Waypoint API root, e.g. http://127.0.0.1:8787
    pub waypoint_api_base: String,
    pub local_llm_enabled: bool,
    pub data_dir: PathBuf,
}

impl AppConfig {
    pub fn load() -> Self {
        let baked = embedded();
        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("waypoint");
        let _ = std::fs::create_dir_all(&data_dir);

        // Optional overrides for developers only (env / .env). End users rely on baked secrets.
        let mut candidates = vec![data_dir.join(".env"), PathBuf::from(".env"), PathBuf::from("../.env")];
        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.insert(0, dir.join(".env"));
            }
        }
        for path in candidates {
            let _ = dotenvy::from_path(&path);
        }
        let _ = dotenvy::dotenv();

        let gemini_model = env::var("GEMINI_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if baked.gemini_model.is_empty() {
                    "gemini-flash-latest".into()
                } else {
                    baked.gemini_model.clone()
                }
            });

        let local_llm_base = env::var("LOCAL_LLM_BASE")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| env::var("WAYPOINT_API_BASE").ok().filter(|s| !s.is_empty()).map(|b| {
                format!("{}/v1/coach", b.trim_end_matches('/'))
            }))
            .or_else(|| {
                if baked.local_llm_base.is_empty() {
                    None
                } else {
                    Some(baked.local_llm_base.clone())
                }
            })
            // Default: Waypoint API coach proxy (Ollama runs on the API host).
            .unwrap_or_else(|| "http://127.0.0.1:8787/v1/coach".into());

        let coach_api_token = first_nonempty(&[
            env::var("COACH_API_TOKEN").ok(),
            Some(baked.coach_api_token),
        ]);

        let local_llm_model = env::var("LOCAL_LLM_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                if baked.local_llm_model.is_empty() {
                    None
                } else {
                    Some(baked.local_llm_model.clone())
                }
            })
            .unwrap_or_else(|| "qwen2.5:0.5b".into());

        let local_vision_model = env::var("LOCAL_VISION_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                if baked.local_vision_model.is_empty() {
                    None
                } else {
                    Some(baked.local_vision_model.clone())
                }
            })
            .unwrap_or_else(|| "moondream".into());

        let local_llm_enabled = env::var("LOCAL_LLM_ENABLED")
            .ok()
            .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "off" | "no"))
            .unwrap_or(true);

        let waypoint_api_base = env::var("WAYPOINT_API_BASE")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                if baked.waypoint_api_base.is_empty() {
                    None
                } else {
                    Some(baked.waypoint_api_base.clone())
                }
            })
            .unwrap_or_else(|| "http://127.0.0.1:8787".into());

        // Never bake Gemini or Google OAuth into the binary — extractable from Waypoint.app.
        // Gemini + Google live on the Waypoint API. Optional env overrides are for local
        // legacy tooling only and are still not written into secrets.toml.
        Self {
            gemini_api_key: None,
            gemini_model,
            presage_api_key: first_nonempty(&[
                env::var("PRESAGE_API_KEY").ok(),
                Some(baked.presage_api_key),
            ]),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            local_llm_base,
            local_llm_model,
            local_vision_model,
            coach_api_token,
            waypoint_api_base,
            local_llm_enabled,
            data_dir,
        }
    }

    pub fn api_base(&self) -> &str {
        self.waypoint_api_base.trim_end_matches('/')
    }

    /// Sync Bearer header for the Waypoint coach proxy (JWT preferred, else baked token).
    ///
    /// Does **not** refresh on 401 — HTTP callers must use [`crate::api::coach_authed_raw`].
    pub fn coach_auth_header(&self) -> Option<(&str, String)> {
        crate::api::coach_authorization(self).map(|(_, value)| ("Authorization", value))
    }

    pub fn google_token_path(&self) -> PathBuf {
        self.data_dir.join("google_tokens.json")
    }

    /// Google Calendar/Drive connect is available when signed into Waypoint (server-side OAuth).
    pub fn google_oauth_ready(&self) -> bool {
        crate::auth::load_tokens(self).is_some()
    }
}
