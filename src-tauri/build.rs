use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let secrets = manifest.join("secrets.toml");
    let example = manifest.join("secrets.example.toml");

    // secrets.toml is baked into the binary — never put Gemini/Google secrets here.
    // End-user builds only need waypoint_api_base (and optional Presage).
    if !secrets.exists() {
        if example.exists() {
            fs::copy(&example, &secrets).expect("copy secrets.example.toml → secrets.toml");
        } else {
            fs::write(
                &secrets,
                "waypoint_api_base = \"http://127.0.0.1:8787\"\n\
                 local_llm_base = \"http://127.0.0.1:8787/v1/coach\"\n\
                 local_llm_model = \"qwen2.5:0.5b\"\n\
                 local_vision_model = \"moondream\"\n\
                 coach_api_token = \"\"\n\
                 presage_api_key = \"\"\n",
            )
            .expect("write src-tauri/secrets.toml");
        }
    }

    // Fail the build if someone tries to bake extractable secrets into Waypoint.app.
    // Gemini/Google/xAI belong on the API. Presage/coach tokens must also stay empty
    // for release — camera observe uses the signed-in user JWT instead.
    let raw = fs::read_to_string(&secrets).expect("read secrets.toml");
    let profile = env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";
    for forbidden in [
        "gemini_api_key",
        "google_client_id",
        "google_client_secret",
        "xai_api_key",
        // Always refuse in release; warn-only in debug so local experiments still work.
        "presage_api_key",
        "coach_api_token",
    ] {
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != forbidden {
                continue;
            }
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            if value.is_empty() {
                continue;
            }
            let soft = matches!(forbidden, "presage_api_key" | "coach_api_token");
            if soft && !is_release {
                println!(
                    "cargo:warning={forbidden} is set in secrets.toml — extractable from the binary. \
                     Leave empty for release; prefer signed-in JWT / server-side keys."
                );
                continue;
            }
            panic!(
                "{forbidden} must not be set in secrets.toml — it would be extractable from \
                 Waypoint.app. Put server secrets only in the Waypoint API .env."
            );
        }
    }

    // Release shipping builds must not bake localhost API URLs.
    // `npm run app:build` sets WAYPOINT_RELEASE=1 after secrets.toml points at PUBLIC_BASE_URL.
    if env::var("WAYPOINT_RELEASE").ok().as_deref() == Some("1") {
        let mut api_base = String::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != "waypoint_api_base" {
                continue;
            }
            api_base = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim()
                .to_string();
            break;
        }
        let lower = api_base.to_ascii_lowercase();
        if api_base.is_empty()
            || lower.contains("127.0.0.1")
            || lower.contains("localhost")
            || lower.starts_with("http://")
        {
            panic!(
                "WAYPOINT_RELEASE=1 requires waypoint_api_base to be a public https:// origin \
                 (got {api_base:?}). Update secrets.toml to match PUBLIC_BASE_URL, then rebuild."
            );
        }
    }

    println!("cargo:rerun-if-changed=secrets.toml");
    println!("cargo:rerun-if-changed=secrets.example.toml");
    println!("cargo:rerun-if-env-changed=WAYPOINT_RELEASE");
    println!("cargo:rerun-if-changed=Info.plist");
    println!("cargo:rerun-if-changed=tools/ocr_vision.swift");
    println!("cargo:rerun-if-changed=tools/encode_clip.swift");

    compile_ocr_helper(&manifest);
    compile_encode_clip_helper(&manifest);

    // Autogenerate allow-*/deny-* for every custom command so Tauri enforces
    // per-window ACL (empty app manifest previously skipped checks for local webviews).
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_status",
            "get_system_permissions",
            "request_camera_permission",
            "local_llm_status",
            "service_status",
            "gemini_status",
            "sign_in_waypoint",
            "sign_in_waypoint_google",
            "sign_in_waypoint_guest",
            "cancel_sign_in_waypoint_google",
            "sign_out_waypoint",
            "study_memory_stats",
            "connect_google",
            "disconnect_google",
            "get_google_context",
            "ensure_school_digest",
            "refresh_school_digest",
            "chat_send",
            "clear_chat",
            "companion_send",
            "companion_clear",
            "companion_live_info",
            "companion_grab_screencap",
            "voice_stop",
            "start_lock_in",
            "stop_lock_in",
            "set_lock_in_paused",
            "start_break_timer",
            "end_break_timer",
            "suggest_break_timer",
            "get_break_timer_status",
            "get_session",
            "get_settings",
            "save_settings",
            "voice_speak",
            "voice_listen_test",
            "delete_all_user_data",
        ]),
    );
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}

fn compile_ocr_helper(manifest: &PathBuf) {
    let swift = manifest.join("tools/ocr_vision.swift");
    let bin_dir = manifest.join("bin");
    let _ = fs::create_dir_all(&bin_dir);
    let out = bin_dir.join("waypoint-ocr");
    println!("cargo:rustc-env=WAYPOINT_OCR_BIN={}", out.display());

    if !swift.exists() {
        eprintln!("cargo:warning=OCR swift source missing at {}", swift.display());
        return;
    }
    if cfg!(not(target_os = "macos")) {
        return;
    }
    let status = Command::new("swiftc")
        .args([
            "-O",
            "-framework",
            "Vision",
            "-framework",
            "AppKit",
            "-o",
            out.to_str().unwrap_or("waypoint-ocr"),
            swift.to_str().unwrap_or("ocr_vision.swift"),
        ])
        .status();
    match status {
        Ok(s) if s.success() => {
            // Success is silent — cargo:warning would spam every rebuild.
        }
        Ok(s) => {
            // Real failure: surface so builders know OCR is off.
            println!("cargo:warning=swiftc failed ({s}) — local OCR disabled until rebuild succeeds");
        }
        Err(e) => {
            println!("cargo:warning=swiftc not available ({e}) — local OCR disabled");
        }
    }
}

fn compile_encode_clip_helper(manifest: &PathBuf) {
    let swift = manifest.join("tools/encode_clip.swift");
    let bin_dir = manifest.join("bin");
    let _ = fs::create_dir_all(&bin_dir);
    let out = bin_dir.join("waypoint-encode-clip");
    println!("cargo:rustc-env=WAYPOINT_ENCODE_CLIP_BIN={}", out.display());

    if !swift.exists() {
        eprintln!(
            "cargo:warning=encode_clip.swift missing at {}",
            swift.display()
        );
        return;
    }
    if cfg!(not(target_os = "macos")) {
        return;
    }
    let status = Command::new("swiftc")
        .args([
            "-O",
            "-framework",
            "AVFoundation",
            "-framework",
            "AppKit",
            "-framework",
            "CoreMedia",
            "-framework",
            "CoreVideo",
            "-o",
            out.to_str().unwrap_or("waypoint-encode-clip"),
            swift.to_str().unwrap_or("encode_clip.swift"),
        ])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!(
                "cargo:warning=swiftc encode_clip failed ({s}) — will try ffmpeg fallback"
            );
        }
        Err(e) => {
            println!("cargo:warning=swiftc not available ({e}) — encode_clip disabled");
        }
    }
}
