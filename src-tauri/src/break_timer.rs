//! Pomodoro break timer — fullscreen blocking window + lock-in pause plumbing.
//!
//! Commands (invoke from main / break window):
//! - `start_break_timer` — `{ durationSecs?, reason? }` pause + fullscreen break
//! - `end_break_timer` — destroy break window, unpause mission (Resume)
//! - `suggest_break_timer` — `{ durationSecs?, reason? }` emit suggestion only
//! - `get_break_timer_status` — whether a break window is currently up
//!
//! Events:
//! - `break-timer-started` → break + main (`{ duration_secs, reason }`)
//! - `break-timer-finished` → emitted by break window FE when countdown hits 0
//! - `break-timer-suggested` → main (`{ duration_secs, reason }`)
//! - `break-timer-ended` → main after Resume / end

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::overlay;
use crate::session::LockInSession;
use crate::AppState;

pub const BREAK_LABEL: &str = "break-timer";
const DEFAULT_DURATION_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize)]
pub struct BreakStartedPayload {
    pub duration_secs: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreakSuggestedPayload {
    pub duration_secs: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreakTimerStatus {
    pub active: bool,
    pub duration_secs: u64,
    pub reason: String,
}

fn normalize_duration(secs: Option<u64>) -> u64 {
    match secs {
        Some(0) | None => DEFAULT_DURATION_SECS,
        Some(n) => n.min(60 * 60),
    }
}

fn normalize_reason(reason: Option<&str>, fallback: &str) -> String {
    match reason.map(str::trim).filter(|s| !s.is_empty()) {
        Some(r) => r.to_string(),
        None => fallback.to_string(),
    }
}

fn session_is_paused(state: &AppState) -> bool {
    state
        .session
        .lock()
        .as_ref()
        .map(|s| s.active && s.paused)
        .unwrap_or(false)
}

/// Apply lock-in pause/resume (shared with `set_lock_in_paused`).
pub(crate) fn apply_lock_in_paused(
    app: &AppHandle,
    state: &AppState,
    paused: bool,
) -> Result<LockInSession, String> {
    let snap = {
        let mut guard = state.session.lock();
        let session = guard
            .as_mut()
            .filter(|s| s.active)
            .ok_or_else(|| "No active mission to pause.".to_string())?;
        if session.paused == paused {
            return Ok(session.clone());
        }
        if paused {
            session.paused = true;
            *state.pause_started.lock() = Some(chrono::Utc::now());
            session.watching_note = "On a break — tap Resume when you’re ready.".into();
        } else {
            let started = state.pause_started.lock().take();
            session.finalize_open_pause(started);
            session.paused = false;
            session.watching_note = "Back on course — watching with you.".into();
        }
        session.clone()
    };
    let _ = app.emit("session-update", &snap);
    Ok(snap)
}

/// Main-window pause toggle. Resume while a break window is up must tear it down
/// (same exit path as the break screen Resume button).
pub(crate) fn set_lock_in_paused_with_break(
    app: &AppHandle,
    state: &AppState,
    paused: bool,
) -> Result<LockInSession, String> {
    if !paused && state.break_active.load(Ordering::SeqCst) {
        return end_break_inner(app, state)?
            .ok_or_else(|| "No active mission to resume.".to_string());
    }
    if paused && state.break_active.load(Ordering::SeqCst) {
        // Already inside the fullscreen break — don't nest another pause stamp.
        let guard = state.session.lock();
        return guard
            .as_ref()
            .filter(|s| s.active)
            .cloned()
            .ok_or_else(|| "No active mission to pause.".to_string());
    }
    apply_lock_in_paused(app, state, paused)
}

fn attach_close_guard(app: &AppHandle, window: &WebviewWindow) {
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let allowed = handle
                .try_state::<AppState>()
                .map(|s| s.break_allow_close.load(Ordering::SeqCst))
                .unwrap_or(false);
            if !allowed {
                api.prevent_close();
            }
        }
    });
}

