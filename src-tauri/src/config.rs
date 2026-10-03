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
        let data_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("waypoint");
        let _ = std::fs::create_dir_all(&data_dir);

        // Release-friendly lookup order:
        // 1) ~/.config/waypoint/.env (or macOS/Windows equivalent)
        // 2) .env beside the Waypoint executable
        // 3) current working directory / repo root (dev)
        let mut candidates = vec![data_dir.join(".env")];

        if let Ok(exe) = env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join(".env"));
            }
        }

        candidates.push(PathBuf::from(".env"));
        candidates.push(PathBuf::from("../.env"));
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            candidates.push(PathBuf::from(manifest_dir).join("../.env"));
        }

        for path in candidates {
            if path.as_os_str().is_empty() {
                continue;
            }
            if dotenvy::from_path(&path).is_ok() {
                tracing::info!("loaded env from {}", path.display());
                break;
            }
        }
        let _ = dotenvy::dotenv();

        // Seed an empty config-dir .env template the first time so users know where keys go.
        let template = data_dir.join(".env");
        if !template.exists() {
            let _ = std::fs::write(
                &template,
                "# Waypoint local secrets — never commit this file\n\
                 GEMINI_API_KEY=\n\
                 GEMINI_MODEL=gemini-flash-latest\n\
                 PRESAGE_API_KEY=\n\
                 GOOGLE_CLIENT_ID=\n\
                 GOOGLE_CLIENT_SECRET=\n",
            );
        }

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
