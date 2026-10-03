use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::capture::{self, camera, screen};
use crate::gemini::{CoachVisionResult, GeminiClient};
use crate::presage::{self, PresageClient, VitalsSnapshot};
use crate::session::{CoachPrompt, SessionStatusKind};
use crate::AppState;

const TICK_SECS: u64 = 12;
const PROMPT_COOLDOWN_SECS: i64 = 25;

pub fn spawn_coach_loop(app: AppHandle, session_id: String) {
    tauri::async_runtime::spawn(async move {
        let stop = Arc::new(AtomicBool::new(false));
        {
            let state = app.state::<AppState>();
            *state.coach_stop.lock() = Some(stop.clone());
        }

        let mut last_prompt_at = chrono::Utc::now() - chrono::Duration::seconds(PROMPT_COOLDOWN_SECS);
        let mut camera_frame_paths = Vec::new();
        let mut last_vitals = VitalsSnapshot::default();

        while !stop.load(Ordering::SeqCst) {
            let (active, goals, modality, remaining) = {
                let state = app.state::<AppState>();
                let guard = state.session.lock();
                match guard.as_ref() {
                    Some(s) if s.active && s.id == session_id => (
                        true,
                        s.goals.clone(),
                        s.modality.clone(),
                        s.remaining_secs(),
                    ),
                    _ => (false, String::new(), String::new(), 0),
                }
            };

            if !active || remaining <= 0 {
                finish_session(&app).await;
                break;
            }

            let camera_jpeg = match tokio::task::spawn_blocking(camera::grab_jpeg).await {
                Ok(Ok(bytes)) => Some(bytes),
                Ok(Err(e)) => {
                    tracing::warn!("camera: {e}");
                    None
                }
                Err(e) => {
                    tracing::warn!("camera join: {e}");
                    None
                }
            };

            let screen_jpeg = match tokio::task::spawn_blocking(screen::grab_primary_jpeg).await {
                Ok(Ok(bytes)) => Some(bytes),
                Ok(Err(e)) => {
                    tracing::warn!("screen: {e}");
                    None
                }
                Err(e) => {
                    tracing::warn!("screen join: {e}");
                    None
                }
            };

            if let Some(bytes) = camera_jpeg.as_ref() {
                if let Ok(dir) = capture::temp_session_dir(&session_id) {
                    if let Ok(path) = capture::save_jpeg(&dir, "cam", bytes) {
                        camera_frame_paths.push(path);
                        if camera_frame_paths.len() > 40 {
                            let _ = std::fs::remove_file(&camera_frame_paths[0]);
                            camera_frame_paths.remove(0);
                        }
                    }
                }
            }

            if camera_frame_paths.len() >= 15 && camera_frame_paths.len() % 15 == 0 {
                if let Some(v) = try_presage_clip(&app, &session_id, &camera_frame_paths).await {
                    last_vitals = v;
                }
            }

            let cfg = app.state::<AppState>().config.lock().clone();
            let vision = match GeminiClient::from_config(&cfg) {
                Ok(client) => {
                    client
                        .analyze_session(
                            &goals,
                            &modality,
                            &last_vitals.raw_summary,
                            screen_jpeg.as_deref(),
                            camera_jpeg.as_deref(),
                        )
                        .await
                }
                Err(e) => Err(e),
            };

            match vision {
                Ok(result) => {
                    apply_coach_tick(&app, &result, &mut last_vitals, &mut last_prompt_at).await;
                }
                Err(e) => {
                    tracing::warn!("coach vision: {e}");
                    let _ = app.emit("coach-error", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;
        }

        {
            let state = app.state::<AppState>();
            *state.coach_stop.lock() = None;
        }
    });
}

async fn try_presage_clip(
    app: &AppHandle,
    session_id: &str,
    frames: &[std::path::PathBuf],
) -> Option<VitalsSnapshot> {
    let cfg = app.state::<AppState>().config.lock().clone();
    if !PresageClient::configured(&cfg) {
        return None;
    }
    let dir = capture::temp_session_dir(session_id).ok()?;
    let out = dir.join("presage-clip.mp4");
    let recent: Vec<_> = frames.iter().rev().take(25).cloned().collect::<Vec<_>>();
    let recent: Vec<_> = recent.into_iter().rev().collect();
    if capture::encode_clip_from_jpegs(&recent, &out).is_err() {
        return None;
    }
    let client = PresageClient::from_config(&cfg).ok()?;
    let id = client.queue_video_hr_rr(&out).await.ok()?;
    match client.retrieve_result(&id, 20).await {
        Ok(data) => {
            let vitals = PresageClient::vitals_from_result(&data);
            let _ = app.emit("vitals-update", &vitals);
            Some(vitals)
        }
        Err(_) => None,
    }
}

async fn apply_coach_tick(
    app: &AppHandle,
    result: &CoachVisionResult,
    last_vitals: &mut VitalsSnapshot,
    last_prompt_at: &mut chrono::DateTime<chrono::Utc>,
) {
    if last_vitals.source != "presage" {
        *last_vitals = presage::fallback_vitals(result.stress_cue);
    } else if result.stress_cue {
        last_vitals.stressed = true;
        last_vitals.focus_ok = false;
    }

    let status = if result.needs_help {
        SessionStatusKind::NeedsHelp
    } else if last_vitals.stressed || result.stress_cue {
        SessionStatusKind::Stressed
    } else if !result.on_task {
        SessionStatusKind::Distracted
    } else {
        SessionStatusKind::OnTask
    };

    let mut prompt_text = result.coach_line.clone();
    let objects_lower: Vec<String> = result.objects.iter().map(|o| o.to_lowercase()).collect();
    let has_phone = objects_lower.iter().any(|o| o.contains("phone"));
    let has_calc = objects_lower
        .iter()
        .any(|o| o.contains("calculator") || o.contains("calc"));

    if has_phone && !has_calc {
        prompt_text = "Phone spotted — gently park it and come back to the work.".into();
    } else if has_calc && result.on_task && prompt_text.to_lowercase().contains("phone") {
        prompt_text = "Calculator is fair game — keep working the problem.".into();
    }

    if (last_vitals.stressed || result.stress_cue)
        && !prompt_text.to_lowercase().contains("breath")
    {
        prompt_text =
            format!("{prompt_text} If shoulders are tight, take one slow breath, then continue.");
    }

    let now = chrono::Utc::now();
    let should_prompt = now.signed_duration_since(*last_prompt_at).num_seconds()
        >= PROMPT_COOLDOWN_SECS
        && (!result.on_task || result.needs_help || last_vitals.stressed || has_phone);

    let prompt = if should_prompt {
        *last_prompt_at = now;
        Some(CoachPrompt {
            id: Uuid::new_v4().to_string(),
            at: now.to_rfc3339(),
            text: prompt_text,
            kind: format!("{:?}", status).to_lowercase(),
        })
    } else {
        None
    };

    let snapshot = {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.total_ticks += 1;
            if result.on_task {
                session.on_task_ticks += 1;
            }
            if last_vitals.stressed || result.stress_cue {
                session.stress_spikes += 1;
            }
            if let Some(d) = &result.distraction {
                session.bump_distraction(d);
            } else if has_phone {
                session.bump_distraction("phone");
            }
            session.status = status.clone();
            session.vitals = last_vitals.clone();
            if let Some(p) = prompt.clone() {
                session.prompts.push(p);
            }
            session.clone()
        } else {
            return;
        }
    };

    let _ = app.emit("session-update", &snapshot);
    if let Some(p) = prompt {
        let _ = app.emit("coach-prompt", &p);
    }
}

async fn finish_session(app: &AppHandle) {
    let summary = {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.active = false;
            Some(session.summarize())
        } else {
            None
        }
    };
    if let Some(summary) = summary {
        let _ = app.emit("session-ended", &summary);
    }
}

pub fn stop_coach(app: &AppHandle) {
    let state = app.state::<AppState>();
    {
        let stop_guard = state.coach_stop.lock();
        if let Some(stop) = stop_guard.as_ref() {
            stop.store(true, Ordering::SeqCst);
        }
    }
    {
        let mut session_guard = state.session.lock();
        if let Some(session) = session_guard.as_mut() {
            session.active = false;
        }
    }
}