fn size_to_primary(window: &WebviewWindow) -> Result<(), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("primary monitor: {e}"))?
        .ok_or_else(|| "No primary monitor found.".to_string())?;
    let size = monitor.size();
    let pos = monitor.position();
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: pos.x,
            y: pos.y,
        }))
        .map_err(|e| format!("break timer position: {e}"))?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: size.width,
            height: size.height,
        }))
        .map_err(|e| format!("break timer size: {e}"))?;
    Ok(())
}

fn ensure_break_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(BREAK_LABEL) {
        return Ok(existing);
    }

    let window = WebviewWindowBuilder::new(
        app,
        BREAK_LABEL,
        WebviewUrl::App("break-timer.html".into()),
    )
    .title("Waypoint • Break")
    .decorations(false)
    .resizable(false)
    .closable(false)
    .maximizable(false)
    .minimizable(false)
    .fullscreen(true)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .skip_taskbar(true)
    .focused(true)
    .visible(false)
    .build()
    .map_err(|e| format!("break timer window: {e}"))?;

    attach_close_guard(app, &window);
    Ok(window)
}

fn present_break_window(window: &WebviewWindow) -> Result<(), String> {
    if let Err(e) = size_to_primary(window) {
        tracing::warn!("break timer size_to_primary: {e}");
    }
    if let Err(e) = window.set_fullscreen(true) {
        tracing::warn!("break timer set_fullscreen: {e}");
    }
    if let Err(e) = window.set_always_on_top(true) {
        tracing::warn!("break timer set_always_on_top: {e}");
    }
    let _ = window.unminimize();
    window
        .show()
        .map_err(|e| format!("Couldn’t show the break timer window ({e})."))?;
    if let Err(e) = window.set_focus() {
        tracing::warn!("break timer set_focus: {e}");
    }
    // Re-assert stacking after show — some hosts drop always-on-top on first map.
    let _ = window.set_always_on_top(true);
    let _ = window.set_fullscreen(true);
    Ok(())
}

fn push_started(app: &AppHandle, window: &WebviewWindow, payload: &BreakStartedPayload) {
    let _ = app.emit_to(BREAK_LABEL, "break-timer-started", payload);
    if let Ok(json) = serde_json::to_string(payload) {
        let _ = window.eval(&format!(
            "window.__waypointBreakStart && window.__waypointBreakStart({json})"
        ));
    }
}

fn clear_break_flags(state: &AppState) {
    state.break_active.store(false, Ordering::SeqCst);
    state.break_allow_close.store(false, Ordering::SeqCst);
}

