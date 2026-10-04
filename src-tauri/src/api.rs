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
/// Default authenticated JSON timeout (status, Google, short RPCs).
const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(60);
/// Gemini / companion chat — BE `GEMINI_CHAT_TIMEOUT_MS` is 90s; client must outlive it.
pub const CHAT_HTTP_TIMEOUT: Duration = Duration::from_secs(120);

fn http() -> Client {
    Client::builder()
        .timeout(DEFAULT_HTTP_TIMEOUT)
        .build()
        .expect("HTTP client")
}

/// Pick client timeout from path. Chat must exceed BE Gemini 90s; everything else stays snappy.
fn timeout_for_path(path: &str) -> Duration {
    if path.contains("/gemini/chat") || path.contains("/companion/chat") {
        CHAT_HTTP_TIMEOUT
    } else {
        DEFAULT_HTTP_TIMEOUT
    }
}

fn http_with_timeout(timeout: Duration) -> Client {
    Client::builder()
        .timeout(timeout)
        .build()
        .expect("HTTP client")
}

/// Structured API failure — keeps `Retry-After` for observe/chat backoff.
#[derive(Debug, Clone)]
pub struct ApiClientError {
    pub message: String,
    pub status: Option<u16>,
    pub code: Option<String>,
    pub retry_after_secs: Option<u64>,
}

impl ApiClientError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: None,
            code: None,
            retry_after_secs: None,
        }
    }

    pub fn is_rate_limited(&self) -> bool {
        self.status == Some(429)
            || self
                .code
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case("rate_limited"))
            || self.message.to_lowercase().contains("rate_limited")
            || self.message.contains("(429)")
    }
}

impl std::fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<ApiClientError> for String {
    fn from(value: ApiClientError) -> Self {
        value.message
    }
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
    if coach_authorization(cfg).is_none() {
        return Err("Sign in with Google to use Grok voice.".into());
    }
    let url = format!("{}/v1/voice/tts", cfg.api_base().trim_end_matches('/'));
    // No voice_id → API `XAI_TTS_VOICE` (default eve), same as Live `connectGrokTts`.
    let body = if extended_wait {
        json!({ "text": snippet, "language": "en", "extended_wait": true })
    } else {
        json!({ "text": snippet, "language": "en" })
    };
    let res = coach_authed_raw(cfg, reqwest::Method::POST, &url, Some(&body), timeout).await?;
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

/// Fresh coach Authorization header (disk JWT preferred, else baked coach token).
/// Prefer [`coach_authed_raw`] for HTTP — it refreshes once on 401.
pub fn coach_authorization(cfg: &AppConfig) -> Option<(String, String)> {
    if let Some(tokens) = auth::load_tokens(cfg) {
        if !tokens.access_token.is_empty() {
            return Some((
                "Authorization".into(),
                format!("Bearer {}", tokens.access_token),
            ));
        }
    }
    cfg.coach_api_token
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|t| ("Authorization".into(), format!("Bearer {t}")))
}

fn coach_uses_refreshable_jwt(cfg: &AppConfig) -> bool {
    auth::load_tokens(cfg)
        .map(|t| !t.access_token.is_empty() && !t.refresh_token.is_empty())
        .unwrap_or(false)
}

fn baked_coach_authorization(cfg: &AppConfig) -> Option<(String, String)> {
    cfg.coach_api_token
        .as_ref()
        .filter(|t| !t.is_empty())
        .map(|t| ("Authorization".into(), format!("Bearer {t}")))
}

/// Coach/Ollama-proxy HTTP with one JWT refresh retry on 401 (same policy as [`authed_json`]).
/// `url` is absolute (e.g. `{local_llm_base}/api/generate` or `{api_base}/v1/voice/tts`).
pub async fn coach_authed_raw(
    cfg: &AppConfig,
    method: reqwest::Method,
    url: &str,
    body: Option<&Value>,
    timeout: Duration,
) -> Result<reqwest::Response, String> {
    let auth = coach_authorization(cfg);
    let res = coach_once(method.clone(), url, body, timeout, auth.as_ref()).await?;
    if res.status().as_u16() != 401 || !coach_uses_refreshable_jwt(cfg) {
        return Ok(res);
    }
    let Some(tokens) = auth::load_tokens(cfg) else {
        return Ok(res);
    };
    drop(res);
    match refresh_tokens(cfg, &tokens).await {
        Ok(tokens) => {
            let auth = Some((
                "Authorization".into(),
                format!("Bearer {}", tokens.access_token),
            ));
            coach_once(method, url, body, timeout, auth.as_ref()).await
        }
        // Refresh failed — fall through to baked coach token so lock-in can keep judging.
        Err(_) => {
            let Some(auth) = baked_coach_authorization(cfg) else {
                return Err("Sign in again, or configure coach_api_token.".into());
            };
            coach_once(method, url, body, timeout, Some(&auth)).await
        }
    }
}

