use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::camera_observe;
use crate::camera_presence_live::{LivePresenceInference, LivePresenceState};
use crate::capture::{camera, camera_live, frontmost};
use crate::gemini::CoachVisionResult;
use crate::local_judge;
use crate::local_vision;
use crate::overlay;
use crate::presage::{self, VitalsSnapshot};
use crate::session::{CoachPrompt, LockInSession, SessionStatusKind};
use crate::AppState;

/// Poll for app/tab switches — free OS signals, so stay snappy.
const LOCAL_TICK_SECS: u64 = 1;
/// Apple Vision OCR on a small window capture (cheap, no cloud).
const OCR_TICK_SECS: u64 = 3;
/// Rare tiny local VLM (moondream) when OCR/OS screen signals are inconclusive.
/// Never used on webcam — camera presence/phone is Presage (VIDEOINPUT), not local LLM.
const LOCAL_VLM_TICK_SECS: u64 = 45;
/// Ambiguous context judgments per session (local model only — no Gemini in lock-in).
const MAX_CONTEXT_JUDGMENTS_PER_SESSION: u32 = 120;
/// Soft coach lines (help / stress) — don't spam.
const PROMPT_COOLDOWN_SECS: i64 = 14;
/// Absolute floor between any popup/voice (including praise).
const EPHEMERAL_FLOOR_SECS: i64 = 8;
/// Camera presence nudges use their own short anti-spam — they must not be
/// permanently dropped when a screen nag just claimed the 8s global floor
/// (server may already have advanced `ladderSpoken`).
const CAMERA_PRESENCE_COOLDOWN_SECS: i64 = 4;
/// Tiny gap between back-on-task dings — does not share the nag/voice floor.
const DING_COOLDOWN_SECS: i64 = 2;
/// Max spoken/popup reminders for one continuous distraction (e.g. one Instagram stay).
const MAX_NAGS_PER_EPISODE: u32 = 3;
/// Seconds to wait after nag 1 → nag 2, then after nag 2 → nag 3.
const EPISODE_GAP_AFTER_FIRST_SECS: i64 = 14;
const EPISODE_GAP_AFTER_SECOND_SECS: i64 = 22;
/// Optional sparse Presage vitals clip (demoted — live face owns presence/nudges).
/// Kept for a future ring-buffer upload path; live loop does not open a second camera.
#[allow(dead_code)]
const PRESAGE_CLIP_SECS: u64 = 12;
#[allow(dead_code)]
const PRESAGE_FPS: u32 = 12;
/// Sparse vitals-only gap if clip upload is re-enabled alongside live presence.
#[allow(dead_code)]
const PRESAGE_VITALS_GAP_SECS: u64 = 90;
/// Quiet-phase JPEG heartbeats (no webcam) — keep sparse vs the observe rate bucket.
const PRESAGE_QUIET_GAP_SECS: u64 = 60;
/// Brief settle so opener / local watch start before the first webcam grab.
const PRESAGE_START_DELAY_SECS: u64 = 20;
/// Let the student settle before distraction tracking / nags begin.
const TRACKING_WARMUP_SECS: i64 = 15;
/// Fixed opener — never LLM/system-prompt text (tiny models regurgitate prompts).
const SESSION_OPENER: &str = "You're locked in. I'll check in if you drift.";
const SESSION_OPENER_NO_SCREEN: &str = "You're locked in. Screen sharing is off, so I'm just keeping time.";

/// Shared across local watch ticks so OCR/VLM don't re-fire the same nag.
static LAST_EPHEMERAL_AT: Mutex<Option<chrono::DateTime<chrono::Utc>>> = Mutex::new(None);
static LAST_DING_AT: Mutex<Option<chrono::DateTime<chrono::Utc>>> = Mutex::new(None);
static LAST_CAMERA_PRESENCE_AT: Mutex<Option<chrono::DateTime<chrono::Utc>>> = Mutex::new(None);

#[derive(Clone)]
struct DistractionEpisode {
    key: String,
    nags: u32,
    last_nag_at: chrono::DateTime<chrono::Utc>,
}

static DISTRACTION_EPISODE: Mutex<Option<DistractionEpisode>> = Mutex::new(None);

/// At most one queued camera-presence speak when the short camera cooldown blocks delivery.
#[derive(Clone)]
struct PendingCameraPresence {
    kind: String,
    text: String,
}

static PENDING_CAMERA_PRESENCE: Mutex<Option<PendingCameraPresence>> = Mutex::new(None);

fn reset_ephemeral_cooldown() {
    if let Ok(mut guard) = LAST_EPHEMERAL_AT.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = LAST_DING_AT.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = LAST_CAMERA_PRESENCE_AT.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = DISTRACTION_EPISODE.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = PENDING_CAMERA_PRESENCE.lock() {
        *guard = None;
    }
}

fn claim_ding_slot() -> bool {
    let now = chrono::Utc::now();
    let Ok(mut guard) = LAST_DING_AT.lock() else {
        return true;
    };
    if let Some(last) = *guard {
        if now.signed_duration_since(last).num_seconds() < DING_COOLDOWN_SECS {
            return false;
        }
    }
    *guard = Some(now);
    true
}

/// Global floor so voice/overlay never stack, regardless of distraction episode.
fn claim_global_floor() -> bool {
    let now = chrono::Utc::now();
    let Ok(mut at_guard) = LAST_EPHEMERAL_AT.lock() else {
        return true;
    };
    if let Some(last) = *at_guard {
        if now.signed_duration_since(last).num_seconds() < EPHEMERAL_FLOOR_SECS {
            return false;
        }
    }
    *at_guard = Some(now);
    true
}

/// Few spaced reminders per continuous distraction; resets when they leave that behavior.
fn claim_distraction_nag(key: &str) -> bool {
    let now = chrono::Utc::now();
    if !claim_global_floor() {
        return false;
    }
    let Ok(mut guard) = DISTRACTION_EPISODE.lock() else {
        return true;
    };
    match guard.as_mut() {
        Some(ep) if ep.key == key => {
            if ep.nags >= MAX_NAGS_PER_EPISODE {
                // Already reminded enough this stay — stay quiet until they leave.
                // Undo the global floor claim so praise/other keys aren't blocked forever.
                if let Ok(mut at) = LAST_EPHEMERAL_AT.lock() {
                    *at = None;
                }
                return false;
            }
            let gap = match ep.nags {
                1 => EPISODE_GAP_AFTER_FIRST_SECS,
                2 => EPISODE_GAP_AFTER_SECOND_SECS,
                _ => EPISODE_GAP_AFTER_SECOND_SECS,
            };
            if now.signed_duration_since(ep.last_nag_at).num_seconds() < gap {
                if let Ok(mut at) = LAST_EPHEMERAL_AT.lock() {
                    *at = None;
                }
                return false;
            }
            ep.nags += 1;
            ep.last_nag_at = now;
            true
        }
        _ => {
            *guard = Some(DistractionEpisode {
                key: key.to_string(),
                nags: 1,
                last_nag_at: now,
            });
            true
        }
    }
}

fn clear_distraction_episode() {
    if let Ok(mut guard) = DISTRACTION_EPISODE.lock() {
        *guard = None;
    }
}

fn distraction_episode_nags(key: &str) -> u32 {
    let Ok(guard) = DISTRACTION_EPISODE.lock() else {
        return 0;
    };
    match guard.as_ref() {
        Some(ep) if ep.key == key => ep.nags,
        _ => 0,
    }
}

/// Desk-away / gaze / obstructed nudges — must deliver even if a screen nag just
/// claimed the global overlay floor (server may already have advanced the ladder).
fn is_camera_presence_kind(kind: &str) -> bool {
    matches!(
        kind,
        "left_desk"
            | "left_desk_pause"
            | "look_back"
            | "welcome_back"
            | "camera_obstructed"
    )
}

fn claim_camera_presence_slot() -> bool {
    let now = chrono::Utc::now();
    let Ok(mut at_guard) = LAST_CAMERA_PRESENCE_AT.lock() else {
        return true;
    };
    if let Some(last) = *at_guard {
        if now.signed_duration_since(last).num_seconds() < CAMERA_PRESENCE_COOLDOWN_SECS {
            return false;
        }
    }
    *at_guard = Some(now);
    true
}

fn stamp_global_floor() {
    if let Ok(mut at) = LAST_EPHEMERAL_AT.lock() {
        *at = Some(chrono::Utc::now());
    }
}

