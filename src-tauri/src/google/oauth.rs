//! Legacy local Google token store (pre–server OAuth). New sign-in uses the Waypoint API;
//! these helpers remain for clearing old tokens and rare local-calendar fallbacks.

use std::fs;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, RedirectUrl, RefreshToken, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

fn client(cfg: &AppConfig, redirect: &str) -> Result<BasicClient, String> {
    let client_id = cfg
        .google_client_id
        .clone()
        .ok_or_else(|| "GOOGLE_CLIENT_ID is not set".to_string())?;
    let client_secret = cfg
        .google_client_secret
        .clone()
        .ok_or_else(|| "GOOGLE_CLIENT_SECRET is not set".to_string())?;
    Ok(BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(AUTH_URL.to_string()).map_err(|e| e.to_string())?,
        Some(TokenUrl::new(TOKEN_URL.to_string()).map_err(|e| e.to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(redirect.to_string()).map_err(|e| e.to_string())?))
}

pub fn load_tokens(cfg: &AppConfig) -> Option<StoredTokens> {
    let path = cfg.google_token_path();
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_tokens(cfg: &AppConfig, tokens: &StoredTokens) -> Result<(), String> {
    let path = cfg.google_token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn clear_tokens(cfg: &AppConfig) {
    let _ = fs::remove_file(cfg.google_token_path());
}

pub async fn ensure_access_token(cfg: &AppConfig) -> Result<String, String> {
    let mut tokens = load_tokens(cfg).ok_or_else(|| "Google not connected".to_string())?;
    let now = chrono::Utc::now().timestamp();
    if let Some(exp) = tokens.expires_at {
        if exp > now + 60 {
            return Ok(tokens.access_token);
        }
    }
    let refresh = tokens
        .refresh_token
        .clone()
        .ok_or_else(|| "No refresh token; reconnect Google".to_string())?;

    // Redirect URI unused for refresh but required by client builder.
    let oauth = client(cfg, "http://127.0.0.1:17899/oauth2/callback")?;
    let token = oauth
        .exchange_refresh_token(&RefreshToken::new(refresh))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| format!("token refresh failed: {e}"))?;

    tokens.access_token = token.access_token().secret().clone();
    if let Some(rt) = token.refresh_token() {
        tokens.refresh_token = Some(rt.secret().clone());
    }
    if let Some(expires) = token.expires_in() {
        tokens.expires_at = Some(now + expires.as_secs() as i64);
    }
    save_tokens(cfg, &tokens)?;
    Ok(tokens.access_token)
}

pub fn is_connected(cfg: &AppConfig) -> bool {
    load_tokens(cfg).is_some()
}
