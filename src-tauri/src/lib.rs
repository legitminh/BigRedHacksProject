mod capture;
mod coach;
mod config;
mod gemini;
mod google;
mod presage;
mod session;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::State;

use config::AppConfig;
use gemini::{ChatMessage, GeminiClient};
use google::GoogleContext;
use session::{LockInSession, SessionSummary};

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub session: Mutex<Option<LockInSession>>,
    pub chat_history: Mutex<Vec<ChatMessage>>,
    pub coach_stop: Mutex<Option<Arc<AtomicBool>>>,
}

#[derive(Serialize)]
struct StatusPayload {
    google_connected: bool,
    gemini_ready: bool,
    presage_ready: bool,
    session: Option<LockInSession>,
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> StatusPayload {
    let cfg = state.config.lock().clone();
    StatusPayload {
        google_connected: google::oauth::is_connected(&cfg),
        gemini_ready: cfg.gemini_api_key.is_some(),
        presage_ready: cfg.presage_api_key.is_some(),
        session: state.session.lock().clone(),
    }
}

#[tauri::command]
async fn connect_google(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.config.lock().clone();
    google::oauth::connect_google(&cfg).await
}

#[tauri::command]
fn disconnect_google(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.config.lock().clone();
    google::oauth::clear_tokens(&cfg);
    Ok(())
}

#[tauri::command]
async fn get_google_context(state: State<'_, AppState>) -> Result<GoogleContext, String> {
    let cfg = state.config.lock().clone();
    if !google::oauth::is_connected(&cfg) {
        return Ok(GoogleContext {
            connected: false,
            calendar_summary: "Google not connected.".into(),
            drive_summary: String::new(),
        });
    }
    let calendar_summary = google::calendar::upcoming_events_summary(&cfg, 8)
        .await
        .unwrap_or_else(|e| format!("Calendar unavailable: {e}"));
    let drive_summary = google::drive::recent_files_summary(&cfg, 6)
        .await
        .unwrap_or_else(|e| format!("Drive unavailable: {e}"));
    Ok(GoogleContext {
        connected: true,
        calendar_summary,
        drive_summary,
    })
}

#[tauri::command]
async fn chat_send(state: State<'_, AppState>, message: String) -> Result<ChatMessage, String> {
    let cfg = state.config.lock().clone();
    let client = GeminiClient::from_config(&cfg)?;

    let mut context_bits = Vec::new();
    if google::oauth::is_connected(&cfg) {
        if let Ok(cal) = google::calendar::upcoming_events_summary(&cfg, 8).await {
            context_bits.push(cal);
        }
        if let Ok(drive) = google::drive::recent_files_summary(&cfg, 5).await {
            context_bits.push(drive);
        }
        let q = message.trim();
        if q.len() > 3 && q.len() < 80 {
            if let Ok(search) = google::drive::search_files(&cfg, q, 5).await {
                context_bits.push(search);
            }
        }
    }

    let system = format!(
        "You are Waypoint, a school navigation coach for stressed students.\n\
         Help with priorities, deadlines, study plans, and clarifying what to do next.\n\
         Be concrete and calm. Navigation theme: help them find the next waypoint.\n\
         Do not invent calendar/drive facts — use only the context provided.\n\n\
         CONTEXT:\n{}",
        if context_bits.is_empty() {
            "No Google context connected yet.".into()
        } else {
            context_bits.join("\n\n")
        }
    );

    let history = state.chat_history.lock().clone();
    let reply = client.chat(&system, &history, &message).await?;

    let user = ChatMessage {
        role: "user".into(),
        content: message,
    };
    let assistant = ChatMessage {
        role: "assistant".into(),
        content: reply,
    };
    {
        let mut hist = state.chat_history.lock();
        hist.push(user);
        hist.push(assistant.clone());
        if hist.len() > 40 {
            let drain = hist.len() - 40;
            hist.drain(0..drain);
        }
    }
    Ok(assistant)
}

#[tauri::command]
fn clear_chat(state: State<'_, AppState>) {
    state.chat_history.lock().clear();
}

#[tauri::command]
async fn start_lock_in(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    goals: String,
    duration_mins: u64,
) -> Result<LockInSession, String> {
    if goals.trim().is_empty() {
        return Err("Describe what you want to lock in on.".into());
    }
    coach::stop_coach(&app);

    let cfg = state.config.lock().clone();
    let screen_jpeg = tokio::task::spawn_blocking(capture::screen::grab_primary_jpeg)
        .await
        .ok()
        .and_then(|r| r.ok());

    let modality = match GeminiClient::from_config(&cfg) {
        Ok(client) => client
            .classify_modality(&goals, screen_jpeg.as_deref())
            .await
            .unwrap_or_else(|_| "computer".into()),
        Err(_) => "computer".into(),
    };

    let session = LockInSession::start(goals, duration_mins.max(1), modality);
    let id = session.id.clone();
    *state.session.lock() = Some(session.clone());

    coach::spawn_coach_loop(app, id);
    Ok(session)
}

#[tauri::command]
async fn stop_lock_in(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<SessionSummary>, String> {
    coach::stop_coach(&app);
    let summary = state.session.lock().as_ref().map(|s| s.summarize());
    Ok(summary)
}

#[tauri::command]
fn get_session(state: State<'_, AppState>) -> Option<LockInSession> {
    state.session.lock().clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("waypoint=info,info")
        .try_init();

    let config = AppConfig::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: Mutex::new(config),
            session: Mutex::new(None),
            chat_history: Mutex::new(Vec::new()),
            coach_stop: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            connect_google,
            disconnect_google,
            get_google_context,
            chat_send,
            clear_chat,
            start_lock_in,
            stop_lock_in,
            get_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running Waypoint");
}
