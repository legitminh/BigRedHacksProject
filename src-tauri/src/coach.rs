use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::capture::{self, camera, frontmost, screen};
use crate::gemini::{self, CoachVisionResult, GeminiClient};
use crate::overlay;
use crate::presage::{self, PresageClient, VitalsSnapshot};
use crate::session::{CoachPrompt, SessionStatusKind};
use crate::AppState;

/// Poll for app/tab switches — free OS signals, so stay snappy.
const LOCAL_TICK_SECS: u64 = 1;
/// Full-desktop Gemini when on-task. Free tier ~250 RPD; ~50s ≈ 70/hr with chat headroom.
const GEMINI_TICK_ON_TASK_SECS: u64 = 50;
/// While local already knows you're distracted, vision is only a rare double-check.
const GEMINI_TICK_OFF_TASK_SECS: u64 = 90;
/// Cap vision calls per lock-in so a long session can't burn the daily quota.
const MAX_VISION_PER_SESSION: u32 = 100;
/// Text context judgments (YouTube titles) are cheap — still cap per session.
const MAX_CONTEXT_JUDGMENTS_PER_SESSION: u32 = 80;
const RATE_LIMIT_COOLDOWN_SECS: u64 = 120;
/// Soft coach lines (help / stress) — don't spam.
const PROMPT_COOLDOWN_SECS: i64 = 14;
/// Off-task nags — matches overlay display so the next toast lands as the last fades.
const SOCIAL_PROMPT_COOLDOWN_SECS: i64 = 7;
const PRESAGE_CLIP_SECS: u64 = 22;
const PRESAGE_FPS: u32 = 12;
const PRESAGE_GAP_SECS: u64 = 70;
/// Don't touch the camera until coaching has already started.
const PRESAGE_START_DELAY_SECS: u64 = 90;

/// Shared across local + Gemini loops so both don't fire the same nag.
static LAST_EPHEMERAL_AT: Mutex<Option<chrono::DateTime<chrono::Utc>>> = Mutex::new(None);

fn reset_ephemeral_cooldown() {
    if let Ok(mut guard) = LAST_EPHEMERAL_AT.lock() {
        *guard = None;
    }
}

/// Returns true if enough time has passed to show another overlay/voice nag.
fn claim_ephemeral_slot(cooldown_secs: i64) -> bool {
    let now = chrono::Utc::now();
    let Ok(mut guard) = LAST_EPHEMERAL_AT.lock() else {
        return true;
    };
    if let Some(last) = *guard {
        if now.signed_duration_since(last).num_seconds() < cooldown_secs {
            return false;
        }
    }
    *guard = Some(now);
    true
}