async fn coach_once(
    method: reqwest::Method,
    url: &str,
    body: Option<&Value>,
    timeout: Duration,
    auth: Option<&(String, String)>,
) -> Result<reqwest::Response, String> {
    let mut req = http_with_timeout(timeout).request(method, url);
    if let Some((name, value)) = auth {
        req = req.header(name, value);
    }
    if let Some(b) = body {
        req = req.json(b);
    }
    req.send().await.map_err(|e| e.to_string())
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

fn parse_retry_after(res: &reqwest::Response) -> Option<u64> {
    let raw = res.headers().get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().filter(|s| *s > 0 && *s < 3600)
}

async fn read_error_detailed(res: reqwest::Response) -> ApiClientError {
    let status = res.status().as_u16();
    let retry_after_secs = parse_retry_after(&res);
    let text = res.text().await.unwrap_or_default();
    let mut code: Option<String> = None;
    let message = if let Ok(body) = serde_json::from_str::<ApiErrorBody>(&text) {
        if let Some(err) = body.error {
            code = err.code.filter(|c| !c.is_empty());
            if let Some(msg) = err.message.filter(|m| !m.is_empty()) {
                match &code {
                    Some(c) => format!("{msg} ({c})"),
                    None => msg,
                }
            } else if let Some(c) = code.clone() {
                format!("API error ({status}: {c})")
            } else if text.is_empty() {
                format!("API error ({status})")
            } else {
                format!("API error ({status}): {text}")
            }
        } else if text.is_empty() {
            format!("API error ({status})")
        } else {
            format!("API error ({status}): {text}")
        }
    } else if text.is_empty() {
        format!("API error ({status})")
    } else {
        format!("API error ({status}): {text}")
    };
    ApiClientError {
        message,
        status: Some(status),
        code,
        retry_after_secs,
    }
}

async fn read_error(res: reqwest::Response) -> String {
    read_error_detailed(res).await.message
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
/// Chat paths (`/gemini/chat`, `/companion/chat`) use [`CHAT_HTTP_TIMEOUT`] (120s);
/// other routes keep [`DEFAULT_HTTP_TIMEOUT`] (60s). Lock-in judge/OCR callers do not
/// go through this helper — leave their short budgets alone.
pub async fn authed_json<T: DeserializeOwned>(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
) -> Result<T, String> {
    authed_json_timeout(cfg, method, path, body, timeout_for_path(path))
        .await
        .map_err(|e| e.message)
}

/// Like `authed_json` but with a custom timeout and structured errors (`Retry-After`).
pub async fn authed_json_timeout<T: DeserializeOwned>(
    cfg: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
    timeout: Duration,
) -> Result<T, ApiClientError> {
    let mut tokens = auth::load_tokens(cfg).ok_or_else(|| ApiClientError::msg("Sign in to Waypoint first."))?;
    match authed_once::<T>(cfg, method.clone(), path, body, &tokens.access_token, timeout).await {
        Ok(v) => Ok(v),
        Err(e)
            if e.status == Some(401)
                || e.message.contains("401")
                || e.message.to_lowercase().contains("sign in")
                || e.message.to_lowercase().contains("unauthorized") =>
        {
            tokens = refresh_tokens(cfg, &tokens)
                .await
                .map_err(ApiClientError::msg)?;
            authed_once::<T>(cfg, method, path, body, &tokens.access_token, timeout).await
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
    timeout: Duration,
) -> Result<T, ApiClientError> {
    let url = format!("{}{}", cfg.api_base().trim_end_matches('/'), path);
    let mut req = http_with_timeout(timeout)
        .request(method, url)
        .bearer_auth(access);
    if let Some(b) = body {
        req = req.json(b);
    }
    let res = req
        .send()
        .await
        .map_err(|e| ApiClientError::msg(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 401 {
        return Err(ApiClientError {
            message: "unauthorized (401)".into(),
            status: Some(401),
            code: Some("unauthorized".into()),
            retry_after_secs: None,
        });
    }
    if !status.is_success() {
        return Err(read_error_detailed(res).await);
    }
    if status.as_u16() == 204 {
        // Caller should use authed_empty for 204; tolerate empty JSON.
        return serde_json::from_value(Value::Null).map_err(|e| ApiClientError::msg(e.to_string()));
    }
    res.json()
        .await
        .map_err(|e| ApiClientError::msg(e.to_string()))
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

/// Serializes concurrent 401→refresh so rotated refresh tokens are not replayed
/// (`refresh_reuse` would revoke the session family and force logout).
static REFRESH_SINGLEFLIGHT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn refresh_tokens(cfg: &AppConfig, tokens: &AuthTokens) -> Result<AuthTokens, String> {
    let _guard = REFRESH_SINGLEFLIGHT.lock().await;

    // Another waiter may have already rotated while we queued on the lock.
    if let Some(current) = auth::load_tokens(cfg) {
        if current.refresh_token != tokens.refresh_token
            || current.access_token != tokens.access_token
        {
            return Ok(current);
        }
    } else {
        return Err("Session expired. Sign in again.".into());
    }

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
        // Another path may have rotated while the request was in flight — prefer disk.
        if let Some(current) = auth::load_tokens(cfg) {
            if current.refresh_token != tokens.refresh_token
                || current.access_token != tokens.access_token
            {
                return Ok(current);
            }
        }
        let status = res.status().as_u16();
        let err = read_error(res).await;
        let hard_revoke = status == 401
            && (err.contains("refresh_reuse")
                || err.contains("invalid_refresh")
                || err.to_lowercase().contains("refresh"));
        // Transient failures must not wipe the session; only hard refresh denials do.
        if hard_revoke {
            auth::clear_tokens(cfg);
        }
        return Err(err);
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
