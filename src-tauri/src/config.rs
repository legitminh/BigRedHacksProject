use std::env;
use std::path::PathBuf;

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
        // Prefer repo-root .env; also accept cwd /.env for local runs.
        let candidates = [
            PathBuf::from(".env"),
            PathBuf::from("../.env"),
            env::var("CARGO_MANIFEST_DIR")
                .map(|d| PathBuf::from(d).join("../.env"))
                .unwrap_or_default(),
        ];
        for path in candidates {
            if path.as_os_str().is_empty() {
                continue;
            }
            if dotenvy::from_path(&path).is_ok() {
                break;
            }
        }
        let _ = dotenvy::dotenv();

        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("waypoint");
        let _ = std::fs::create_dir_all(&data_dir);

        Self {
            gemini_api_key: env::var("GEMINI_API_KEY").ok().filter(|s| !s.is_empty()),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-flash-latest".into()),
            presage_api_key: env::var("PRESAGE_API_KEY").ok().filter(|s| !s.is_empty()),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            data_dir,
        }
    }

    pub fn google_token_path(&self) -> PathBuf {
        self.data_dir.join("google_tokens.json")
    }
}
