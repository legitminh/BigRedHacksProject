use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::capture::{self, camera, frontmost};
use crate::gemini::CoachVisionResult;
use crate::local_judge;
use crate::local_vision;
use crate::overlay;
use crate::presage::{self, PresageClient, VitalsSnapshot};
use crate::session::{CoachPrompt, SessionStatusKind};
use crate::AppState;

/// Poll for app/tab switches — free OS signals, so stay snappy.
const LOCAL_TICK_SECS: u64 = 1;
/// Apple Vision OCR on a small window capture (cheap, no cloud).
const OCR_TICK_SECS: u64 = 3;
/// Rare tiny local VLM (moondream) when OCR/OS signals are inconclusive.
const LOCAL_VLM_TICK_SECS: u64 = 45;
/// Ambiguous context judgments per session (local model only — no Gemini in lock-in).
const MAX_CONTEXT_JUDGMENTS_PER_SESSION: u32 = 120;
/// Soft coach lines (help / stress) — don't spam.
const PROMPT_COOLDOWN_SECS: i64 = 14;
/// Absolute floor between any popup/voice (including praise).
const EPHEMERAL_FLOOR_SECS: i64 = 8;
/// Max spoken/popup reminders for one continuous distraction (e.g. one Instagram stay).
const MAX_NAGS_PER_EPISODE: u32 = 3;
/// Seconds to wait after nag 1 → nag 2, then after nag 2 → nag 3.
const EPISODE_GAP_AFTER_FIRST_SECS: i64 = 14;
const EPISODE_GAP_AFTER_SECOND_SECS: i64 = 22;
const PRESAGE_CLIP_SECS: u64 = 22;
const PRESAGE_FPS: u32 = 12;
const PRESAGE_GAP_SECS: u64 = 70;
/// Don't touch the camera until coaching has already started.
const PRESAGE_START_DELAY_SECS: u64 = 90;
/// Let the student settle before distraction tracking / nags begin.
const TRACKING_WARMUP_SECS: i64 = 15;
/// Fixed opener — never LLM/system-prompt text (tiny models regurgitate prompts).
const SESSION_OPENER: &str = "You're locked in. I'll check in if you drift.";

/// Shared across local watch ticks so OCR/VLM don't re-fire the same nag.
static LAST_EPHEMERAL_AT: Mutex<Option<chrono::DateTime<chrono::Utc>>> = Mutex::new(None);

#[derive(Clone)]
struct DistractionEpisode {
    key: String,
    nags: u32,
    last_nag_at: chrono::DateTime<chrono::Utc>,
}

static DISTRACTION_EPISODE: Mutex<Option<DistractionEpisode>> = Mutex::new(None);

fn reset_ephemeral_cooldown() {
    if let Ok(mut guard) = LAST_EPHEMERAL_AT.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = DISTRACTION_EPISODE.lock() {
        *guard = None;
    }
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

fn claim_ephemeral_slot_keyed(cooldown_secs: i64, key: Option<&str>) -> bool {
    let _ = cooldown_secs;
    // Non-distraction lines (praise / watching) — global floor only.
    if let Some(k) = key {
        if matches!(k, "encourage" | "watching" | "on_task") {
            if matches!(k, "encourage" | "on_task") {
                clear_distraction_episode();
            }
            return claim_global_floor();
        }
        return claim_distraction_nag(k);
    }
    claim_global_floor()
}

pub fn spawn_coach_loop(app: AppHandle, session_id: String, use_camera: bool, use_presage: bool) {
    reset_ephemeral_cooldown();
    {
        let greet_app = app.clone();
        tauri::async_runtime::spawn(async move {
            // Short fixed phrase only — never compose/LLM (avoids reading the prompt aloud).
            push_prompt(&greet_app, SESSION_OPENER, "watching");
        });
    }

    {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.watching_note = format!(
                "Settling in — tracking starts in {TRACKING_WARMUP_SECS}s…"
            );
            session.status = SessionStatusKind::OnTask;
            let snap = session.clone();
            drop(guard);
            let _ = app.emit("session-update", &snap);
        }
    }

    // Probe Ollama in the background so the UI can show if on-device judging is live.
    {
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
                    session.watching_note =
                        format!("Local watch · {ocr_line} · {text_line} · {vlm_line}");
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
        run_local_watch_loop(local_app, local_id, local_stop).await;
    });

    if use_camera && use_presage {
        let vitals_app = app;
        let vitals_id = session_id;
        let vitals_stop = stop;
        tauri::async_runtime::spawn(async move {
            run_presage_loop(vitals_app, vitals_id, vitals_stop).await;
        });
    }
}

