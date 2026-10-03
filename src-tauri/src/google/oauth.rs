use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::Query;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken,
    Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use url::Url;

use crate::config::AppConfig;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
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
    fs::write(path, serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?)
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

/// Start localhost OAuth, open browser, persist tokens.
pub async fn connect_google(cfg: &AppConfig) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let redirect = format!("http://127.0.0.1:{}/oauth2/callback", addr.port());
    let oauth = client(cfg, &redirect)?;

    let (auth_url, csrf) = oauth
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly".into(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/drive.readonly".into(),
        ))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));
    let expected_state = csrf.secret().clone();

    let app = Router::new().route(
        "/oauth2/callback",
        get({
            let tx = tx.clone();
            move |Query(q): Query<CallbackQuery>| {
                let tx = tx.clone();
                let expected_state = expected_state.clone();
                async move {
                    let result = if let Some(err) = q.error {
                        Err(format!("OAuth error: {err}"))
                    } else if q.state.as_deref() != Some(expected_state.as_str()) {
                        Err("OAuth state mismatch".into())
                    } else if let Some(code) = q.code {
                        Ok(code)
                    } else {
                        Err("Missing OAuth code".into())
                    };

                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(result.clone());
                    }

                    match result {
                        Ok(_) => Html(
                            "<html><body style='font-family:sans-serif;padding:2rem'><h1>Waypoint connected</h1><p>You can close this tab and return to the app.</p></body></html>"
                                .to_string(),
                        ),
                        Err(e) => Html(format!(
                            "<html><body style='font-family:sans-serif;padding:2rem'><h1>Connection failed</h1><p>{e}</p></body></html>"
                        )),
                    }
                }
            }
        }),
    );

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    open::that(auth_url.as_str()).map_err(|e| format!("failed to open browser: {e}"))?;

    tokio::select! {
        _ = server => return Err("OAuth server stopped unexpectedly".into()),
        result = rx => {
            let code = result.map_err(|_| "OAuth cancelled".to_string())??;
            let token = oauth
                .exchange_code(AuthorizationCode::new(code))
                .request_async(oauth2::reqwest::async_http_client)
                .await
                .map_err(|e| format!("code exchange failed: {e}"))?;

            let stored = StoredTokens {
                access_token: token.access_token().secret().clone(),
                refresh_token: token.refresh_token().map(|t| t.secret().clone()),
                expires_at: token.expires_in().map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64),
            };
            save_tokens(cfg, &stored)?;
            Ok(())
        }
    }
}

#[allow(dead_code)]
pub fn auth_url_preview(cfg: &AppConfig) -> Result<Url, String> {
    let oauth = client(cfg, "http://127.0.0.1:17899/oauth2/callback")?;
    Ok(oauth
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly".into(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/drive.readonly".into(),
        ))
        .url()
        .0)
}
