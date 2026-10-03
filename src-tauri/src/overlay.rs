use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::session::CoachPrompt;

const OVERLAY_LABEL: &str = "coach-overlay";

pub fn ensure_overlay(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        WebviewUrl::App("overlay.html".into()),
    )
    .title("Waypoint Coach")
    .inner_size(520.0, 160.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focused(false)
    .build()
    .map_err(|e| format!("coach overlay window: {e}"))?;

    // Never steal clicks — toast is visual-only.
    let _ = window.set_ignore_cursor_events(true);

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let width = (520.0 * scale) as i32;
        let x = (size.width as i32 - width) / 2;
        let y = (48.0 * scale) as i32;
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }

    Ok(())
}

pub fn show_prompt(app: &AppHandle, prompt: &CoachPrompt) {
    if let Err(e) = ensure_overlay(app) {
        tracing::warn!("{e}");
        return;
    }
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_always_on_top(true);

    let _ = app.emit_to(OVERLAY_LABEL, "overlay-prompt", prompt);
    if let Ok(json) = serde_json::to_string(prompt) {
        let _ = window.eval(&format!(
            "window.__waypointShow && window.__waypointShow({json})"
        ));
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = app.emit_to(OVERLAY_LABEL, "overlay-clear", ());
        let _ = window.eval("window.__waypointClear && window.__waypointClear()");
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.hide();
    }
}
