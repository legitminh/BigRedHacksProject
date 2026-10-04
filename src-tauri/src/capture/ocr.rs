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

/// Strong site evidence only — never bare brand words.
/// Mentions like “Instagram’s algorithm” in an article must NOT count.
/// Host/URL evidence outranks title-chrome brand guesses when they conflict.
pub fn labels_in_text(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();

    // Prefer host-like tokens OCR actually saw in the address bar / page chrome.
    // YouTube hosts first so Shorts/watch URLs win ordering over later chrome guesses.
    let hosts: &[(&str, &str)] = &[
        ("youtube.com", "youtube"),
        ("youtu.be", "youtube"),
        ("instagram.com", "instagram"),
        ("tiktok.com", "tiktok"),
        ("reddit.com", "reddit"),
        ("discord.com", "discord"),
        ("discordapp.com", "discord"),
        ("netflix.com", "netflix"),
        ("twitch.tv", "twitch"),
        ("mail.google.com", "email"),
        ("outlook.live.com", "email"),
        ("amazon.com", "shopping"),
        ("amazon.", "shopping"),
        ("facebook.com", "facebook"),
        ("web.whatsapp.com", "texting"),
    ];
    for (needle, label) in hosts {
        if lower.contains(needle) && !out.contains(label) {
            out.push(*label);
        }
    }

    let has_youtube_host = lower.contains("youtube.com") || lower.contains("youtu.be");
    let has_instagram_host = lower.contains("instagram.com");
    // Address-bar / URL evidence for a non-YouTube page (Canvas, docs, …).
    // A stray "YouTube" chrome line must not win over canvas.cornell.edu.
    let has_other_page_host = ocr_has_non_youtube_page_host(&lower);

    // Tab/app chrome titles OCR sometimes reads as their own line.
    for line in lower.lines() {
        let t = line.trim();
        // YouTube URL/host wins: never add Instagram from a false-positive chrome line
        // (sidebar/OCR misread) while the address bar clearly says youtube.com.
        if title_is_brand_chrome(t, "instagram")
            && !out.contains(&"instagram")
            && !has_youtube_host
        {
            out.push("instagram");
        }
        // YouTube chrome alone is not enough when OCR also saw another page host.
        if title_is_brand_chrome(t, "youtube")
            && !out.contains(&"youtube")
            && (has_youtube_host || !has_other_page_host)
        {
            out.push("youtube");
        }
        if title_is_brand_chrome(t, "tiktok") && !out.contains(&"tiktok") {
            out.push("tiktok");
        }
    }

    // If host evidence says YouTube and not Instagram, drop Instagram chrome guesses.
    if has_youtube_host && !has_instagram_host {
        out.retain(|l| *l != "instagram");
    }
    // Canvas / school hosts: never keep a youtube label without youtube.com / youtu.be.
    if has_other_page_host && !has_youtube_host {
        out.retain(|l| *l != "youtube");
    }

    out
}

/// True when OCR text shows a URL/host that is clearly not YouTube (Canvas, LMS, …).
fn ocr_has_non_youtube_page_host(lower: &str) -> bool {
    const OTHER: &[&str] = &[
        "canvas.",
        "instructure.com",
        "blackboard.",
        "moodle.",
        "classroom.google",
        "docs.google.com",
        "drive.google.com",
        "github.com",
        "gitlab.com",
        "notion.so",
        "notion.site",
        "cornell.edu",
        ".edu/",
        ".edu?",
        "arxiv.org",
        "scholar.google",
        "wikipedia.org",
        "stackoverflow.com",
        "stackexchange.com",
    ];
    if OTHER.iter().any(|n| lower.contains(n)) {
        return true;
    }
    // Generic https://host/… that isn't youtube / youtu.be.
    for token in lower.split_whitespace() {
        let t = token.trim_matches(|c: char| "()[]<>,;\"'".contains(c));
        if let Some(rest) = t
            .strip_prefix("https://")
            .or_else(|| t.strip_prefix("http://"))
        {
            let host = rest.split('/').next().unwrap_or("").to_lowercase();
            if !host.is_empty()
                && !host.contains("youtube.com")
                && !host.contains("youtu.be")
                && host.contains('.')
            {
                return true;
            }
        }
    }
    false
}

fn title_is_brand_chrome(line: &str, brand: &str) -> bool {
    if line == brand {
        return true;
    }
    // "Something - Instagram" / "Something | Instagram" / "Something • Instagram"
    for sep in [" - ", " | ", " • ", " – ", " — "] {
        if let Some((_, right)) = line.rsplit_once(sep) {
            if right.trim() == brand {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_host_beats_instagram_chrome_false_positive() {
        let text = "\
https://www.youtube.com/shorts/abc123
Want to learn how to code? #shorts
Instagram
Home
Shorts
";
        let labels = labels_in_text(text);
        assert!(
            labels.contains(&"youtube"),
            "expected youtube from host: {labels:?}"
        );
        assert!(
            !labels.contains(&"instagram"),
            "Instagram chrome must not win over youtube.com: {labels:?}"
        );
        assert_eq!(labels.first().copied(), Some("youtube"));
    }

    #[test]
    fn instagram_host_still_labels_instagram() {
        let labels = labels_in_text("https://www.instagram.com/reel/xyz\nReel title - Instagram");
        assert_eq!(labels.first().copied(), Some("instagram"));
    }

    #[test]
    fn bare_instagram_word_in_article_is_ignored() {
        let labels = labels_in_text(
            "Why Instagram changed its algorithm — The Verge\nhttps://www.theverge.com/article",
        );
        assert!(!labels.contains(&"instagram"));
    }

    #[test]
    fn canvas_page_without_youtube_host_is_not_youtube() {
        let text = "\
https://canvas.cornell.edu/courses/12345/discussion_topics/678
ENGL 1140
Alright, I am working on my discussion post now
Historian as Curandera by Aurora Levins Morales
Indigo Kifer
";
        let labels = labels_in_text(text);
        assert!(
            !labels.contains(&"youtube"),
            "Canvas discussion must not yield youtube: {labels:?}"
        );
    }

    #[test]
    fn canvas_with_stray_youtube_chrome_line_is_not_youtube() {
        // Desktop OCR sometimes catches a background tab title or misreads a line.
        let text = "\
https://canvas.cornell.edu/courses/12345/discussion_topics/678
ENGL 1140 Discussion
YouTube
Alright, I am working on my discussion post now
";
        let labels = labels_in_text(text);
        assert!(
            !labels.contains(&"youtube"),
            "stray YouTube chrome must not win over canvas host: {labels:?}"
        );
    }

    #[test]
    fn youtube_watch_host_still_labels_youtube() {
        let labels = labels_in_text(
            "KSI - Thick of it - YouTube\nhttps://www.youtube.com/watch?v=SbTT1f3xZVg\nHome",
        );
        assert_eq!(labels.first().copied(), Some("youtube"));
    }

    #[test]
    fn youtube_mention_in_canvas_discussion_is_ignored() {
        let text = "\
canvas.cornell.edu/courses/1/discussion_topics/2
I watched a youtube explainer last week for ENGL 1140
";
        let labels = labels_in_text(text);
        assert!(
            !labels.contains(&"youtube"),
            "bare youtube mention on Canvas must not label: {labels:?}"
        );
    }
}
