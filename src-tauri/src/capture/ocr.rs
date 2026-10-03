use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Run Apple Vision OCR on an image file (fast recognition level).
pub fn ocr_image_file(path: &Path) -> Result<String, String> {
    let bin = ocr_binary()?;
    let output = Command::new(&bin)
        .arg(path)
        .output()
        .map_err(|e| {
            format!(
                "OCR helper failed to start ({e}). Rebuild the app so waypoint-ocr is compiled."
            )
        })?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("OCR failed: {err}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// OCR JPEG bytes by writing a short-lived temp file.
pub fn ocr_jpeg_bytes(jpeg: &[u8]) -> Result<String, String> {
    let dir = env::temp_dir().join("waypoint-ocr");
    let _ = std::fs::create_dir_all(&dir);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = dir.join(format!("frame-{ts}.jpg"));
    std::fs::write(&path, jpeg).map_err(|e| e.to_string())?;
    let result = ocr_image_file(&path);
    let _ = std::fs::remove_file(&path);
    result
}

fn ocr_binary() -> Result<PathBuf, String> {
    // Compile-time path from build.rs
    let baked = env!("WAYPOINT_OCR_BIN");
    let baked_path = PathBuf::from(baked);
    if baked_path.is_file() {
        return Ok(baked_path);
    }
    // Dev / relocated binary next to the app
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("waypoint-ocr");
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/waypoint-ocr");
    if manifest.is_file() {
        return Ok(manifest);
    }
    Err("waypoint-ocr binary missing — rebuild src-tauri".into())
}

/// Pull distraction labels from OCR / page text without a model.
pub fn labels_in_text(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let rules: &[(&str, &str)] = &[
        ("youtube", "youtube"),
        ("youtu.be", "youtube"),
        ("instagram", "instagram"),
        ("tiktok", "tiktok"),
        ("reddit", "reddit"),
        ("discord", "discord"),
        ("netflix", "netflix"),
        ("twitch", "twitch"),
        ("gmail", "email"),
        ("inbox", "email"),
        ("amazon", "shopping"),
        ("add to cart", "shopping"),
        ("shopping cart", "shopping"),
        ("facebook", "facebook"),
        ("twitter", "twitter"),
        ("whatsapp", "texting"),
        ("imessage", "texting"),
        ("messages", "texting"),
    ];
    for (needle, label) in rules {
        if lower.contains(needle) && !out.contains(label) {
            out.push(*label);
        }
    }
    out
}