fn enqueue_camera_presence(kind: &str, text: &str) {
    if let Ok(mut guard) = PENDING_CAMERA_PRESENCE.lock() {
        *guard = Some(PendingCameraPresence {
            kind: kind.to_string(),
            text: text.to_string(),
        });
    }
}

fn take_pending_camera_presence() -> Option<PendingCameraPresence> {
    let Ok(mut guard) = PENDING_CAMERA_PRESENCE.lock() else {
        return None;
    };
    guard.take()
}

/// Away-ladder mid step vs stress-family break invite (same nudge kind).
fn suggest_break_reason(text: &str, presence: Option<&str>) -> &'static str {
    let lower = text.to_lowercase();
    if presence == Some("left_frame")
        || lower.contains("away")
        || lower.contains("stepped")
    {
        "away"
    } else {
        "stress"
    }
}

fn claim_ephemeral_slot_keyed(cooldown_secs: i64, key: Option<&str>) -> bool {
    let _ = cooldown_secs;
    // Non-distraction lines (praise / watching / soft camera) — global floor only.
    // Desk-presence kinds bypass the 8s global floor (see `claim_camera_presence_slot`)
    // so a recent Instagram nag cannot permanently drop `left_desk` / `look_back`.
    // They must NOT share the Instagram-style episode cap (max 3) either.
    if let Some(k) = key {
        if is_camera_presence_kind(k) {
            return claim_camera_presence_slot();
        }
        if matches!(
            k,
            "encourage" | "watching" | "on_task" | "stressed" | "camera" | "suggest_break"
        ) {
            if matches!(k, "encourage" | "on_task") {
                clear_distraction_episode();
            }
            return claim_global_floor();
        }
        return claim_distraction_nag(k);
    }
    claim_global_floor()
}

/// `use_screen`: the student opted in to screen watching (frontmost app/title/URL, OCR, local VLM).
/// When false none of those run — the loop only keeps the timer honest.
/// `use_camera`: camera opted in AND permission available; gates webcam record + observe/Presage.
pub fn spawn_coach_loop(
    app: AppHandle,
    session_id: String,
    use_screen: bool,
    use_camera: bool,
    use_presage: bool,
) {
    reset_ephemeral_cooldown();
    {
        let greet_app = app.clone();
        tauri::async_runtime::spawn(async move {
            // Short fixed phrase only — never compose/LLM (avoids reading the prompt aloud).
            let line = if use_screen {
                SESSION_OPENER
            } else {
                SESSION_OPENER_NO_SCREEN
            };
            push_prompt(&greet_app, line, "watching");
        });
    }

    {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.watching_note = if use_screen {
                if use_camera || session.camera_enabled {
                    format!("Settling in — tracking starts in {TRACKING_WARMUP_SECS}s…")
                } else {
                    format!(
                        "Settling in — tracking starts in {TRACKING_WARMUP_SECS}s · camera accountability off"
                    )
                }
            } else {
                "Screen sharing off · timer and check-ins only".into()
            };
            session.status = SessionStatusKind::OnTask;
            let snap = session.clone();
            drop(guard);
            let _ = app.emit("session-update", &snap);
        }
    }

    // Probe Ollama in the background so the UI can show if on-device judging is live.
    // Skipped entirely when screen watching is off (nothing local would be judging).
    if use_screen {
        let probe_app = app.clone();
        let probe_id = session_id.clone();
        tauri::async_runtime::spawn(async move {
            let cfg = probe_app.state::<AppState>().config.lock().clone();
            let text_line = local_judge::status_line(&cfg).await;
            let vlm_line = local_vision::vlm_status_line(&cfg).await;
            let ocr_line = if std::path::Path::new(env!("WAYPOINT_OCR_BIN")).is_file() {
                "OCR ready"
            } else {
                "OCR missing (rebuild)"
            };
            let state = probe_app.state::<AppState>();
            let mut guard = state.session.lock();
            if let Some(session) = guard.as_mut() {
                if session.active && session.id == probe_id {
                    let mut note =
                        format!("Local watch · {ocr_line} · {text_line} · {vlm_line}");
                    if !session.camera_enabled {
                        note.push_str(" · camera accountability off");
                    }
                    session.watching_note = note;
                    let snap = session.clone();
                    drop(guard);
                    let _ = probe_app.emit("session-update", &snap);
                }
            }
        });
    }

    let stop = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<AppState>();
        *state.coach_stop.lock() = Some(stop.clone());
    }

    let local_app = app.clone();
    let local_id = session_id.clone();
    let local_stop = stop.clone();
    tauri::async_runtime::spawn(async move {
        run_local_watch_loop(local_app, local_id, local_stop, use_screen).await;
    });

    // Camera accountability: continuous local webcam (~2 Hz) + PresenceInference.
    // Presage clip upload is demoted — live face owns presence/nudges (one camera owner).
    if use_camera {
        let _ = use_presage;
        let vitals_app = app;
        let vitals_id = session_id;
        let vitals_stop = stop;
        tauri::async_runtime::spawn(async move {
            run_camera_accountability_loop(vitals_app, vitals_id, vitals_stop).await;
        });
    }
}

