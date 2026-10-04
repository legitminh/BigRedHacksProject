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

    // Fail the build if someone tries to bake server-only secrets into Waypoint.app.
    let raw = fs::read_to_string(&secrets).expect("read secrets.toml");
    for forbidden in [
        "gemini_api_key",
        "google_client_id",
        "google_client_secret",
        "xai_api_key",
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
            if !value.is_empty() {
                panic!(
                    "{forbidden} must not be set in secrets.toml — it would be extractable from \
                     Waypoint.app. Put Gemini/Google secrets only in the Waypoint API .env."
                );
            }
        }
    }

    println!("cargo:rerun-if-changed=secrets.toml");
    println!("cargo:rerun-if-changed=secrets.example.toml");
    println!("cargo:rerun-if-changed=Info.plist");
    println!("cargo:rerun-if-changed=tools/ocr_vision.swift");
    println!("cargo:rerun-if-changed=tools/encode_clip.swift");

    compile_ocr_helper(&manifest);
    compile_encode_clip_helper(&manifest);

    tauri_build::build()
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
