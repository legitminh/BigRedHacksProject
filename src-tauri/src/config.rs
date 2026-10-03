use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
struct EmbeddedSecrets {
    gemini_api_key: String,
    gemini_model: String,
    presage_api_key: String,
    google_client_id: String,
    google_client_secret: String,
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

        Self {
            gemini_api_key: first_nonempty(&[
                env::var("GEMINI_API_KEY").ok(),
                Some(baked.gemini_api_key),
            ]),
            gemini_model,
            presage_api_key: first_nonempty(&[
                env::var("PRESAGE_API_KEY").ok(),
                Some(baked.presage_api_key),
            ]),
            google_client_id: first_nonempty(&[
                env::var("GOOGLE_CLIENT_ID").ok(),
                Some(baked.google_client_id),
            ]),
            google_client_secret: first_nonempty(&[
                env::var("GOOGLE_CLIENT_SECRET").ok(),
                Some(baked.google_client_secret),
            ]),
            data_dir,
        }
    }

    pub fn google_token_path(&self) -> PathBuf {
        self.data_dir.join("google_tokens.json")
    }

    pub fn google_oauth_ready(&self) -> bool {
        self.google_client_id.is_some() && self.google_client_secret.is_some()
    }
}
