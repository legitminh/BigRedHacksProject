use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::capture::{self, camera, screen};
use crate::gemini::{CoachVisionResult, GeminiClient};
use crate::overlay;
use crate::presage::{self, PresageClient, VitalsSnapshot};
use crate::session::{CoachPrompt, SessionStatusKind};
use crate::AppState;

const SCREEN_TICK_SECS: u64 = 10;
const PROMPT_COOLDOWN_SECS: i64 = 20;
const PRESAGE_CLIP_SECS: u64 = 18;
const PRESAGE_FPS: u32 = 12;
const PRESAGE_GAP_SECS: u64 = 55;

pub fn spawn_coach_loop(app: AppHandle, session_id: String, use_camera: bool, use_presage: bool) {
    push_prompt(
        &app,
        "I'm watching your screen against your goals. Keep the work front and center.",
        "watching",
    );

    let stop = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<AppState>();
        *state.coach_stop.lock() = Some(stop.clone());
    }

    let screen_app = app.clone();
    let screen_id = session_id.clone();
    let screen_stop = stop.clone();
    tauri::async_runtime::spawn(async move {
        run_screen_loop(screen_app, screen_id, screen_stop, use_camera).await;
    });

    if use_camera && use_presage {
        let vitals_app = app.clone();
        let vitals_id = session_id;
        let vitals_stop = stop;
        tauri::async_runtime::spawn(async move {
            run_presage_loop(vitals_app, vitals_id, vitals_stop).await;
        });
    } else if use_presage && !use_camera {
        push_prompt(
            &app,
            "Presage is configured, but camera access is off — enable Camera in System Settings for stress checks.",
            "wellness",
        );
    } else if use_camera && !use_presage {
        push_prompt(
            &app,
            "Webcam is on. Add a Presage API key in secrets.toml to unlock heart-rate / stress readings.",
            "wellness",
        );
    }
}

async fn run_screen_loop(
    app: AppHandle,
    session_id: String,
    stop: Arc<AtomicBool>,
    use_camera: bool,
) {
    let mut last_prompt_at = chrono::Utc::now() - chrono::Duration::seconds(PROMPT_COOLDOWN_SECS);
    let mut last_vitals = VitalsSnapshot::default();
    let mut first = true;

    while !stop.load(Ordering::SeqCst) {
        let (active, goals, modality, remaining) = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => {
                    last_vitals = s.vitals.clone();
                    (
                        true,
                        s.goals.clone(),
                        s.modality.clone(),
                        s.remaining_secs(),
                    )
                }
                _ => (false, String::new(), String::new(), 0),
            }
        };

        if !active || remaining <= 0 {
            finish_session(&app).await;
            break;
        }

        if !first {
            tokio::time::sleep(Duration::from_secs(SCREEN_TICK_SECS)).await;
            if stop.load(Ordering::SeqCst) {
                break;
            }
        }
        first = false;

        let screen_jpeg = match tokio::task::spawn_blocking(screen::grab_primary_jpeg).await {
            Ok(Ok(bytes)) => Some(bytes),
            Ok(Err(e)) => {
                tracing::warn!("screen: {e}");
                let _ = app.emit("coach-error", e);
                None
            }
            Err(e) => {
                tracing::warn!("screen join: {e}");
                None
            }
        };

        // Occasional snapshot for posture context — not used for Presage timing.
        let camera_jpeg = if use_camera {
            match tokio::task::spawn_blocking(camera::grab_jpeg).await {
                Ok(Ok(bytes)) => Some(bytes),
                Ok(Err(e)) => {
                    tracing::warn!("camera snapshot: {e}");
                    None
                }
                Err(_) => None,
            }
        } else {
            None
        };

        if screen_jpeg.is_none() {
            continue;
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
                let _ = app.emit("coach-error", format!("Screen coach hiccup: {e}"));
            }
        }
    }
}