/// Primary interrupter: reacts to app/tab open/switch events; YouTube gets text context.
async fn run_local_watch_loop(
    app: AppHandle,
    session_id: String,
    stop: Arc<AtomicBool>,
    use_screen: bool,
) {
    let mut last_vitals = VitalsSnapshot::default();
    let mut was_distracted = false;
    let mut last_fingerprint = String::new();
    let mut last_judged_fingerprint = String::new();
    let mut context_judgments: u32 = 0;
    let mut last_ocr_at = chrono::Utc::now() - chrono::Duration::seconds(OCR_TICK_SECS as i64);
    let mut last_vlm_at = chrono::Utc::now() - chrono::Duration::seconds(LOCAL_VLM_TICK_SECS as i64);
    let mut last_ocr_snippet = String::new();
    let tracking_ready_at = chrono::Utc::now() + chrono::Duration::seconds(TRACKING_WARMUP_SECS);
    let mut announced_tracking = false;

    while !stop.load(Ordering::SeqCst) {
        let (active, expired, goals, paused) = {
            let state = app.state::<AppState>();
            let pause_started = *state.pause_started.lock();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => {
                    last_vitals = s.vitals.clone();
                    // Pause-aware: a break must never trip natural expiry.
                    (true, s.is_expired(pause_started), s.goals.clone(), s.paused)
                }
                _ => (false, false, String::new(), false),
            }
        };
        if !active || expired {
            // Only a natural timer expiry finishes here. A user "End" goes through
            // `stop_lock_in`, which owns the summary and emits `lock-in-stopped`.
            // Emitting `session-ended` here too would double-count the mission.
            if active && expired && !stop.load(Ordering::SeqCst) {
                finish_session(&app, &session_id).await;
            }
            break;
        }
        if paused {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }
        // Retry a camera presence nudge that was deferred (short camera cooldown).
        flush_pending_camera_presence(&app);
        if !use_screen {
            // No consent for screen watching: no frontmost/title/URL reads, OCR, or VLM.
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let now = chrono::Utc::now();
        if now < tracking_ready_at {
            let left = (tracking_ready_at - now).num_seconds().max(0);
            {
                let state = app.state::<AppState>();
                let mut guard = state.session.lock();
                if let Some(session) = guard.as_mut() {
                    let note = format!("Settling in — tracking in {left}s…");
                    // Emit ≤1 Hz during warmup (only when the displayed second changes).
                    if session.watching_note != note {
                        session.watching_note = note;
                        session.status = SessionStatusKind::OnTask;
                        let snap = session.clone();
                        drop(guard);
                        let _ = app.emit("session-update", &snap);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }
        if !announced_tracking {
            announced_tracking = true;
            {
                let state = app.state::<AppState>();
                let mut guard = state.session.lock();
                if let Some(session) = guard.as_mut() {
                    session.watching_note = "Watching with you…".into();
                    let snap = session.clone();
                    drop(guard);
                    let _ = app.emit("session-update", &snap);
                }
            }
            // Seed switch fingerprint only — leave last_judged empty so an
            // ambiguous page already open during warmup still gets judged.
            if let Ok(Ok(info)) = tokio::task::spawn_blocking(frontmost::frontmost_info).await {
                last_fingerprint = info.fingerprint();
            }
        }

        let goals_for_scan = goals.clone();
        let scanned = tokio::task::spawn_blocking(move || frontmost::evaluate_focus(&goals_for_scan))
            .await;
        let event = match scanned {
            Ok(Ok(event)) => Some(event),
            Ok(Err(e)) => {
                tracing::warn!("focus scan: {e}");
                let state = app.state::<AppState>();
                let mut guard = state.session.lock();
                if let Some(session) = guard.as_mut() {
                    session.watching_note = format!(
                        "Can’t read screen focus ({e}). Allow Accessibility + Automation for Waypoint in System Settings."
                    );
                    let snap = session.clone();
                    drop(guard);
                    let _ = app.emit("session-update", &snap);
                }
                None
            }
            Err(e) => {
                tracing::warn!("focus scan join: {e}");
                None
            }
        };

        if let Some(event) = event {
            match event {
                frontmost::FocusEvent::Hard(hit) => {
                    // Defensive (works with or without A's frontmost fix): never "leave youtube"
                    // while the focused host is Canvas / anything non-YouTube.
                    let front_now = tokio::task::spawn_blocking(frontmost::frontmost_info)
                        .await
                        .ok()
                        .and_then(|r| r.ok());
                    if matches!(hit.label, "youtube" | "video") {
                        if let Some(ref info) = front_now {
                            if !frontmost::focus_supports_distraction_label(hit.label, info) {
                                append_screen_log(
                                    &app,
                                    &format!(
                                        "Skip {} hard-nag — focused {}",
                                        hit.label,
                                        info.summary()
                                    ),
                                );
                                if was_distracted
                                    || last_fingerprint.contains("youtube")
                                    || last_fingerprint.contains("video")
                                {
                                    was_distracted = false;
                                    clear_distraction_episode();
                                    last_fingerprint = info.fingerprint();
                                    play_back_on_task_ding(&app);
                                }
                                mark_local_on_task(&app, info);
                                // Fall through to OCR tick with a clean distraction state.
                                tokio::time::sleep(Duration::from_millis(450)).await;
                                continue;
                            }
                        }
                    }
                    // Key by label only — detail churn was resetting cooldown and stacking voice.
                    let fp = format!("hard:{}", hit.label);
                    last_fingerprint = fp;
                    append_screen_log(&app, &format!("{} · {}", hit.label, hit.detail));
                    let cfg = app.state::<AppState>().config.lock().clone();
                    let nag_n = distraction_episode_nags(hit.label).saturating_add(1);
                    // Background-tab hits must not say the focused app *is* the distraction.
                    let coach_line = if !hit.focused {
                        frontmost::distraction_coach_line(&hit)
                    } else {
                        local_judge::compose_coach_line(
                            &cfg,
                            "distracted",
                            &goals,
                            hit.label,
                            &hit.detail,
                            nag_n,
                        )
                        .await
                    };
                    let result = CoachVisionResult {
                        on_task: false,
                        objects: vec![hit.label.into(), hit.detail.clone()],
                        distraction: Some(hit.label.into()),
                        needs_help: false,
                        stress_cue: last_vitals.stressed,
                        coach_line,
                        modality: Some("computer".into()),
                    };
                    apply_coach_tick(&app, &result, &mut last_vitals).await;
                    was_distracted = true;
                }
                frontmost::FocusEvent::NeedsJudgment {
                    kind,
                    info,
                    page_text,
                } => {
                    // Defensive: if kind says youtube but focused host isn't, don't imply
                    // the student is on YouTube (stale/background mislabel).
                    let kind_supported = !matches!(kind, "youtube" | "video")
                        || frontmost::focus_supports_distraction_label(kind, &info);
                    if !kind_supported {
                        append_screen_log(
                            &app,
                            &format!("Skip {kind} judgment — focused {}", info.summary()),
                        );
                        if was_distracted {
                            was_distracted = false;
                            clear_distraction_episode();
                            play_back_on_task_ding(&app);
                        }
                        last_fingerprint = info.fingerprint();
                        mark_local_on_task(&app, &info);
                    } else {
                        let fp = info.fingerprint();
                        let switched = fp != last_fingerprint;
                        last_fingerprint = fp.clone();
                        if switched {
                            append_screen_log(&app, &info.summary());
                            if !page_text.trim().is_empty() {
                                append_screen_log(&app, &format!("Page: {page_text}"));
                            }
                        }

                        // 1) Instant keyword guess for obvious study vs entertainment.
                        //    Unclear YouTube/video/gaming → off-task (interrupt by default).
                        let mut heuristic =
                            frontmost::local_context_guess(kind, &page_text, &goals);
                        if heuristic.is_none() && matches!(kind, "youtube" | "video" | "gaming")
                        {
                            heuristic = Some(false);
                        }
                        if let Some(on_task) = heuristic {
                            if on_task {
                                if was_distracted && switched {
                                    was_distracted = false;
                                    play_back_on_task_ding(&app);
                                }
                                mark_local_on_task(&app, &info);
                            } else {
                                let hit = frontmost::DistractionHit {
                                    label: kind,
                                    detail: info.summary(),
                                    focused: true,
                                };
                                let cfg = app.state::<AppState>().config.lock().clone();
                                let nag_n = distraction_episode_nags(kind).saturating_add(1);
                                let coach_line = local_judge::compose_coach_line(
                                    &cfg,
                                    "distracted",
                                    &goals,
                                    kind,
                                    &hit.detail,
                                    nag_n,
                                )
                                .await;
                                let result = CoachVisionResult {
                                    on_task: false,
                                    objects: vec![kind.into(), info.window_title.clone()],
                                    distraction: Some(kind.into()),
                                    needs_help: false,
                                    stress_cue: last_vitals.stressed,
                                    coach_line,
                                    modality: Some("computer".into()),
                                };
                                apply_coach_tick(&app, &result, &mut last_vitals).await;
                                was_distracted = true;
                            }
                        } else if switched {
                            let state = app.state::<AppState>();
                            let mut guard = state.session.lock();
                            if let Some(session) = guard.as_mut() {
                                session.watching_note =
                                    format!("Checking if this {kind} fits your goals…");
                                let snap = session.clone();
                                drop(guard);
                                let _ = app.emit("session-update", &snap);
                            }
                        }

                        // 2) Ambiguous → local Ollama/Llama judge only (no Gemini in lock-in).
                        if heuristic.is_none()
                            && fp != last_judged_fingerprint
                            && context_judgments < MAX_CONTEXT_JUDGMENTS_PER_SESSION
                        {
                            context_judgments = context_judgments.saturating_add(1);
                            let judged = tokio::time::timeout(
                                Duration::from_secs(12),
                                judge_context_cascade(
                                    &app,
                                    &goals,
                                    kind,
                                    &info,
                                    &page_text,
                                    &mut last_vitals,
                                ),
                            )
                            .await;
                            match judged {
                                Ok(Some(on_task)) => {
                                    last_judged_fingerprint = fp;
                                    if on_task {
                                        if was_distracted {
                                            was_distracted = false;
                                            play_back_on_task_ding(&app);
                                        }
                                    } else {
                                        was_distracted = true;
                                    }
                                }
                                Ok(None) | Err(_) => {
                                    // Don't pin fingerprint on failure — allow retry next tick.
                                    tracing::warn!(
                                        "context judge failed/timed out for {kind} — treating as off-task"
                                    );
                                    let hit = frontmost::DistractionHit {
                                        label: kind,
                                        detail: info.summary(),
                                        focused: true,
                                    };
                                    let result = CoachVisionResult {
                                        on_task: false,
                                        objects: vec![kind.into(), info.window_title.clone()],
                                        distraction: Some(kind.into()),
                                        needs_help: false,
                                        stress_cue: last_vitals.stressed,
                                        coach_line: frontmost::distraction_coach_line(&hit),
                                        modality: Some("computer".into()),
                                    };
                                    apply_coach_tick(&app, &result, &mut last_vitals).await;
                                    was_distracted = true;
                                    last_judged_fingerprint = fp;
                                }
                            }
                        }
                    }
                }
                frontmost::FocusEvent::Clear(info) => {
                    let fp = info.fingerprint();
                    // School/Canvas Clear must kill a stale youtube episode — don't keep
                    // firing "leave youtube" from a prior OCR/hard fingerprint.
                    let stale_youtube = last_fingerprint.contains("youtube")
                        || last_fingerprint.contains("video")
                        || last_fingerprint.starts_with("ocr:youtube")
                        || last_fingerprint.starts_with("hard:youtube");
                    if (was_distracted || stale_youtube)
                        && !frontmost::focus_supports_distraction_label("youtube", &info)
                    {
                        clear_distraction_episode();
                    }
                    last_fingerprint = fp;
                    if was_distracted {
                        was_distracted = false;
                        play_back_on_task_ding(&app);
                    }
                    mark_local_on_task(&app, &info);
                    // Focused app/title only — never surface raw OCR glyphs in the mission UI.
                    // Keep Presage/camera presence notes when the student is away/obstructed.
                    let state = app.state::<AppState>();
                    let mut guard = state.session.lock();
                    if let Some(session) = guard.as_mut() {
                        session.push_screen_log(&info.summary());
                        if matches!(session.status, SessionStatusKind::OnTask)
                            && !camera_presence_owns_note(&session.vitals.raw_summary)
                        {
                            session.watching_note = format!("On task · {}", info.summary());
                            let snap = session.clone();
                            drop(guard);
                            let _ = app.emit("session-update", &snap);
                        }
                    }
                }
            }
        }

        // Local OCR pass — sees the actual webpage pixels when AppleScript URL fails.
        let now = chrono::Utc::now();
        if now.signed_duration_since(last_ocr_at).num_seconds() >= OCR_TICK_SECS as i64 {
            last_ocr_at = now;
            // Only skip OCR while the Waypoint coach UI is focused — Cursor/editors
            // must still be goal-checked (English discussion in Cursor ≠ auto-clear).
            let focus_now = tokio::task::spawn_blocking(frontmost::frontmost_info)
                .await
                .ok()
                .and_then(|r| r.ok());
            let coach_focused = focus_now
                .as_ref()
                .map(|i| frontmost::is_coach_app(&i.app_name))
                .unwrap_or(false);
            if coach_focused {
                last_ocr_at = now;
                tokio::time::sleep(Duration::from_millis(450)).await;
                continue;
            }
            let ocr_result = tokio::task::spawn_blocking(local_vision::read_screen_ocr).await;
            match ocr_result {
                Ok(Ok(reading)) => {
                    last_ocr_snippet = reading
                        .ocr_text
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !last_ocr_snippet.trim().is_empty() {
                        append_screen_log(&app, &format!("OCR: {last_ocr_snippet}"));
                    }
                    // OCR labels are host/chrome-strong only — ignore brand word mentions.
                    if let Some(label) = reading.labels.first().copied() {
                        // Frontmost URL/title must corroborate the OCR brand. Canvas on
                        // canvas.cornell.edu must never nag "leave youtube" from a ghost label.
                        let corroborated = focus_now
                            .as_ref()
                            .map(|info| {
                                frontmost::focus_supports_distraction_label(label, info)
                            })
                            .unwrap_or(true);
                        if !corroborated {
                            append_screen_log(
                                &app,
                                &format!(
                                    "OCR ignore {label} — focused {}",
                                    focus_now
                                        .as_ref()
                                        .map(|i| i.summary())
                                        .unwrap_or_default()
                                ),
                            );
                            if was_distracted
                                && matches!(label, "youtube" | "video")
                                && focus_now.as_ref().is_some_and(|info| {
                                    !frontmost::focus_supports_distraction_label("youtube", info)
                                })
                            {
                                was_distracted = false;
                                clear_distraction_episode();
                            }
                        } else {
                            // YouTube only skips the OCR nag when title/OCR clearly looks like study.
                            // Unclear entertainment must still distract (interrupt by default).
                            let study_ok = label == "youtube"
                                && frontmost::local_context_guess(
                                    "youtube",
                                    &reading.ocr_text,
                                    &goals,
                                ) == Some(true);
                            if !study_ok {
                                last_fingerprint = format!("ocr:{label}");
                                let hit = frontmost::DistractionHit {
                                    label,
                                    detail: truncate_note(
                                        &reading.ocr_text.replace('\n', " "),
                                        64,
                                    ),
                                    focused: true,
                                };
                                let result = CoachVisionResult {
                                    on_task: false,
                                    objects: vec![label.into(), "ocr".into()],
                                    distraction: Some(label.into()),
                                    needs_help: false,
                                    stress_cue: last_vitals.stressed,
                                    coach_line: frontmost::distraction_coach_line(&hit),
                                    modality: Some("computer".into()),
                                };
                                apply_coach_tick(&app, &result, &mut last_vitals).await;
                                was_distracted = true;
                            }
                        }
                    } else if !reading.ocr_text.is_empty() {
                        // Feed OCR into local text model when OS signals were Clear/ambiguous.
                        let unclear = !was_distracted
                            && context_judgments < MAX_CONTEXT_JUDGMENTS_PER_SESSION
                            && now.signed_duration_since(last_vlm_at).num_seconds()
                                >= (LOCAL_VLM_TICK_SECS as i64 / 2);
                        if unclear {
                            let cfg = app.state::<AppState>().config.lock().clone();
                            if local_judge::is_available(&cfg).await {
                                context_judgments = context_judgments.saturating_add(1);
                                if let Ok(j) = local_judge::judge_on_task(
                                    &cfg,
                                    &goals,
                                    "screen",
                                    "Screen",
                                    &last_ocr_snippet,
                                    "",
                                    &reading.ocr_text,
                                )
                                .await
                                {
                                    if j.confidence >= local_judge::min_confidence() {
                                        let verdict = if j.result.on_task {
                                            "on task"
                                        } else {
                                            "off task"
                                        };
                                        append_screen_log(
                                            &app,
                                            &format!("Judge: {verdict} — {}", j.result.coach_line),
                                        );
                                        if !j.result.on_task {
                                            apply_coach_tick(&app, &j.result, &mut last_vitals).await;
                                            was_distracted = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => tracing::warn!("ocr: {e}"),
                Err(e) => tracing::warn!("ocr join: {e}"),
            }
        }

        // Rare tiny VLM — only when still looking "on task" (OCR found nothing bad).
        if !was_distracted
            && now.signed_duration_since(last_vlm_at).num_seconds() >= LOCAL_VLM_TICK_SECS as i64
        {
            last_vlm_at = now;
            let cfg = app.state::<AppState>().config.lock().clone();
            if local_vision::vlm_available(&cfg).await {
                let frame = tokio::task::spawn_blocking(local_vision::grab_judge_jpeg)
                    .await
                    .ok()
                    .and_then(|r| r.ok());
                if let Some(jpeg) = frame {
                    match local_vision::judge_frame(&cfg, &goals, &jpeg, &last_ocr_snippet).await
                    {
                        Ok((result, conf)) => {
                            tracing::info!("local VLM conf={conf:.2} on_task={}", result.on_task);
                            if !result.on_task {
                                apply_coach_tick(&app, &result, &mut last_vitals).await;
                                was_distracted = true;
                            }
                        }
                        Err(e) => tracing::warn!("local VLM: {e}"),
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(LOCAL_TICK_SECS)).await;
    }
}

fn truncate_note(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        let cut: String = flat.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Local quantized model only — Gemini is never used during lock-in.
async fn judge_context_cascade(
    app: &AppHandle,
    goals: &str,
    kind: &str,
    info: &frontmost::FrontmostInfo,
    page_text: &str,
    last_vitals: &mut VitalsSnapshot,
) -> Option<bool> {
    let cfg = app.state::<AppState>().config.lock().clone();

    // Enrich YouTube with oEmbed title when page JS is blocked.
    let mut text = page_text.to_string();
    if kind == "youtube" && !info.url.is_empty() {
        if let Some(oembed) = fetch_youtube_oembed_title(&info.url).await {
            if !text.to_lowercase().contains(&oembed.to_lowercase()) {
                text = format!("{oembed}\n{text}");
            }
        }
    }

    if !local_judge::is_available(&cfg).await {
        return None;
    }

    match local_judge::judge_on_task(
        &cfg,
        goals,
        kind,
        &info.app_name,
        &info.window_title,
        &info.url,
        &text,
    )
    .await
    {
        Ok(judgment) if judgment.confidence >= local_judge::min_confidence() => {
            tracing::info!(
                "local judge ({}) conf={:.2} on_task={}",
                judgment.model,
                judgment.confidence,
                judgment.result.on_task
            );
            let on_task = judgment.result.on_task && judgment.result.distraction.is_none();
            let verdict = if on_task { "on task" } else { "off task" };
            append_screen_log(
                app,
                &format!(
                    "Judge: {verdict} · {} · {} — {}",
                    info.app_name, info.window_title, judgment.result.coach_line
                ),
            );
            apply_coach_tick(app, &judgment.result, last_vitals).await;
            Some(on_task)
        }
        Ok(judgment) => {
            tracing::info!(
                "local judge low confidence ({:.2}) — skipping (no Gemini in lock-in)",
                judgment.confidence
            );
            None
        }
        Err(e) => {
            tracing::warn!("local judge: {e}");
            None
        }
    }
}

async fn fetch_youtube_oembed_title(url: &str) -> Option<String> {
    let endpoint = format!(
        "https://www.youtube.com/oembed?format=json&url={}",
        urlencoding::encode(url)
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .ok()?;
    let value: serde_json::Value = client.get(endpoint).send().await.ok()?.json().await.ok()?;
    value
        .get("title")
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn mark_local_on_task(app: &AppHandle, info: &frontmost::FrontmostInfo) {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock();
    let Some(session) = guard.as_mut() else {
        return;
    };
    // Don't clobber an active stress state from Presage.
    if matches!(session.status, SessionStatusKind::Stressed) {
        return;
    }
    session.total_ticks = session.total_ticks.saturating_add(1);
    session.on_task_ticks = session.on_task_ticks.saturating_add(1);
    session.status = SessionStatusKind::OnTask;
    // Screen focus note must not erase Presage away/obstructed accountability copy.
    if !camera_presence_owns_note(&session.vitals.raw_summary) {
        session.watching_note = if session.camera_enabled {
            format!("Watching full screen · {}", info.summary())
        } else {
            format!(
                "Watching full screen · {} · camera accountability off",
                info.summary()
            )
        };
    }
    session.push_screen_log(&info.summary());
    let snap = session.clone();
    drop(guard);
    let _ = app.emit("session-update", &snap);
}

/// Presage/presence summaries that should keep owning the mission status line.
fn camera_presence_owns_note(raw_summary: &str) -> bool {
    raw_summary.contains("presence=left_frame")
        || raw_summary.contains("presence=camera_obstructed")
}

async fn run_camera_accountability_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
    // Fully deferred — never blocks coaching / Instagram catch at session start.
    tokio::time::sleep(Duration::from_secs(PRESAGE_START_DELAY_SECS)).await;

    // Preference / auth gates before opening the camera.
    {
        let (active, camera_enabled) = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => (true, s.camera_enabled),
                _ => (false, false),
            }
        };
        if !active || !camera_enabled {
            tracing::debug!("camera accountability idle — inactive or preference off");
            return;
        }
        let cfg = app.state::<AppState>().config.lock().clone();
        if crate::auth::load_tokens(&cfg).is_none() {
            tracing::debug!("camera accountability idle — not signed in");
            return;
        }
        if let Err(e) = camera::request_permission_timeout(Duration::from_secs(8)).await {
            tracing::warn!("camera permission: {e}");
            apply_watching_note(
                &app,
                "Camera accountability unavailable · allow Camera permission in System Settings",
            );
            return;
        }
    }

    apply_watching_note(&app, "Camera accountability · checking");

    // Dedicated stop for the grab thread — do NOT flip the shared coach stop
    // (that would also kill the local screen-watch loop).
    let cam_stop = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<camera_live::LiveCameraSample>(8);
    let cam_stop_thread = cam_stop.clone();
    let cam_handle = tokio::task::spawn_blocking(move || {
        camera_live::run_live_camera_loop(cam_stop_thread, |sample| {
            // Drop if the consumer is slow — never block the grab loop forever.
            let _ = tx.try_send(sample);
        })
    });

    let mut presence = LivePresenceInference::with_defaults();
    let mut last_quiet_heartbeat: Option<std::time::Instant> = None;

    while !stop.load(Ordering::SeqCst) && !cam_stop.load(Ordering::SeqCst) {
        let (active, paused, on_pomodoro_break, camera_enabled) = {
            let state = app.state::<AppState>();
            let on_pomodoro_break = state.break_active.load(Ordering::SeqCst);
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => {
                    (true, s.paused, on_pomodoro_break, s.camera_enabled)
                }
                _ => (false, false, false, false),
            }
        };
        if !active || !camera_enabled {
            break;
        }

        let phase = camera_observe_phase(paused, on_pomodoro_break);

        // Quiet phases: sparse JPEG heartbeat so the API refreshes stress cooldown.
        // Live webcam keeps running; PresenceInference stays silent on paused/break.
        if paused || on_pomodoro_break {
            let due = last_quiet_heartbeat
                .map(|t| t.elapsed() >= Duration::from_secs(PRESAGE_QUIET_GAP_SECS))
                .unwrap_or(true);
            if due {
                last_quiet_heartbeat = Some(std::time::Instant::now());
                let cfg = app.state::<AppState>().config.lock().clone();
                match camera_observe::observe_quiet_phase(&cfg, &session_id, phase).await {
                    Ok(resp) => {
                        // Do not let server watching_note overwrite live presence status.
                        if let Some(vitals) = resp.to_vitals() {
                            store_vitals(&app, &vitals, None);
                            let _ = app.emit("vitals-update", &vitals);
                        }
                    }
                    Err(e) if is_quiet_observe_error(&e.message) => {
                        tracing::debug!("camera quiet heartbeat: {e}");
                        if is_auth_observe_error(&e.message) {
                            break;
                        }
                    }
                    Err(e) => tracing::warn!("camera quiet heartbeat: {e}"),
                }
            }
        }

        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(sample)) => {
                let outcome = presence.observe(&sample, phase);
                apply_watching_note(&app, &outcome.watching_note);
                // Lightweight vitals-shaped signal so UI can show presence=… without Presage.
                if matches!(
                    outcome.presence,
                    LivePresenceState::LeftFrame | LivePresenceState::CameraObstructed
                ) {
                    let summary = format!(
                        "presence={} face={} attention={} brightness={:.0}",
                        outcome.presence.as_str(),
                        sample.face_detected,
                        sample.attention,
                        sample.brightness
                    );
                    let mut vitals = VitalsSnapshot::default();
                    vitals.raw_summary = summary;
                    vitals.source = "presence".into();
                    store_vitals(&app, &vitals, Some(&outcome.watching_note));
                    let _ = app.emit("vitals-update", &vitals);
                }
                if let Some(nudge) = outcome.nudge {
                    if !paused && !on_pomodoro_break {
                        deliver_camera_nudge(
                            &app,
                            &nudge.kind,
                            &nudge.text,
                            Some(outcome.presence.as_str()),
                        );
                    }
                }
                flush_pending_camera_presence(&app);
            }
            Ok(None) => {
                // Camera thread ended.
                break;
            }
            Err(_) => {
                // Timeout — check stop / session and continue.
            }
        }
    }

    cam_stop.store(true, Ordering::SeqCst);
    match cam_handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::warn!("live camera loop: {e}");
            apply_watching_note(
                &app,
                "Camera accountability unavailable · live webcam failed (check Camera permission / close other apps using the webcam)",
            );
        }
        Err(e) => tracing::warn!("live camera join: {e}"),
    }
}

/// `active` | `paused` | `break` — server stays silent on the quiet phases.
fn camera_observe_phase(paused: bool, on_pomodoro_break: bool) -> &'static str {
    if on_pomodoro_break {
        "break"
    } else if paused {
        "paused"
    } else {
        "active"
    }
}

/// Whether this nudge kind should open the in-app Accept/Not now break invite.
fn is_suggest_break_nudge(kind: &str) -> bool {
    kind == "suggest_break"
}

/// Server observe → UI (kept for optional sparse Presage vitals upload).
#[allow(dead_code)]
fn apply_observe_response(app: &AppHandle, resp: &camera_observe::ObserveResponse, paused: bool) {
    if let Some(note) = resp.watching_note.as_deref() {
        apply_watching_note(app, note);
    }
    // Presage scalars + presence ladder (VIDEOINPUT) — sole camera accountability path.
    if let Some(vitals) = resp.to_vitals() {
        store_vitals(app, &vitals, resp.watching_note.as_deref());
        let _ = app.emit("vitals-update", &vitals);
    }
    // Quiet phases: server should return null nudges; never surface break invites while paused.
    if paused {
        return;
    }
    let Some(nudge) = resp.nudge.as_ref() else {
        return;
    };
    let kind = if nudge.kind.trim().is_empty() {
        "camera"
    } else {
        nudge.kind.as_str()
    };
    deliver_camera_nudge(app, kind, &nudge.text, resp.presence.as_deref());
}

fn deliver_camera_nudge(app: &AppHandle, kind: &str, text: &str, presence: Option<&str>) {
    if is_suggest_break_nudge(kind) {
        // Break invite → local Accept/Not now card. Never auto-start; never push_prompt
        // (overlay-prompt opens check-in UI which would hide #session-break-suggest).
        // Away-ladder mid step must use reason "away" so the UI does not claim stress.
        let already_breaking = app
            .state::<AppState>()
            .break_active
            .load(Ordering::SeqCst);
        if already_breaking {
            return;
        }
        let reason = suggest_break_reason(text, presence);
        let handle = app.clone();
        let speak_text = text.to_string();
        // Ack side-channel for F4 ladder speak-ack (server re-emits until acked).
        camera_observe::note_nudge_delivered(kind);
        tauri::async_runtime::spawn(async move {
            let _ = crate::break_timer::suggest_break_timer(
                handle.clone(),
                Some(300),
                Some(reason.into()),
            )
            .await;
            let silent = handle
                .state::<AppState>()
                .silent_mode
                .load(Ordering::SeqCst);
            if !silent {
                speak_heads_up(&handle, &speak_text).await;
            }
        });
        return;
    }
    // Desk presence / gaze: bypass the 8s global distraction floor (own short cooldown +
    // one-slot retry queue). Stressed / other camera kinds still use the shared floor.
    if is_camera_presence_kind(kind) {
        deliver_or_queue_camera_presence(app, kind, text);
        return;
    }
    // stressed → overlay + TTS via global floor.
    push_prompt(app, text, kind);
}

fn deliver_or_queue_camera_presence(app: &AppHandle, kind: &str, text: &str) {
    if !claim_camera_presence_slot() {
        // Short camera anti-spam — keep one nudge and retry on the next local tick.
        enqueue_camera_presence(kind, text);
        return;
    }
    // Do not require the global floor to be free; stamp it so a screen nag does not
    // immediately stack voice on top of this presence speak.
    stamp_global_floor();
    push_camera_presence_prompt(app, text, kind);
}

fn flush_pending_camera_presence(app: &AppHandle) {
    let Some(pending) = take_pending_camera_presence() else {
        return;
    };
    if !claim_camera_presence_slot() {
        // Cooldown still hot — put back unless a newer nudge won the slot.
        if let Ok(mut guard) = PENDING_CAMERA_PRESENCE.lock() {
            if guard.is_none() {
                *guard = Some(pending);
            }
        }
        return;
    }
    stamp_global_floor();
    push_camera_presence_prompt(app, &pending.text, &pending.kind);
}

/// Overlay + TTS for a camera presence kind that already claimed its slot.
fn push_camera_presence_prompt(app: &AppHandle, text: &str, kind: &str) {
    let goals = app
        .state::<AppState>()
        .session
        .lock()
        .as_ref()
        .map(|s| s.goals.clone())
        .unwrap_or_default();
    let safe = local_judge::sanitize_coach_line(text, kind, sanitize_distraction_arg(kind), &goals);
    let prompt = CoachPrompt {
        id: Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        text: safe,
        kind: kind.into(),
    };
    deliver_ephemeral(app, &prompt);
    // F4: next observe posts last_nudge_ack so server can consume the ladder rung.
    camera_observe::note_nudge_delivered(kind);
}

fn is_quiet_observe_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("too many")
        || is_auth_observe_error(err)
}

fn is_auth_observe_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("sign in")
}

fn append_screen_log(app: &AppHandle, line: &str) {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock();
    if let Some(session) = guard.as_mut() {
        session.push_screen_log(line);
    }
}

fn apply_watching_note(app: &AppHandle, note: &str) {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock();
    if let Some(session) = guard.as_mut() {
        session.watching_note = note.to_string();
        let snap = session.clone();
        drop(guard);
        let _ = app.emit("session-update", &snap);
    }
}

fn store_vitals(app: &AppHandle, vitals: &VitalsSnapshot, watching_note: Option<&str>) {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock();
    if let Some(session) = guard.as_mut() {
        if vitals.stressed {
            session.stress_spikes += 1;
            session.status = SessionStatusKind::Stressed;
        }
        session.vitals = vitals.clone();
        if let Some(note) = watching_note {
            session.watching_note = note.to_string();
        } else {
            let prefix = if session.screen_enabled {
                "Screen watch active"
            } else {
                "Camera accountability on"
            };
            session.watching_note = if vitals.stressed {
                format!("{prefix} · stress elevated")
            } else {
                format!("{prefix} · steady")
            };
        }
        let snap = session.clone();
        drop(guard);
        let _ = app.emit("session-update", &snap);
    }
}

fn social_distraction_label(result: &CoachVisionResult) -> Option<&'static str> {
    // Prefer URL/app/object signals over model distraction guesses when they conflict
    // (VLM often says "instagram" while objects/frontmost already know it's YouTube).
    let blob = result.objects.join(" ").to_lowercase();
    let object_youtube = blob.split_whitespace().any(|w| w == "youtube")
        || blob.contains("youtube.com")
        || blob.contains("youtu.be");
    let object_instagram = blob.split_whitespace().any(|w| w == "instagram")
        || blob.contains("instagram.com");

    // Trust the explicit distraction field first — never scan coach_line prose
    // (that caused false "Instagram" hits from sentences mentioning the brand).
    if let Some(d) = result.distraction.as_deref() {
        if let Some(label) = static_distraction_label(d) {
            if label == "instagram" && object_youtube && !object_instagram {
                return if result.on_task {
                    None
                } else {
                    Some("youtube")
                };
            }
            if label == "youtube" && result.on_task {
                return None;
            }
            return Some(label);
        }
    }
    const SITES: &[(&str, &str)] = &[
        // YouTube before Instagram so object tokens prefer the real surface.
        ("youtube", "youtube"),
        ("instagram", "instagram"),
        ("tiktok", "tiktok"),
        ("twitter", "twitter"),
        ("facebook", "facebook"),
        ("reddit", "reddit"),
        ("discord", "discord"),
        ("snapchat", "snapchat"),
        ("netflix", "netflix"),
        ("twitch", "twitch"),
        ("gmail", "email"),
        ("outlook", "email"),
        ("amazon", "shopping"),
        ("shopping", "shopping"),
        ("imessage", "texting"),
        ("whatsapp", "texting"),
        ("telegram", "texting"),
        ("texting", "texting"),
    ];
    for (needle, label) in SITES {
        if blob.split_whitespace().any(|w| w == *needle) || blob == *needle {
            if *label == "instagram" && object_youtube && !object_instagram {
                continue;
            }
            if *label == "youtube" && result.on_task {
                return None;
            }
            return Some(*label);
        }
    }
    None
}

fn static_distraction_label(label: &str) -> Option<&'static str> {
    match label {
        "texting" | "messages" | "whatsapp" | "telegram" | "imessage" => Some("texting"),
        "phone" | "cellphone" | "smartphone" => Some("phone"),
        "instagram" => Some("instagram"),
        "youtube" => Some("youtube"),
        "tiktok" => Some("tiktok"),
        "discord" => Some("discord"),
        "email" | "gmail" | "outlook" => Some("email"),
        "shopping" | "amazon" => Some("shopping"),
        "twitter" => Some("twitter"),
        "facebook" => Some("facebook"),
        "reddit" => Some("reddit"),
        "snapchat" => Some("snapchat"),
        "netflix" => Some("netflix"),
        "twitch" => Some("twitch"),
        _ => None,
    }
}

fn is_camera_source(source: &str) -> bool {
    source == "presage" || source == "presence"
}

async fn apply_coach_tick(
    app: &AppHandle,
    result: &CoachVisionResult,
    last_vitals: &mut VitalsSnapshot,
) {
    // Prefer real Presage/presence readings; otherwise soft-fallback from vision stress cue.
    if !is_camera_source(&last_vitals.source) {
        *last_vitals = presage::fallback_vitals(result.stress_cue);
    } else if result.stress_cue {
        last_vitals.stressed = true;
        last_vitals.focus_ok = false;
    }

    // Hard social/shopping/etc. always off-task. YouTube is contextual — trust the judge.
    let social = match social_distraction_label(result) {
        Some("youtube") if result.on_task => None,
        other => other,
    };
    let mut on_task = result.on_task && social.is_none();
    let mut distraction = result.distraction.clone();
    if let Some(label) = social {
        on_task = false;
        distraction = Some(label.to_string());
    } else if result.on_task {
        distraction = None;
    }

    let objects_lower: Vec<String> = result.objects.iter().map(|o| o.to_lowercase()).collect();
    // Local VLM often puts the label in `distraction` while `objects` stays ["screen"].
    let has_phone = objects_lower.iter().any(|o| o.contains("phone"))
        || distraction
            .as_deref()
            .is_some_and(|d| d.to_lowercase().contains("phone"));
    let has_calc = objects_lower
        .iter()
        .any(|o| o.contains("calculator") || o.contains("calc"))
        || distraction.as_deref().is_some_and(|d| {
            let d = d.to_lowercase();
            d.contains("calculator") || d.contains("calc")
        });
    if has_phone && !has_calc {
        on_task = false;
        if distraction.is_none() {
            distraction = Some("phone".into());
        }
    }

    let status = if result.needs_help && on_task {
        SessionStatusKind::NeedsHelp
    } else if last_vitals.stressed || result.stress_cue {
        SessionStatusKind::Stressed
    } else if !on_task {
        SessionStatusKind::Distracted
    } else {
        SessionStatusKind::OnTask
    };

    let mut prompt_text = result.coach_line.clone();
    if has_phone && !has_calc {
        prompt_text.clear(); // force a fresh local line below
    } else if has_calc && on_task && prompt_text.to_lowercase().contains("phone") {
        prompt_text = "Calculator is fair game — keep working the problem.".into();
    }

    let wants_prompt = !on_task || result.needs_help || last_vitals.stressed || has_phone;
    if on_task {
        clear_distraction_episode();
    }
    let nag_key = distraction
        .as_deref()
        .or(social)
        .unwrap_or(if on_task { "on_task" } else { "off_task" });
    let cooldown = if on_task {
        PROMPT_COOLDOWN_SECS
    } else {
        EPHEMERAL_FLOOR_SECS
    };
    let prompt = if wants_prompt && claim_ephemeral_slot_keyed(cooldown, Some(nag_key)) {
        let nag_n = distraction_episode_nags(nag_key).max(1);
        let cfg = app.state::<AppState>().config.lock().clone();
        let goals = app
            .state::<AppState>()
            .session
            .lock()
            .as_ref()
            .map(|s| s.goals.clone())
            .unwrap_or_default();
        let kind = if has_phone && !has_calc {
            "distracted"
        } else if result.needs_help {
            "encourage"
        } else if on_task {
            "on_task"
        } else {
            "distracted"
        };
        let label = if has_phone && !has_calc {
            "phone"
        } else {
            distraction.as_deref().or(social).unwrap_or("off-task")
        };
        let detail = result.objects.join(" · ");
        let mut text = local_judge::freshen_coach_line(
            &cfg,
            &prompt_text,
            kind,
            &goals,
            label,
            &detail,
            nag_n,
        )
        .await;
        // Strip model “take a break” copy on distraction nags (stress breaks use suggest_break).
        text = local_judge::sanitize_coach_line(&text, kind, label, &goals);
        if (last_vitals.stressed || result.stress_cue)
            && !text.to_lowercase().contains("breath")
        {
            text = format!("{text} One slow breath, then back to it.");
        }
        Some(CoachPrompt {
            id: Uuid::new_v4().to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            text,
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
            if on_task {
                session.on_task_ticks += 1;
            }
            if (last_vitals.stressed || result.stress_cue) && !is_camera_source(&last_vitals.source) {
                // Presage / camera path increments stress_spikes when a reading arrives.
                session.stress_spikes += 1;
            }
            if let Some(d) = &distraction {
                session.bump_distraction(d);
            }
            session.status = status;
            // Prefer Presage/presence vitals over screen-coach soft fallbacks.
            if is_camera_source(&last_vitals.source) || !is_camera_source(&session.vitals.source) {
                session.vitals = last_vitals.clone();
            } else {
                *last_vitals = session.vitals.clone();
            }
            if on_task {
                if !camera_presence_owns_note(&session.vitals.raw_summary) {
                    session.watching_note = "Watching your screen".into();
                }
            } else {
                let d = distraction.as_deref().unwrap_or("off-task");
                let nags = distraction_episode_nags(d);
                session.watching_note = if nags >= MAX_NAGS_PER_EPISODE {
                    format!("Distracted · {d} (quiet until you switch back)")
                } else {
                    format!("Distracted · {d}")
                };
            }
            // Ephemeral coach nags stay off the in-app feed — overlay + voice only.
            session.clone()
        } else {
            return;
        }
    };

    let _ = app.emit("session-update", &snapshot);
    if let Some(p) = prompt {
        deliver_ephemeral(app, &p);
    }
}

fn push_prompt(app: &AppHandle, text: &str, kind: &str) {
    push_ephemeral(app, text, kind);
}

/// Quiet positive feedback when they return to task — ding only, no toast/TTS.
fn play_back_on_task_ding(app: &AppHandle) {
    clear_distraction_episode();
    let silent = app.state::<AppState>().silent_mode.load(Ordering::SeqCst);
    if silent {
        return;
    }
    // Own cooldown — must not share/block the nag voice floor.
    if !claim_ding_slot() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let play = tokio::task::spawn_blocking(waypoint_voice::play_positive_ding).await;
        if let Ok(Err(e)) = play {
            tracing::debug!("back-on-task ding: {e}");
        }
    });
}

/// Distraction label for sanitize — empty for camera/status kinds so tags like
/// `left_desk` are never spoken as if they were Instagram/Discord.
fn sanitize_distraction_arg(kind: &str) -> &str {
    match kind {
        "left_desk"
        | "left_desk_pause"
        | "look_back"
        | "welcome_back"
        | "camera_obstructed"
        | "camera"
        | "suggest_break"
        | "stressed"
        | "watching"
        | "encourage"
        | "on_task" => "",
        other => other,
    }
}

fn push_ephemeral(app: &AppHandle, text: &str, kind: &str) {
    // Praise / watching lines share the same single-slot gate — never stack over a nag.
    if !claim_ephemeral_slot_keyed(EPHEMERAL_FLOOR_SECS, Some(kind)) {
        return;
    }
    // Last line of defense: never toast schema echoes like “one short sentence”.
    let goals = app
        .state::<AppState>()
        .session
        .lock()
        .as_ref()
        .map(|s| s.goals.clone())
        .unwrap_or_default();
    let safe = local_judge::sanitize_coach_line(text, kind, sanitize_distraction_arg(kind), &goals);
    let prompt = CoachPrompt {
        id: Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        text: safe,
        kind: kind.into(),
    };
    deliver_ephemeral(app, &prompt);
}

fn deliver_ephemeral(app: &AppHandle, prompt: &CoachPrompt) {
    // Last defense: never toast “take a break” for distraction / off-task kinds.
    let goals = app
        .state::<AppState>()
        .session
        .lock()
        .as_ref()
        .map(|s| s.goals.clone())
        .unwrap_or_default();
    let safe_text = local_judge::sanitize_coach_line(
        &prompt.text,
        &prompt.kind,
        sanitize_distraction_arg(&prompt.kind),
        &goals,
    );
    let prompt = CoachPrompt {
        id: prompt.id.clone(),
        at: prompt.at.clone(),
        text: safe_text,
        kind: prompt.kind.clone(),
    };
    // Show the toast first — never wait on TTS teardown before the popup.
    overlay::show_prompt(app, &prompt);
    // Camera accountability always speaks — silent_mode only mutes screen/coach chatter.
    let silent = app.state::<AppState>().silent_mode.load(Ordering::SeqCst);
    let force_camera_voice = is_camera_presence_kind(&prompt.kind);
    if silent && !force_camera_voice {
        return;
    }
    // Prefer xAI / Grok TTS via API proxy; fall back to local macOS `say`.
    let text = prompt.text.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        speak_heads_up(&handle, &text).await;
    });
}