fn destroy_break_window(app: &AppHandle, state: &AppState) {
    state.break_allow_close.store(true, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window(BREAK_LABEL) {
        if let Err(e) = window.destroy() {
            tracing::warn!("break timer destroy: {e}");
            // Fallback: hide so it doesn't keep blocking the desktop.
            let _ = window.hide();
        }
    }
    clear_break_flags(state);
}

/// Tear down break window without unpausing (e.g. mission ended).
pub fn force_close_break_window(app: &AppHandle, state: &AppState) {
    let was_active = state.break_active.load(Ordering::SeqCst)
        || app.get_webview_window(BREAK_LABEL).is_some();
    destroy_break_window(app, state);
    if was_active {
        let _ = app.emit("break-timer-ended", ());
    }
}

fn end_break_inner(app: &AppHandle, state: &AppState) -> Result<Option<LockInSession>, String> {
    destroy_break_window(app, state);
    let _ = app.emit("break-timer-ended", ());

    // Unpause if a mission is still active; ignore "no session" so Resume still closes UI.
    match apply_lock_in_paused(app, state, false) {
        Ok(snap) => Ok(Some(snap)),
        Err(_) => Ok(None),
    }
}

fn rollback_failed_start(app: &AppHandle, state: &AppState, was_paused: bool) {
    destroy_break_window(app, state);
    if !was_paused {
        if let Err(e) = apply_lock_in_paused(app, state, false) {
            tracing::warn!("break timer rollback unpause: {e}");
        }
    }
}

#[tauri::command]
pub async fn start_break_timer(
    app: AppHandle,
    state: State<'_, AppState>,
    duration_secs: Option<u64>,
    reason: Option<String>,
) -> Result<BreakTimerStatus, String> {
    let duration_secs = normalize_duration(duration_secs);
    let reason = normalize_reason(reason.as_deref(), "user");

    // Require an active mission up front (clearer than failing mid-flight).
    {
        let guard = state.session.lock();
        if guard.as_ref().filter(|s| s.active).is_none() {
            return Err("No active mission to pause for a break.".into());
        }
    }

    // Orphaned flag with no window — clear before starting fresh.
    if state.break_active.load(Ordering::SeqCst) && app.get_webview_window(BREAK_LABEL).is_none() {
        tracing::warn!("break_active set without window — clearing orphaned flag");
        clear_break_flags(&state);
    }

    // Already breaking with a live window — re-present + refresh countdown (idempotent).
    if state.break_active.load(Ordering::SeqCst) {
        if let Some(window) = app.get_webview_window(BREAK_LABEL) {
            overlay::hide(&app);
            if let Err(e) = present_break_window(&window) {
                return Err(e);
            }
            state.break_duration_secs.store(duration_secs, Ordering::SeqCst);
            let payload = BreakStartedPayload {
                duration_secs,
                reason: reason.clone(),
            };
            push_started(&app, &window, &payload);
            let _ = app.emit("break-timer-started", &payload);
            return Ok(BreakTimerStatus {
                active: true,
                duration_secs,
                reason,
            });
        }
    }

    let was_paused = session_is_paused(&state);

    // Pause first so coach/camera treat the session as paused immediately.
    apply_lock_in_paused(&app, &state, true)?;
    overlay::hide(&app);
    state.break_allow_close.store(false, Ordering::SeqCst);

    let window = match ensure_break_window(&app) {
        Ok(w) => w,
        Err(e) => {
            rollback_failed_start(&app, &state, was_paused);
            return Err(e);
        }
    };

    if let Err(e) = present_break_window(&window) {
        rollback_failed_start(&app, &state, was_paused);
        return Err(e);
    }

    state.break_active.store(true, Ordering::SeqCst);
    state.break_duration_secs.store(duration_secs, Ordering::SeqCst);

    let payload = BreakStartedPayload {
        duration_secs,
        reason: reason.clone(),
    };
    push_started(&app, &window, &payload);
    let _ = app.emit("break-timer-started", &payload);

    // Page may still be loading — re-push shortly so countdown picks up duration.
    let handle = app.clone();
    let delayed = payload.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        if let Some(win) = handle.get_webview_window(BREAK_LABEL) {
            // Only refresh if break is still active.
            if handle
                .try_state::<AppState>()
                .map(|s| s.break_active.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                push_started(&handle, &win, &delayed);
                let _ = present_break_window(&win);
            }
        }
    });

    Ok(BreakTimerStatus {
        active: true,
        duration_secs,
        reason,
    })
}

#[tauri::command]
pub async fn end_break_timer(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<LockInSession>, String> {
    end_break_inner(&app, &state)
}

#[tauri::command]
pub async fn suggest_break_timer(
    app: AppHandle,
    duration_secs: Option<u64>,
    reason: Option<String>,
) -> Result<BreakSuggestedPayload, String> {
    let payload = BreakSuggestedPayload {
        duration_secs: normalize_duration(duration_secs),
        reason: normalize_reason(reason.as_deref(), "stress"),
    };

    // Never auto-start. Skip noise while already paused / breaking.
    if let Some(state) = app.try_state::<AppState>() {
        if state.break_active.load(Ordering::SeqCst) || session_is_paused(&state) {
            tracing::debug!("suggest_break_timer skipped — already paused or on break");
            return Ok(payload);
        }
    }

    let _ = app.emit("break-timer-suggested", &payload);
    Ok(payload)
}

#[tauri::command]
pub fn get_break_timer_status(state: State<'_, AppState>) -> BreakTimerStatus {
    let active = state.break_active.load(Ordering::SeqCst);
    BreakTimerStatus {
        active,
        duration_secs: state.break_duration_secs.load(Ordering::SeqCst),
        reason: if active {
            "active".into()
        } else {
            "idle".into()
        },
    }
}