/// Primary interrupter: reacts to app/tab open/switch events; YouTube gets text context.
async fn run_local_watch_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
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
        let (active, remaining, goals, paused) = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => {
                    last_vitals = s.vitals.clone();
                    (true, s.remaining_secs(), s.goals.clone(), s.paused)
                }
                _ => (false, 0, String::new(), false),
            }
        };
        if !active || remaining <= 0 {
            // Only a natural timer expiry finishes here. A user "End" goes through
            // `stop_lock_in`, which owns the summary — emitting `session-ended` too would
            // double-count the mission and could report pre-pause-fold elapsed time.
            if active && remaining <= 0 && !stop.load(Ordering::SeqCst) {
                finish_session(&app, &session_id).await;
            }
            break;
        }
        if paused {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        let now = chrono::Utc::now();
        if now < tracking_ready_at {
            let left = (tracking_ready_at - now).num_seconds().max(0);
            {
                let state = app.state::<AppState>();
                let mut guard = state.session.lock();
                if let Some(session) = guard.as_mut() {
                    session.watching_note = format!("Settling in — tracking in {left}s…");
                    session.status = SessionStatusKind::OnTask;
                    let snap = session.clone();
                    drop(guard);
                    let _ = app.emit("session-update", &snap);
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
            // Seed fingerprint so the first post-warmup focus doesn't false-nag.
            if let Ok(Ok(info)) = tokio::task::spawn_blocking(frontmost::frontmost_info).await {
                last_fingerprint = info.fingerprint();
                last_judged_fingerprint = last_fingerprint.clone();
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
                    // Key by label only — detail churn was resetting cooldown and stacking voice.
                    let fp = format!("hard:{}", hit.label);
                    last_fingerprint = fp;
                    let cfg = app.state::<AppState>().config.lock().clone();
                    let nag_n = distraction_episode_nags(hit.label).saturating_add(1);
                    let coach_line = local_judge::compose_coach_line(
                        &cfg,
                        "distracted",
                        &goals,
                        hit.label,
                        &hit.detail,
                        nag_n,
                    )
                    .await;
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
                    let fp = info.fingerprint();
                    let switched = fp != last_fingerprint;
                    last_fingerprint = fp.clone();

                    // 1) Instant keyword guess for obvious study vs entertainment.
                    //    YouTube/video must never idle on "Checking…" — unclear = off-task.
                    let mut heuristic = frontmost::local_context_guess(kind, &page_text, &goals);
                    if heuristic.is_none() && matches!(kind, "youtube" | "video") {
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
                frontmost::FocusEvent::Clear(info) => {
                    let fp = info.fingerprint();
                    last_fingerprint = fp;
                    if was_distracted {
                        was_distracted = false;
                        play_back_on_task_ding(&app);
                    }
                    mark_local_on_task(&app, &info);
                    // Always show what we think is focused so detection failures are obvious.
                    let state = app.state::<AppState>();
                    let mut guard = state.session.lock();
                    if let Some(session) = guard.as_mut() {
                        if matches!(session.status, SessionStatusKind::OnTask) {
                            let ocr_bit = if last_ocr_snippet.is_empty() {
                                String::new()
                            } else {
                                format!(" · OCR: {}", truncate_note(&last_ocr_snippet, 48))
                            };
                            session.watching_note =
                                format!("On task · {}{ocr_bit}", info.summary());
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
            let ocr_result = tokio::task::spawn_blocking(local_vision::read_screen_ocr).await;
            match ocr_result {
                Ok(Ok(reading)) => {
                    last_ocr_snippet = reading
                        .ocr_text
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    // OCR labels are host/chrome-strong only — ignore brand word mentions.
                    if let Some(label) = reading.labels.first().copied() {
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
                                detail: truncate_note(&reading.ocr_text.replace('\n', " "), 64),
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
                                    if j.confidence >= local_judge::min_confidence()
                                        && !j.result.on_task
                                    {
                                        apply_coach_tick(&app, &j.result, &mut last_vitals).await;
                                        was_distracted = true;
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
    session.watching_note = format!("Watching full screen · {}", info.summary());
    let snap = session.clone();
    drop(guard);
    let _ = app.emit("session-update", &snap);
}

async fn run_presage_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
    // Fully deferred — never blocks coaching / Instagram catch at session start.
    tokio::time::sleep(Duration::from_secs(PRESAGE_START_DELAY_SECS)).await;
    let mut last_stress_prompt = chrono::Utc::now() - chrono::Duration::minutes(10);

    while !stop.load(Ordering::SeqCst) {
        let (active, paused) = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id && s.remaining_secs() > 25 => {
                    (true, s.paused)
                }
                _ => (false, false),
            }
        };
        if !active {
            break;
        }
        if paused {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        let dir = match capture::temp_session_dir(&session_id) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("wellness temp dir: {e}");
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
                // Only interrupt when stress is up — and at most every few minutes.
                if vitals.stressed {
                    let now = chrono::Utc::now();
                    if now.signed_duration_since(last_stress_prompt).num_seconds() >= 180 {
                        last_stress_prompt = now;
                        push_prompt(
                            &app,
                            "Stress looks elevated — one slow breath, then back to the work on screen.",
                            "stressed",
                        );
                    }
                }
            }
        }

        // Wait between silent clips; wake early if session ends.
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
        Err(_e) => {
            let _ = app.emit("coach-error", "Wellness check unavailable right now.");
            return None;
        }
    };
    let id = match client.queue_video_hr_rr(video_path).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("presage upload: {e}");
            let _ = app.emit("coach-error", "Wellness check couldn’t start. Trying again later.");
            return None;
        }
    };
    match client.retrieve_result(&id, 45).await {
        Ok(data) => Some(PresageClient::vitals_from_result(&data)),
        Err(e) => {
            tracing::warn!("presage retrieve: {e}");
            let _ = app.emit("coach-error", "Wellness check timed out. Continuing without it.");
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

fn social_distraction_label(result: &CoachVisionResult) -> Option<&'static str> {
    // Trust the explicit distraction field first — never scan coach_line prose
    // (that caused false "Instagram" hits from sentences mentioning the brand).
    if let Some(d) = result.distraction.as_deref() {
        if let Some(label) = static_distraction_label(d) {
            if label == "youtube" && result.on_task {
                return None;
            }
            return Some(label);
        }
    }
    let blob = result.objects.join(" ").to_lowercase();
    const SITES: &[(&str, &str)] = &[
        ("instagram", "instagram"),
        ("tiktok", "tiktok"),
        ("twitter", "twitter"),
        ("facebook", "facebook"),
        ("reddit", "reddit"),
        ("discord", "discord"),
        ("snapchat", "snapchat"),
        ("youtube", "youtube"),
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

async fn apply_coach_tick(
    app: &AppHandle,
    result: &CoachVisionResult,
    last_vitals: &mut VitalsSnapshot,
) {
    // Prefer real Presage readings; otherwise soft-fallback from vision stress cue.
    if last_vitals.source != "presage" {
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
    let has_phone = objects_lower.iter().any(|o| o.contains("phone"));
    let has_calc = objects_lower
        .iter()
        .any(|o| o.contains("calculator") || o.contains("calc"));
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
            if (last_vitals.stressed || result.stress_cue) && last_vitals.source != "presage" {
                // Presage path increments stress_spikes when a reading arrives.
                session.stress_spikes += 1;
            }
            if let Some(d) = &distraction {
                session.bump_distraction(d);
            }
            session.status = status;
            if last_vitals.source == "presage" || session.vitals.source != "presage" {
                session.vitals = last_vitals.clone();
            } else {
                *last_vitals = session.vitals.clone();
            }
            session.watching_note = if on_task {
                "Watching your screen".into()
            } else {
                let d = distraction.as_deref().unwrap_or("off-task");
                let nags = distraction_episode_nags(d);
                if nags >= MAX_NAGS_PER_EPISODE {
                    format!("Distracted · {d} (quiet until you switch back)")
                } else {
                    format!("Distracted · {d}")
                }
            };
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
    if !claim_global_floor() {
        return;
    }
    let silent = app.state::<AppState>().silent_mode.load(Ordering::SeqCst);
    if silent {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let play = tokio::task::spawn_blocking(waypoint_voice::play_positive_ding).await;
        if let Ok(Err(e)) = play {
            tracing::debug!("back-on-task ding: {e}");
        }
    });
}

fn push_ephemeral(app: &AppHandle, text: &str, kind: &str) {
    // Praise / watching lines share the same single-slot gate — never stack over a nag.
    if !claim_ephemeral_slot_keyed(EPHEMERAL_FLOOR_SECS, Some(kind)) {
        return;
    }
    let prompt = CoachPrompt {
        id: Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        text: text.into(),
        kind: kind.into(),
    };
    deliver_ephemeral(app, &prompt);
}

fn deliver_ephemeral(app: &AppHandle, prompt: &CoachPrompt) {
    // Show the toast first — never wait on TTS teardown before the popup.
    overlay::show_prompt(app, prompt);
    let silent = app.state::<AppState>().silent_mode.load(Ordering::SeqCst);
    if silent {
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
    let summary = {
        let state = app.state::<AppState>();
        let open_pause = state.pause_started.lock().take();
        let mut guard = state.session.lock();
        match guard.as_mut() {
            // Never summarize a different (newer) session, or one `stop_lock_in` already took.
            Some(session) if session.id == session_id && session.active => {
                session.finalize_open_pause(open_pause);
                session.active = false;
                session.paused = false;
                let summary = session.summarize();
                // Same lifecycle as `stop_lock_in`: clear the finished session.
                *guard = None;
                Some(summary)
            }
            _ => None,
        }
    };
    if let Some(summary) = summary {
        // Natural completions must reach study memory too (stop_lock_in only covers early end).
        crate::persist_session_summary(app, &summary);
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
