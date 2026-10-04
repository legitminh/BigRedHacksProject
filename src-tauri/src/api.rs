//! HTTP client for the Waypoint backend (auth, Google proxy, Gemini chat, memory).

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use crate::auth::{self, AuthTokens};
use crate::config::AppConfig;

/// Soft budget for study heads-up TTS — overlay already shown; local `say` is the fallback.
const HEADS_UP_TTS_TIMEOUT: Duration = Duration::from_millis(3_800);
/// Settings “Test speak” can wait longer for a cold Grok/xAI synthesis.
const TEST_SPEAK_TTS_TIMEOUT: Duration = Duration::from_secs(15);

fn http() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("HTTP client")
}

fn http_tts(timeout: Duration) -> Client {
    Client::builder()
        .timeout(timeout)
        .build()
        .expect("TTS HTTP client")
}

#[derive(Debug)]
pub struct HeadsUpAudio {
    pub bytes: Vec<u8>,
    pub extension: String,
}

/// Fetch Grok/xAI TTS audio for a study heads-up via the API proxy (`POST /v1/voice/tts`).
/// Auth: signed-in JWT preferred, else baked `coach_api_token`.
/// Voice id is omitted so the API applies `XAI_TTS_VOICE` (default `eve`) — same as Live.
pub async fn fetch_heads_up_tts(cfg: &AppConfig, text: &str) -> Result<HeadsUpAudio, String> {
    fetch_grok_tts(cfg, text, HEADS_UP_TTS_TIMEOUT, false).await
}

/// Same Grok proxy / voice as Live + heads-ups, with a longer cold-start budget for Settings “Test speak”.
pub async fn fetch_test_speak_tts(cfg: &AppConfig, text: &str) -> Result<HeadsUpAudio, String> {
    fetch_grok_tts(cfg, text, TEST_SPEAK_TTS_TIMEOUT, true).await
}

async fn fetch_grok_tts(
    cfg: &AppConfig,
    text: &str,
    timeout: Duration,
    extended_wait: bool,
) -> Result<HeadsUpAudio, String> {
    let snippet: String = text.trim().chars().take(160).collect();
    if snippet.is_empty() {
        return Err("empty heads-up text".into());
    }
    let auth = cfg
        .coach_auth_header()
        .ok_or_else(|| "Sign in with Google to use Grok voice.".to_string())?;
    let url = format!("{}/v1/voice/tts", cfg.api_base().trim_end_matches('/'));
    // No voice_id → API `XAI_TTS_VOICE` (default eve), same as Live `connectGrokTts`.
    let body = if extended_wait {
        json!({ "text": snippet, "language": "en", "extended_wait": true })
    } else {
        json!({ "text": snippet, "language": "en" })
    };
    let res = http_tts(timeout)
        .post(url)
        .header(auth.0, auth.1)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(read_error(res).await);
    }
    let content_type = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let bytes = res.bytes().await.map_err(|e| e.to_string())?.to_vec();
    if bytes.len() < 32 {
        return Err("empty TTS audio".into());
    }
    let extension = if content_type.contains("wav") {
        "wav"
    } else if content_type.contains("ogg") {
        "ogg"
    } else {
        "mp3"
    }
    .to_string();
    Ok(HeadsUpAudio { bytes, extension })
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorInner>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorInner {
    message: Option<String>,
    code: Option<String>,
}

async fn read_error(res: reqwest::Response) -> String {
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if let Ok(body) = serde_json::from_str::<ApiErrorBody>(&text) {
        if let Some(err) = body.error {
            let code = err.code.filter(|c| !c.is_empty());
            if let Some(msg) = err.message.filter(|m| !m.is_empty()) {
                return match code {
                    Some(c) => format!("{msg} ({c})"),
                    None => msg,
                };
            }
            if let Some(c) = code {
                return format!("API error ({status}: {c})");
            }
        }
    }
    if text.is_empty() {
        format!("API error ({status})")
    } else {
        format!("API error ({status}): {text}")
    }
}

pub async fn post_json<T: DeserializeOwned>(
    cfg: &AppConfig,
    path: &str,
    body: &Value,
    bearer: Option<&str>,
) -> Result<T, String> {
    let url = format!("{}{}", cfg.api_base().trim_end_matches('/'), path);
    let mut req = http().post(url).json(body);
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(read_error(res).await);
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn get_json<T: DeserializeOwned>(
    cfg: &AppConfig,
    path: &str,
    bearer: Option<&str>,
) -> Result<T, String> {
    let url = format!("{}{}", cfg.api_base().trim_end_matches('/'), path);
    let mut req = http().get(url);
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(read_error(res).await);
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn send_empty(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    bearer: Option<&str>,
    body: Option<&Value>,
) -> Result<(), String> {
    let url = format!("{}{}", cfg.api_base().trim_end_matches('/'), path);
    let mut req = http().request(method, url);
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    if let Some(b) = body {
        req = req.json(b);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(read_error(res).await);
    }
    Ok(())
}

/// Authenticated request with one refresh retry on 401.
pub async fn authed_json<T: DeserializeOwned>(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
) -> Result<T, String> {
    let mut tokens = auth::load_tokens(cfg).ok_or_else(|| "Sign in to Waypoint first.".to_string())?;
    match authed_once::<T>(cfg, method.clone(), path, body, &tokens.access_token).await {
        Ok(v) => Ok(v),
        Err(e) if e.contains("401") || e.to_lowercase().contains("sign in") || e.to_lowercase().contains("unauthorized") => {
            tokens = refresh_tokens(cfg, &tokens).await?;
            authed_once::<T>(cfg, method, path, body, &tokens.access_token).await
        }
        Err(e) => Err(e),
    }
}

async fn authed_once<T: DeserializeOwned>(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
    access: &str,
) -> Result<T, String> {
    let url = format!("{}{}", cfg.api_base().trim_end_matches('/'), path);
    let mut req = http().request(method, url).bearer_auth(access);
    if let Some(b) = body {
        req = req.json(b);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    if status.as_u16() == 401 {
        return Err("unauthorized (401)".into());
    }
    if !status.is_success() {
        return Err(read_error(res).await);
    }
    if status.as_u16() == 204 {
        // Caller should use authed_empty for 204; tolerate empty JSON.
        return serde_json::from_value(Value::Null).map_err(|e| e.to_string());
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn authed_empty(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
) -> Result<(), String> {
    let mut tokens = auth::load_tokens(cfg).ok_or_else(|| "Sign in to Waypoint first.".to_string())?;
    match send_empty(cfg, method.clone(), path, Some(&tokens.access_token), body).await {
        Ok(()) => Ok(()),
        Err(e) if e.contains("401") || e.to_lowercase().contains("unauthorized") => {
            tokens = refresh_tokens(cfg, &tokens).await?;
            send_empty(cfg, method, path, Some(&tokens.access_token), body).await
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

async fn refresh_tokens(cfg: &AppConfig, tokens: &AuthTokens) -> Result<AuthTokens, String> {
    #[derive(Serialize)]
    struct Body<'a> {
        refresh_token: &'a str,
    }
    let url = format!("{}/v1/auth/refresh", cfg.api_base().trim_end_matches('/'));
    let res = http()
        .post(url)
        .json(&Body {
            refresh_token: &tokens.refresh_token,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        auth::clear_tokens(cfg);
        return Err(read_error(res).await);
    }
    let body: RefreshResponse = res.json().await.map_err(|e| e.to_string())?;
    let next = AuthTokens {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + body.expires_in,
        user: tokens.user.clone(),
    };
    auth::save_tokens(cfg, &next)?;
    Ok(next)
}
