use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn toml_string_value(raw: &str, key: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, value)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string();
        if value.is_empty() {
            return None;
        }
        return Some(value);
    }
    None
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let secrets = manifest.join("secrets.toml");
    let example = manifest.join("secrets.example.toml");

    let gemini_from_env = env::var("GEMINI_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let gemini_from_file = if secrets.exists() {
        let raw = fs::read_to_string(&secrets).expect("failed to read src-tauri/secrets.toml");
        toml_string_value(&raw, "gemini_api_key")
    } else {
        None
    };

    if gemini_from_file.is_none() && gemini_from_env.is_none() {
        let hint = if !secrets.exists() {
            if example.exists() {
                "\nMissing src-tauri/secrets.toml.\n\
                 Run:\n\
                   cp src-tauri/secrets.example.toml src-tauri/secrets.toml\n\
                 Then set gemini_api_key = \"YOUR_KEY\" before building.\n"
            } else {
                "\nMissing src-tauri/secrets.toml with gemini_api_key set.\n"
            }
        } else {
            "\ngemini_api_key is empty in src-tauri/secrets.toml \
             (and GEMINI_API_KEY is unset).\n\
             Waypoint cannot compile without a Gemini API key.\n\
             Set gemini_api_key in secrets.toml, then rebuild.\n"
        };
        panic!("{hint}");
    }

    // include_str! in config.rs needs the file to exist; seed from env if needed.
    if !secrets.exists() {
        let key = gemini_from_env.expect("checked above");
        fs::write(
            &secrets,
            format!(
                "gemini_api_key = \"{key}\"\n\
                 gemini_model = \"gemini-flash-latest\"\n\
                 presage_api_key = \"\"\n\
                 google_client_id = \"\"\n\
                 google_client_secret = \"\"\n"
            ),
        )
        .expect("write src-tauri/secrets.toml");
    }

    println!("cargo:rerun-if-changed=secrets.toml");
    println!("cargo:rerun-if-changed=secrets.example.toml");
    println!("cargo:rerun-if-changed=Info.plist");
    println!("cargo:rerun-if-changed=tools/ocr_vision.swift");
    println!("cargo:rerun-if-changed=tools/encode_clip.swift");
    println!("cargo:rerun-if-env-changed=GEMINI_API_KEY");

    compile_ocr_helper(&manifest);
    compile_encode_clip_helper(&manifest);

    tauri_build::build()
}

fn compile_ocr_helper(manifest: &PathBuf) {
    let swift = manifest.join("tools/ocr_vision.swift");
    let bin_dir = manifest.join("bin");
    let _ = fs::create_dir_all(&bin_dir);
    let out = bin_dir.join("waypoint-ocr");
    // Always expose a path so ocr.rs can compile even if swiftc is missing on CI.
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
            println!("cargo:warning=compiled local OCR helper → {}", out.display());
        }
        Ok(s) => {
            eprintln!("cargo:warning=swiftc failed ({s}) — local OCR disabled until rebuild succeeds");
        }
        Err(e) => {
            eprintln!("cargo:warning=swiftc not available ({e}) — local OCR disabled");
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
        Ok(s) if s.success() => {
            println!(
                "cargo:warning=compiled clip encoder → {}",
                out.display()
            );
        }
        Ok(s) => {
            eprintln!("cargo:warning=swiftc encode_clip failed ({s}) — will try ffmpeg fallback");
        }
        Err(e) => {
            eprintln!("cargo:warning=swiftc not available ({e}) — encode_clip disabled");
        }
    }
}
