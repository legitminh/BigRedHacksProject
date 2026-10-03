//! HTTP client for the Waypoint backend (auth, Google proxy, Gemini chat, memory).

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::auth::{self, AuthTokens};
use crate::config::AppConfig;

fn http() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("HTTP client")
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
            if let Some(msg) = err.message {
                return msg;
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

pub async fn access_token(cfg: &AppConfig) -> Result<String, String> {
    let tokens = auth::load_tokens(cfg).ok_or_else(|| "Sign in to Waypoint first.".to_string())?;
    if tokens.expires_at > chrono::Utc::now().timestamp() + 30 {
        return Ok(tokens.access_token);
    }
    Ok(refresh_tokens(cfg, &tokens).await?.access_token)
}