/// Study heads-up / nudge spoken output only.
/// Tries `POST /v1/voice/tts` (xAI key stays on the API host), then macOS `say`.
async fn speak_heads_up(app: &AppHandle, text: &str) {
    let cfg = app.state::<AppState>().config.lock().clone();
    match crate::api::fetch_heads_up_tts(&cfg, text).await {
        Ok(audio) => {
            let bytes = audio.bytes;
            let ext = audio.extension;
            let play = tokio::task::spawn_blocking(move || {
                waypoint_voice::play_audio_bytes(&bytes, &ext)
            })
            .await;
            match play {
                Ok(Ok(())) => {
                    tracing::debug!("heads-up TTS: xAI/Grok voice");
                    return;
                }
                Ok(Err(e)) => tracing::warn!("heads-up xAI playback failed, falling back: {e}"),
                Err(e) => tracing::warn!("heads-up xAI playback join failed, falling back: {e}"),
            }
        }
        Err(e) => {
            tracing::debug!("heads-up xAI TTS unavailable ({e}); using local say");
        }
    }
    if let Err(e) = waypoint_voice::speak(text) {
        tracing::warn!("voice speak: {e}");
    }
}

async fn finish_session(app: &AppHandle, session_id: &str) {
    waypoint_voice::stop_speaking();
    overlay::hide(app);
    // Natural end does not clear companion history, but snapshot before the session goes away
    // so the note still has what the user told the agent.
    let utterances = {
        let state = app.state::<AppState>();
        crate::companion::user_utterances(&state)
    };
    let summary = {
        let state = app.state::<AppState>();
        let open_pause = state.pause_started.lock().take();
        let mut guard = state.session.lock();
        // Never summarize a different (newer) session, or one `stop_lock_in` already took.
        let matches = matches!(
            guard.as_ref(),
            Some(session) if session.id == session_id && session.active
        );
        if !matches {
            None
        } else if let Some(session) = LockInSession::take_finished(&mut guard, open_pause) {
            let mut summary = session.summarize();
            summary.notes_pending =
                crate::maybe_spawn_session_note(app, &session, utterances);
            Some(summary)
        } else {
            None
        }
    };
    if let Some(summary) = summary {
        // Natural completions must reach study memory too (stop_lock_in only covers early end).
        crate::persist_session_summary(app, &summary);
        let _ = app.emit("session-ended", &summary);
    }
}