pub fn spawn_coach_loop(app: AppHandle, session_id: String, use_camera: bool, use_presage: bool) {
    reset_ephemeral_cooldown();
    push_prompt(
        &app,
        "I'm watching your full screen against your goals — off-task apps and sites get called out.",
        "watching",
    );

    {
        let state = app.state::<AppState>();
        let mut guard = state.session.lock();
        if let Some(session) = guard.as_mut() {
            session.watching_note =
                "Watching full screen · wellness runs later in the background".into();
            session.status = SessionStatusKind::OnTask;
            let snap = session.clone();
            drop(guard);
            let _ = app.emit("session-update", &snap);
        }
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

    let vision_app = app.clone();
    let vision_id = session_id.clone();
    let vision_stop = stop.clone();
    tauri::async_runtime::spawn(async move {
        run_gemini_loop(vision_app, vision_id, vision_stop).await;
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

    while !stop.load(Ordering::SeqCst) {
        let (active, remaining, goals) = {
            let state = app.state::<AppState>();
            let guard = state.session.lock();
            match guard.as_ref() {
                Some(s) if s.active && s.id == session_id => {
                    last_vitals = s.vitals.clone();
                    (true, s.remaining_secs(), s.goals.clone())
                }
                _ => (false, 0, String::new()),
            }
        };
        if !active || remaining <= 0 {
            finish_session(&app).await;
            break;
        }

        let event = tokio::task::spawn_blocking(frontmost::evaluate_focus)
            .await
            .ok()
            .and_then(|r| r.ok());

        if let Some(event) = event {
            match event {
                frontmost::FocusEvent::Hard(hit) => {
                    let fp = format!("hard:{}:{}", hit.label, hit.detail);
                    let switched = fp != last_fingerprint;
                    last_fingerprint = fp;
                    if switched {
                        // Opening Discord / Instagram / shopping → interrupt immediately.
                        reset_ephemeral_cooldown();
                    }
                    let result = CoachVisionResult {
                        on_task: false,
                        objects: vec![hit.label.into(), hit.detail.clone()],
                        distraction: Some(hit.label.into()),
                        needs_help: false,
                        stress_cue: last_vitals.stressed,
                        coach_line: frontmost::distraction_coach_line(&hit),
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

                    // Instant local guess for obvious study vs entertainment titles.
                    if let Some(on_task) =
                        frontmost::local_context_guess(kind, &page_text, &goals)
                    {
                        if on_task {
                            if was_distracted && switched {
                                was_distracted = false;
                                reset_ephemeral_cooldown();
                                push_ephemeral(
                                    &app,
                                    "This looks study-related — good. Stay with the material.",
                                    "encourage",
                                );
                            }
                            mark_local_on_task(&app, &info);
                        } else {
                            if switched {
                                reset_ephemeral_cooldown();
                            }
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
                        }
                    } else {
                        // Unclear — hold status lightly until text Gemini returns.
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

                    // Cheap text Gemini confirm on open/switch (not every poll).
                    if switched && fp != last_judged_fingerprint
                        && context_judgments < MAX_CONTEXT_JUDGMENTS_PER_SESSION
                    {
                        last_judged_fingerprint = fp;
                        context_judgments = context_judgments.saturating_add(1);
                        let judged = judge_context_with_gemini(
                            &app,
                            &goals,
                            kind,
                            &info,
                            &page_text,
                            &mut last_vitals,
                        )
                        .await;
                        if let Some(on_task) = judged {
                            if on_task {
                                if was_distracted {
                                    was_distracted = false;
                                    reset_ephemeral_cooldown();
                                    push_ephemeral(
                                        &app,
                                        "Nice — this fits your lock-in. Keep going.",
                                        "encourage",
                                    );
                                }
                            } else {
                                was_distracted = true;
                            }
                        }
                    }
                }
                frontmost::FocusEvent::Clear(info) => {
                    let fp = info.fingerprint();
                    last_fingerprint = fp;
                    if was_distracted {
                        was_distracted = false;
                        reset_ephemeral_cooldown();
                        push_ephemeral(
                            &app,
                            "Nice — you’re back on your lock-in. Good job, keep it up.",
                            "encourage",
                        );
                    }
                    mark_local_on_task(&app, &info);
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(LOCAL_TICK_SECS)).await;
    }
}

async fn judge_context_with_gemini(
    app: &AppHandle,
    goals: &str,
    kind: &str,
    info: &frontmost::FrontmostInfo,
    page_text: &str,
    last_vitals: &mut VitalsSnapshot,
) -> Option<bool> {
    let cfg = app.state::<AppState>().config.lock().clone();
    let client = GeminiClient::from_config(&cfg).ok()?;
    // Enrich YouTube with oEmbed title when page JS is blocked.
    let mut text = page_text.to_string();
    if kind == "youtube" && !info.url.is_empty() {
        if let Some(oembed) = fetch_youtube_oembed_title(&info.url).await {
            if !text.to_lowercase().contains(&oembed.to_lowercase()) {
                text = format!("{oembed}\n{text}");
            }
        }
    }

    match client
        .judge_focus_context(
            goals,
            kind,
            &info.app_name,
            &info.window_title,
            &info.url,
            &text,
        )
        .await
    {
        Ok(mut result) => {
            if result.coach_line.trim().is_empty() && !result.on_task {
                result.coach_line = format!(
                    "This {kind} doesn’t look like it advances your lock-in — switch back."
                );
            }
            let on_task = result.on_task && result.distraction.is_none();
            apply_coach_tick(app, &result, last_vitals).await;
            Some(on_task)
        }
        Err(e) => {
            tracing::warn!("context judge: {e}");
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

async fn run_gemini_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
    let mut last_vitals = VitalsSnapshot::default();
    let mut last_error_notice = chrono::Utc::now() - chrono::Duration::seconds(60);
    let mut extra_cooldown_secs = 0u64;
    let mut was_distracted = false;
    let mut vision_calls: u32 = 0;

    // First full-screen check quickly after start.
    tokio::time::sleep(Duration::from_secs(3)).await;

    while !stop.load(Ordering::SeqCst) {
        let (active, goals, modality, remaining, already_distracted) = {
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
                        matches!(s.status, SessionStatusKind::Distracted),
                    )
                }
                _ => (false, String::new(), String::new(), 0, false),
            }
        };

        if !active || remaining <= 0 {
            break;
        }

        // Local loop already nags known distractions — skip vision to protect daily quota.
        if already_distracted {
            tokio::time::sleep(Duration::from_secs(
                GEMINI_TICK_OFF_TASK_SECS.max(extra_cooldown_secs),
            ))
            .await;
            extra_cooldown_secs = 0;
            continue;
        }

        if vision_calls >= MAX_VISION_PER_SESSION {
            tracing::info!(
                "coach vision: session budget reached ({MAX_VISION_PER_SESSION}); local watch continues"
            );
            tokio::time::sleep(Duration::from_secs(GEMINI_TICK_OFF_TASK_SECS)).await;
            continue;
        }

        let hints = tokio::task::spawn_blocking(|| {
            let info = frontmost::frontmost_info().unwrap_or_default();
            frontmost::open_context_hints(&info)
        })
        .await
        .unwrap_or_default();

        let screen_jpeg = match tokio::task::spawn_blocking(screen::grab_desktop_jpeg).await {
            Ok(Ok(bytes)) => Some(bytes),
            Ok(Err(e)) => {
                tracing::warn!("screen: {e}");
                maybe_soft_error(&app, &mut last_error_notice, &e);
                None
            }
            Err(e) => {
                tracing::warn!("screen join: {e}");
                None
            }
        };

        if let Some(bytes) = screen_jpeg {
            let cfg = app.state::<AppState>().config.lock().clone();
            let vision = match GeminiClient::from_config(&cfg) {
                Ok(client) => {
                    vision_calls = vision_calls.saturating_add(1);
                    client
                        .analyze_session(
                            &goals,
                            &modality,
                            &last_vitals.raw_summary,
                            Some(&bytes),
                            None,
                            &hints,
                        )
                        .await
                }
                Err(e) => Err(e),
            };

            match vision {
                Ok(result) => {
                    let distracted = !result.on_task || result.distraction.is_some();
                    if distracted {
                        was_distracted = true;
                    } else if was_distracted {
                        was_distracted = false;
                        reset_ephemeral_cooldown();
                        push_ephemeral(
                            &app,
                            "Nice — you’re back on your lock-in. Good job, keep it up.",
                            "encourage",
                        );
                    }
                    apply_coach_tick(&app, &result, &mut last_vitals).await;
                }
                Err(e) => {
                    tracing::warn!("coach vision: {e}");
                    if gemini::error_looks_rate_limited(&e) {
                        extra_cooldown_secs = RATE_LIMIT_COOLDOWN_SECS;
                    } else {
                        maybe_soft_error(&app, &mut last_error_notice, &e);
                    }
                }
            }
        }

        let wait_secs = GEMINI_TICK_ON_TASK_SECS.max(extra_cooldown_secs);
        extra_cooldown_secs = 0;
        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
    }
}

fn maybe_soft_error(
    app: &AppHandle,
    last_notice: &mut chrono::DateTime<chrono::Utc>,
    message: &str,
) {
    let now = chrono::Utc::now();
    if now.signed_duration_since(*last_notice).num_seconds() < 45 {
        return;
    }
    *last_notice = now;
    push_ephemeral(app, message, "error");
}

async fn run_presage_loop(app: AppHandle, session_id: String, stop: Arc<AtomicBool>) {
    // Fully deferred — never blocks coaching / Instagram catch at session start.
    tokio::time::sleep(Duration::from_secs(PRESAGE_START_DELAY_SECS)).await;
    let mut last_stress_prompt = chrono::Utc::now() - chrono::Duration::minutes(10);

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

fn social_distraction_label(result: &CoachVisionResult) -> Option<&'static str> {
    let blob = format!(
        "{} {} {}",
        result.objects.join(" "),
        result.distraction.as_deref().unwrap_or(""),
        result.coach_line
    )
    .to_lowercase();
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
        ("email", "email"),
        ("amazon", "shopping"),
        ("shopping", "shopping"),
        ("imessage", "texting"),
        ("messages", "texting"),
        ("whatsapp", "texting"),
        ("telegram", "texting"),
        ("texting", "texting"),
    ];
    for (needle, label) in SITES {
        if blob.contains(needle) {
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
    if !on_task {
        if let Some(label) = social.or(distraction.as_deref()).and_then(static_distraction_label) {
            // Prefer model line when it already named the video/context.
            if prompt_text.trim().is_empty() || label != "youtube" {
                prompt_text = frontmost::distraction_coach_line(&frontmost::DistractionHit {
                    label,
                    detail: label.to_string(),
                    focused: true,
                });
            }
        }
    }
    if has_phone && !has_calc {
        prompt_text =
            "Phone out — that’s a distraction. Park it and get back to your lock-in goal.".into();
    } else if has_calc && on_task && prompt_text.to_lowercase().contains("phone") {
        prompt_text = "Calculator is fair game — keep working the problem.".into();
    } else if !on_task && prompt_text.trim().is_empty() {
        prompt_text = "This doesn’t look like your goal — switch back to the work.".into();
    }

    if (last_vitals.stressed || result.stress_cue)
        && !prompt_text.to_lowercase().contains("breath")
    {
        prompt_text =
            format!("{prompt_text} If shoulders are tight, take one slow breath, then continue.");
    }

    let wants_prompt = !on_task || result.needs_help || last_vitals.stressed || has_phone;
    let cooldown = if social.is_some() || !on_task {
        SOCIAL_PROMPT_COOLDOWN_SECS
    } else {
        PROMPT_COOLDOWN_SECS
    };
    let prompt = if wants_prompt && claim_ephemeral_slot(cooldown) {
        Some(CoachPrompt {
            id: Uuid::new_v4().to_string(),
            at: chrono::Utc::now().to_rfc3339(),
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
                format!(
                    "Distracted{}",
                    distraction
                        .as_ref()
                        .map(|d| format!(" · {d}"))
                        .unwrap_or_default()
                )
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

fn push_ephemeral(app: &AppHandle, text: &str, kind: &str) {
    let prompt = CoachPrompt {
        id: Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        text: text.into(),
        kind: kind.into(),
    };
    deliver_ephemeral(app, &prompt);
}

fn deliver_ephemeral(app: &AppHandle, prompt: &CoachPrompt) {
    // Desktop popup only — do not leave a lasting trail in the session feed.
    overlay::show_prompt(app, prompt);
    let silent = app.state::<AppState>().silent_mode.load(Ordering::SeqCst);
    if silent {
        return;
    }
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