async fn run_presage_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
    // Short settle so the first screen tick can land first.
    tokio::time::sleep(Duration::from_secs(4)).await;

    while !stop.load(Ordering::SeqCst) {
        let active = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            guard
                .as_ref()
                .is_some_and(|s| s.active && s.id == session_id && s.remaining_secs() > 25)
        };
        if !active {
            break;
        }

        push_prompt(
            &app,
            "Taking a short wellness reading — face the camera, stay still for ~20 seconds.",
            "wellness",
        );

        let dir = match capture::temp_session_dir(&session_id) {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("coach-error", format!("Wellness temp dir: {e}"));
                break;
            }
        };

        let clip = {
            let dir = dir.clone();
            match tokio::task::spawn_blocking(move || {
                camera::record_presage_clip(&dir, PRESAGE_CLIP_SECS, PRESAGE_FPS)
            })
            .await
            {
                Ok(Ok(path)) => Some(path),
                Ok(Err(e)) => {
                    tracing::warn!("presage record: {e}");
                    let _ = app.emit("coach-error", format!("Wellness clip failed: {e}"));
                    None
                }
                Err(e) => {
                    tracing::warn!("presage record join: {e}");
                    None
                }
            }
        };

        if stop.load(Ordering::SeqCst) {
            break;
        }

        if let Some(path) = clip {
            if let Some(vitals) = upload_presage(&app, &path).await {
                store_vitals(&app, &vitals);
                let _ = app.emit("vitals-update", &vitals);
                if vitals.stressed {
                    push_prompt(
                        &app,
                        "Presage sees elevated stress — one slow breath, then return to the task on screen.",
                        "stressed",
                    );
                } else {
                    push_prompt(
                        &app,
                        "Wellness reading looks steady. Keep going.",
                        "wellness",
                    );
                }
            }
        }

        // Wait between clips; wake early if session ends.
        for _ in 0..PRESAGE_GAP_SECS {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let still = {
                let state = app.state::<AppState>();
                let guard = state.session.lock();
                guard
                    .as_ref()
                    .is_some_and(|s| s.active && s.id == session_id)
            };
            if !still {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn upload_presage(app: &AppHandle, video_path: &std::path::Path) -> Option<VitalsSnapshot> {
    let cfg = app.state::<AppState>().config.lock().clone();
    if !PresageClient::configured(&cfg) {
        return None;
    }
    let client = match PresageClient::from_config(&cfg) {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("coach-error", e);
            return None;
        }
    };
    let id = match client.queue_video_hr_rr(video_path).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("presage upload: {e}");
            let _ = app.emit("coach-error", format!("Presage upload failed: {e}"));
            return None;
        }
    };
    match client.retrieve_result(&id, 45).await {
        Ok(data) => Some(PresageClient::vitals_from_result(&data)),
        Err(e) => {
            tracing::warn!("presage retrieve: {e}");
            let _ = app.emit("coach-error", format!("Presage reading timed out: {e}"));
            None
        }
    }
}

fn store_vitals(app: &AppHandle, vitals: &VitalsSnapshot) {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock();
    if let Some(session) = guard.as_mut() {
        if vitals.stressed {
            session.stress_spikes += 1;
            session.status = SessionStatusKind::Stressed;
        }
        session.vitals = vitals.clone();
        session.watching_note = if vitals.stressed {
            "Screen watch active · Presage: stress elevated".into()
        } else {
            "Screen watch active · Presage: steady".into()
        };
        let snap = session.clone();
        drop(guard);
        let _ = app.emit("session-update", &snap);
    }
}

async fn apply_coach_tick(
    app: &AppHandle,
    result: &CoachVisionResult,
    last_vitals: &mut VitalsSnapshot,
    last_prompt_at: &mut chrono::DateTime<chrono::Utc>,
) {
    // Prefer real Presage readings; otherwise soft-fallback from vision stress cue.
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
        prompt_text = "Phone spotted — park it and come back to the work on screen.".into();
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
            if (last_vitals.stressed || result.stress_cue) && last_vitals.source != "presage" {
                // Presage path increments stress_spikes when a reading arrives.
                session.stress_spikes += 1;
            }
            if let Some(d) = &result.distraction {
                session.bump_distraction(d);
            } else if has_phone {
                session.bump_distraction("phone");
            }
            session.status = status;
            if last_vitals.source == "presage" || session.vitals.source != "presage" {
                session.vitals = last_vitals.clone();
            } else {
                *last_vitals = session.vitals.clone();
            }
            session.watching_note = "Watching your screen".into();
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
        deliver_prompt(app, &p);
    }
}

fn push_prompt(app: &AppHandle, text: &str, kind: &str) {
    let prompt = CoachPrompt {
        id: Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        text: text.into(),
        kind: kind.into(),
    };
    {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.prompts.push(prompt.clone());
        }
    }
    deliver_prompt(app, &prompt);
}

fn deliver_prompt(app: &AppHandle, prompt: &CoachPrompt) {
    // Pop over the desktop so you can see it while studying in other apps.
    overlay::show_prompt(app, prompt);
    // Local TTS stand-in until Grok Voice lands.
    if let Err(e) = waypoint_voice::speak(&prompt.text) {
        tracing::warn!("voice speak: {e}");
    }
}

async fn finish_session(app: &AppHandle) {
    waypoint_voice::stop_speaking();
    overlay::hide(app);
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
    waypoint_voice::stop_speaking();
    overlay::hide(app);
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