/// Signal coach / camera loops to exit. Does **not** mutate or clear `AppState.session` —
/// that belongs to `stop_lock_in` / `finish_session` so we never leave a half-dead
/// session (`active=false` but still present) that the UI can treat as still running.
pub fn stop_coach(app: &AppHandle) {
    waypoint_voice::stop_speaking();
    overlay::hide(app);
    let state = app.state::<AppState>();
    let stop_guard = state.coach_stop.lock();
    if let Some(stop) = stop_guard.as_ref() {
        stop.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod distraction_label_tests {
    use super::*;
    use crate::gemini::CoachVisionResult;

    #[test]
    fn youtube_objects_override_instagram_model_distraction() {
        let result = CoachVisionResult {
            on_task: false,
            objects: vec!["youtube".into(), "Shorts".into()],
            distraction: Some("instagram".into()),
            needs_help: false,
            stress_cue: false,
            coach_line: "Close Instagram and finish your BIOMG quiz.".into(),
            modality: Some("computer".into()),
        };
        assert_eq!(social_distraction_label(&result), Some("youtube"));
    }

    #[test]
    fn youtube_distraction_stays_youtube() {
        let result = CoachVisionResult {
            on_task: false,
            objects: vec!["youtube".into()],
            distraction: Some("youtube".into()),
            needs_help: false,
            stress_cue: false,
            coach_line: "This YouTube isn’t helping — switch back.".into(),
            modality: Some("computer".into()),
        };
        assert_eq!(social_distraction_label(&result), Some("youtube"));
    }
}

#[cfg(test)]
mod camera_observe_timing_tests {
    use super::*;

    #[test]
    fn quiet_heartbeat_stays_sparse() {
        assert!(
            PRESAGE_QUIET_GAP_SECS >= 60,
            "quiet gap {}s should stay sparse vs the 3/75s observe bucket",
            PRESAGE_QUIET_GAP_SECS
        );
    }

    #[test]
    fn live_presence_owns_camera_with_short_start_delay() {
        assert!(
            PRESAGE_START_DELAY_SECS <= 30,
            "start delay {}s should be a short settle, not a long deferral",
            PRESAGE_START_DELAY_SECS
        );
        // Live loop analyzes at 2 Hz; optional vitals clip (if re-enabled) stays sparse.
        assert!(PRESAGE_VITALS_GAP_SECS >= 90);
        assert!((camera_live::ANALYSIS_HZ - 2.0).abs() < f64::EPSILON);
        assert!((camera_live::MIN_PERSISTENCE_S - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn observe_phase_prefers_break_over_paused() {
        assert_eq!(camera_observe_phase(false, false), "active");
        assert_eq!(camera_observe_phase(true, false), "paused");
        assert_eq!(camera_observe_phase(true, true), "break");
        assert_eq!(camera_observe_phase(false, true), "break");
    }

    #[test]
    fn stress_suggest_break_is_invite_not_overlay_kind() {
        assert!(is_suggest_break_nudge("suggest_break"));
        assert!(!is_suggest_break_nudge("stressed"));
        assert!(!is_suggest_break_nudge("left_desk"));
        assert!(!is_suggest_break_nudge("left_desk_pause"));
        assert!(!is_suggest_break_nudge("look_back"));
        assert!(!is_suggest_break_nudge("welcome_back"));
        assert!(!is_suggest_break_nudge("camera_obstructed"));
    }

    #[test]
    fn camera_presence_kind_classification_includes_look_back() {
        assert!(is_camera_presence_kind("left_desk"));
        assert!(is_camera_presence_kind("left_desk_pause"));
        assert!(is_camera_presence_kind("look_back"));
        assert!(is_camera_presence_kind("welcome_back"));
        assert!(is_camera_presence_kind("camera_obstructed"));
        assert!(!is_camera_presence_kind("stressed"));
        assert!(!is_camera_presence_kind("suggest_break"));
        assert!(!is_camera_presence_kind("instagram.com"));
    }

    #[test]
    fn away_ladder_suggest_break_reason_is_not_stress() {
        assert_eq!(
            suggest_break_reason(
                "Still away — optional five-minute break so you know when to return?",
                Some("left_frame"),
            ),
            "away"
        );
        assert_eq!(
            suggest_break_reason("Still away — optional five-minute break?", None),
            "away"
        );
        assert_eq!(
            suggest_break_reason("Feeling tense — optional five-minute break?", None),
            "stress"
        );
        assert_eq!(
            suggest_break_reason(
                "Feeling tense — optional five-minute break?",
                Some("present"),
            ),
            "stress"
        );
    }

    #[test]
    fn camera_presence_kinds_bypass_global_floor_not_episode_cap() {
        // Regression: left_desk used claim_distraction_nag / global floor and could be
        // silenced forever after a screen nag (server already advanced ladderSpoken).
        reset_ephemeral_cooldown();
        assert!(claim_global_floor());
        // Global floor is hot — presence kinds still claim via camera-specific slot.
        assert!(claim_ephemeral_slot_keyed(8, Some("left_desk")));
        // Immediate re-claim blocked by camera cooldown only (not episode gap/cap).
        assert!(!claim_ephemeral_slot_keyed(8, Some("look_back")));
        reset_ephemeral_cooldown();
        assert!(claim_ephemeral_slot_keyed(8, Some("look_back")));
        assert!(!claim_ephemeral_slot_keyed(8, Some("left_desk")));
        reset_ephemeral_cooldown();
        assert!(claim_ephemeral_slot_keyed(8, Some("camera_obstructed")));
        reset_ephemeral_cooldown();
        assert!(claim_ephemeral_slot_keyed(8, Some("welcome_back")));
        // stressed still uses the shared global floor (not camera bypass).
        reset_ephemeral_cooldown();
        assert!(claim_global_floor());
        assert!(!claim_ephemeral_slot_keyed(8, Some("stressed")));
    }

    #[test]
    fn camera_presence_queues_when_camera_cooldown_blocks() {
        reset_ephemeral_cooldown();
        assert!(claim_camera_presence_slot());
        enqueue_camera_presence("left_desk", "You've stepped away.");
        // Cooldown still held — take would leave it for flush to re-queue.
        let pending = take_pending_camera_presence();
        assert!(pending.is_some());
        let pending = pending.unwrap();
        assert_eq!(pending.kind, "left_desk");
        assert!(pending.text.contains("stepped away"));
    }
}
