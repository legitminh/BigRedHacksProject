use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use image::imageops::{self, FilterType};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use serde::Deserialize;

use super::{encode_clip_from_jpegs, jpeg_from_rgb};

/// Ask for camera access, but never block lock-in for long.
pub async fn request_permission_timeout(timeout: std::time::Duration) -> Result<(), String> {
    if nokhwa::nokhwa_check() {
        return Ok(());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    nokhwa::nokhwa_initialize(move |granted| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(granted);
        }
    });
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(
            "Allow Waypoint camera access in System Settings → Privacy & Security → Camera, then try again."
                .into(),
        ),
        Ok(Err(_)) => Err("Camera permission channel closed.".into()),
        Err(_) => Err("Camera permission request timed out.".into()),
    }
}

pub fn permission_granted() -> bool {
    nokhwa::nokhwa_check()
}

/// Grab a single JPEG frame. Opens and closes the device each call so the
/// capture handle never needs to be `Send` across Tauri async tasks.
#[allow(dead_code)]
pub fn grab_jpeg() -> Result<Vec<u8>, String> {
    if !nokhwa::nokhwa_check() {
        return Err("Camera permission is unavailable.".into());
    }
    let index = CameraIndex::Index(0);
    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut cam = Camera::new(index, requested).map_err(|e| format!("camera open: {e}"))?;
    cam.open_stream()
        .map_err(|e| format!("camera stream: {e}"))?;
    // Warm up — first frames are often blank/dark.
    for _ in 0..3 {
        let _ = cam.frame();
    }
    let frame = cam.frame().map_err(|e| format!("camera frame: {e}"))?;
    let decoded = frame
        .decode_image::<RgbFormat>()
        .map_err(|e| format!("decode: {e}"))?;
    let jpeg = jpeg_from_rgb(decoded.width(), decoded.height(), decoded.as_raw())?;
    let _ = cam.stop_stream();
    Ok(jpeg)
}

/// Local attention from Vision face geometry (not an LLM/VLM).
pub type FaceAttention = &'static str;

/// Short webcam clip + mean luminance + local Vision face for server `client_meta`.
pub struct PresageClip {
    pub path: PathBuf,
    /// Mean Rec.601 luma across sampled frames (0–255).
    pub brightness: f64,
    /// Local Vision majority face signal; `None` if the helper was unavailable.
    /// Also written to the clip sidecar for `observe_clip` (coach may not read these yet).
    #[allow(dead_code)]
    pub face_detected: Option<bool>,
    /// `"absent"` | `"present"` | `"looking_down"` | `"looking_away"` when measured.
    #[allow(dead_code)]
    pub attention: Option<String>,
}

/// Mean Rec.601 luma for an RGB24 buffer (0–255). Samples every Nth pixel for speed.
pub(crate) fn mean_luma_rgb(rgb: &[u8], step_px: usize) -> Option<f64> {
    if rgb.len() < 3 {
        return None;
    }
    let step = step_px.max(1) * 3;
    let mut sum = 0.0f64;
    let mut n = 0u64;
    let mut i = 0usize;
    while i + 2 < rgb.len() {
        let r = rgb[i] as f64;
        let g = rgb[i + 1] as f64;
        let b = rgb[i + 2] as f64;
        sum += 0.299 * r + 0.587 * g + 0.114 * b;
        n += 1;
        i += step;
    }
    if n == 0 {
        None
    } else {
        Some(sum / n as f64)
    }
}

#[derive(Debug, Deserialize)]
struct FaceDetectJson {
    face: bool,
    center_y: Option<f64>,
    center_x: Option<f64>,
    /// Head yaw radians from Vision (0 = facing camera).
    yaw: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FaceSample {
    pub face: bool,
    /// Vision-normalized Y (0 = bottom). Lower half of frame ⇒ center_y < 0.5.
    pub center_y: Option<f64>,
    pub center_x: Option<f64>,
    pub yaw: Option<f64>,
}

/// ~20° in radians — head turned away from the webcam.
const YAW_AWAY_RAD: f64 = 0.35;
/// Face box center far from horizontal middle ⇒ looking off to the side.
const CENTER_X_AWAY: f64 = 0.22;

fn sample_looking_away(s: &FaceSample) -> bool {
    if s.yaw.map(|y| y.abs() >= YAW_AWAY_RAD).unwrap_or(false) {
        return true;
    }
    s.center_x
        .map(|x| (x - 0.5).abs() >= CENTER_X_AWAY)
        .unwrap_or(false)
}

fn sample_looking_down(s: &FaceSample) -> bool {
    s.center_y.map(|y| y < 0.5).unwrap_or(false)
}

/// Aggregate sampled Vision results → face_detected + attention.
/// Priority when face present: looking_away (yaw/side) > looking_down > present.
pub(crate) fn aggregate_face_samples(samples: &[FaceSample]) -> Option<(bool, FaceAttention)> {
    if samples.is_empty() {
        return None;
    }
    let face_n = samples.iter().filter(|s| s.face).count();
    let face_detected = face_n * 2 >= samples.len();
    if !face_detected {
        return Some((false, "absent"));
    }
    let face_samples: Vec<_> = samples.iter().filter(|s| s.face).collect();
    let away_n = face_samples
        .iter()
        .filter(|s| sample_looking_away(s))
        .count();
    if away_n * 2 >= face_samples.len() {
        return Some((true, "looking_away"));
    }
    let looking_down_n = face_samples
        .iter()
        .filter(|s| sample_looking_down(s))
        .count();
    let attention: FaceAttention = if looking_down_n * 2 >= face_samples.len() {
        "looking_down"
    } else {
        "present"
    };
    Some((true, attention))
}

fn face_detect_binary() -> Option<PathBuf> {
    // Resolve every call — do NOT cache failure (OnceLock<Option> permanently killed
    // accountability when the first swiftc attempt failed before the helper was bundled).
    resolve_face_detect_helper()
}

fn resolve_face_detect_helper() -> Option<PathBuf> {
    #[cfg(not(target_os = "macos"))]
    {
        return None;
    }
    #[cfg(target_os = "macos")]
    {
        // 1) Bundled next to Waypoint (production / Applications install).
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let bundled = dir.join("waypoint-face-detect");
                if bundled.is_file() {
                    return Some(bundled);
                }
            }
        }
        // 2) Dev tree src-tauri/bin (cargo run / tests).
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/waypoint-face-detect");
        if dev.is_file() {
            return Some(dev);
        }
        // 3) Last resort: compile into temp (needs swiftc on PATH).
        match ensure_face_detect_helper() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("face detect helper unavailable: {e}");
                None
            }
        }
    }
}

fn ensure_face_detect_helper() -> Result<PathBuf, String> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err("face detect helper is macOS-only".into());
    }
    #[cfg(target_os = "macos")]
    {
        let dir = std::env::temp_dir().join("waypoint-face");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let src_path = dir.join("face_detect.swift");
        let bin_path = dir.join("waypoint-face-detect");
        let source = include_str!("../../tools/face_detect.swift");

        let needs_write = match std::fs::read_to_string(&src_path) {
            Ok(existing) => existing != source,
            Err(_) => true,
        };
        if needs_write {
            std::fs::write(&src_path, source).map_err(|e| e.to_string())?;
        }
        let needs_compile = needs_write || !bin_path.is_file();
        if needs_compile {
            let status = Command::new("swiftc")
                .args([
                    "-O",
                    "-framework",
                    "Vision",
                    "-framework",
                    "AppKit",
                    "-o",
                ])
                .arg(&bin_path)
                .arg(&src_path)
                .status()
                .map_err(|e| format!("swiftc face detect: {e}"))?;
            if !status.success() {
                return Err("swiftc face detect failed".into());
            }
        }
        Ok(bin_path)
    }
}

pub(crate) fn detect_face_in_jpeg(path: &Path) -> Option<FaceSample> {
    let bin = face_detect_binary()?;
    let output = Command::new(&bin).arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    let parsed: FaceDetectJson = serde_json::from_str(line).ok()?;
    Some(FaceSample {
        face: parsed.face,
        center_y: parsed.center_y,
        center_x: parsed.center_x,
        yaw: parsed.yaw,
    })
}

/// Sample evenly across captured frames (cap cheap Vision calls).
fn sample_face_signal(paths: &[PathBuf]) -> Option<(bool, FaceAttention)> {
    if paths.is_empty() {
        return None;
    }
    const MAX_SAMPLES: usize = 8;
    let n = paths.len().min(MAX_SAMPLES);
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let idx = if n == 1 {
            0
        } else {
            i * (paths.len() - 1) / (n - 1)
        };
        if let Some(sample) = detect_face_in_jpeg(&paths[idx]) {
            samples.push(sample);
        }
    }
    aggregate_face_samples(&samples)
}

/// Sidecar next to the mp4 so `observe_clip` can send face without coach.rs changes.
pub fn client_meta_sidecar_path(video_path: &Path) -> PathBuf {
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("presage-clip");
    video_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.meta.json"))
}

fn write_client_meta_sidecar(
    video_path: &Path,
    brightness: f64,
    face_detected: Option<bool>,
    attention: Option<&str>,
) {
    let mut meta = serde_json::json!({
        "brightness": brightness,
        "brightness_measured": true,
    });
    if let Some(f) = face_detected {
        meta["face_detected"] = serde_json::json!(f);
    }
    if let Some(a) = attention {
        meta["attention"] = serde_json::json!(a);
    }
    let path = client_meta_sidecar_path(video_path);
    if let Err(e) = std::fs::write(&path, meta.to_string()) {
        tracing::warn!("presage client_meta sidecar: {e}");
    }
}

/// Record a short webcam clip for server-side camera observe (presence / stress).
/// Grabs as fast as the camera/JPEG path allows, then encodes at the measured fps.
/// Duration is clamped so uploads stay under the ~8MB observe limit.
/// Local Apple Vision face rects supply desk-away / looking_down; Presage is optional vitals.
///
/// Demoted: live `camera_live` owns the webcam for presence. Kept for optional sparse
/// vitals upload (must not run alongside the live loop — one camera owner).
#[allow(dead_code)]
pub fn record_presage_clip(
    dir: &Path,
    duration_secs: u64,
    _target_fps: u32,
) -> Result<PresageClip, String> {
    if !nokhwa::nokhwa_check() {
        return Err("Camera permission is unavailable.".into());
    }
    // 12s ≈ enough frames for presence; 30s upper bound avoids oversized uploads.
    let duration_secs = duration_secs.clamp(12, 30);
    let frames_dir = dir.join("presage-frames");
    let _ = std::fs::remove_dir_all(&frames_dir);
    std::fs::create_dir_all(&frames_dir).map_err(|e| e.to_string())?;

    let index = CameraIndex::Index(0);
    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut cam = Camera::new(index, requested).map_err(|e| format!("camera open: {e}"))?;
    cam.open_stream()
        .map_err(|e| format!("camera stream: {e}"))?;

    for _ in 0..8 {
        let _ = cam.frame();
    }

    let started_at = Instant::now();
    let deadline = started_at + Duration::from_secs(duration_secs);
    let mut paths = Vec::new();
    let mut luma_sum = 0.0f64;
    let mut luma_n = 0u64;
    let mut i = 0u32;

    while Instant::now() < deadline {
        match cam.frame() {
            Ok(frame) => match frame.decode_image::<RgbFormat>() {
                Ok(decoded) => {
                    // Downscale for encode speed / smaller upload.
                    let w = decoded.width();
                    let h = decoded.height();
                    let target_w = 640u32.min(w);
                    let target_h = ((h as f32) * (target_w as f32 / w as f32)).round() as u32;
                    let small = if target_w < w {
                        imageops::resize(&decoded, target_w, target_h.max(1), FilterType::Triangle)
                    } else {
                        decoded
                    };
                    if let Some(luma) = mean_luma_rgb(small.as_raw(), 8) {
                        luma_sum += luma;
                        luma_n += 1;
                    }
                    match jpeg_from_rgb(small.width(), small.height(), small.as_raw()) {
                        Ok(jpeg) => {
                            let path = frames_dir.join(format!("frame-{i:05}.jpg"));
                            if std::fs::write(&path, &jpeg).is_ok() {
                                paths.push(path);
                                i += 1;
                            }
                        }
                        Err(e) => tracing::warn!("presage jpeg: {e}"),
                    }
                }
                Err(e) => tracing::warn!("presage decode: {e}"),
            },
            Err(e) => tracing::warn!("presage frame: {e}"),
        }
    }
    let _ = cam.stop_stream();

    let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
    let measured_fps = (paths.len() as f64 / elapsed).round().clamp(5.0, 30.0) as u32;
    // Prefer >10fps; ~24 frames (~2s) is enough for local Vision presence.
    let min_frames = 24usize;
    if paths.len() < min_frames {
        return Err(format!(
            "Only captured {} webcam frames (need ~{min_frames}). Close other apps using the camera, sit facing it in good light, and try again.",
            paths.len()
        ));
    }
    tracing::info!(
        "presage clip frames={} face_helper={}",
        paths.len(),
        face_detect_binary()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "MISSING".into())
    );

    let face_signal = sample_face_signal(&paths);
    let (face_detected, attention) = match face_signal {
        Some((f, a)) => (Some(f), Some(a.to_string())),
        None => (None, None),
    };

    let out = dir.join("presage-clip.mp4");
    encode_clip_from_jpegs(&paths, &out, measured_fps.max(10))?;
    let _ = std::fs::remove_dir_all(&frames_dir);
    let brightness = if luma_n > 0 {
        luma_sum / luma_n as f64
    } else {
        128.0
    };
    write_client_meta_sidecar(
        &out,
        brightness,
        face_detected,
        attention.as_deref(),
    );
    Ok(PresageClip {
        path: out,
        brightness,
        face_detected,
        attention,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(y: f64, x: f64, yaw: Option<f64>) -> FaceSample {
        FaceSample {
            face: true,
            center_y: Some(y),
            center_x: Some(x),
            yaw,
        }
    }

    fn no_face() -> FaceSample {
        FaceSample {
            face: false,
            center_y: None,
            center_x: None,
            yaw: None,
        }
    }

    #[test]
    fn aggregate_majority_absent() {
        let samples = [no_face(), no_face(), face(0.7, 0.5, Some(0.0))];
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(!detected);
        assert_eq!(attn, "absent");
    }

    #[test]
    fn aggregate_present_upper_frame() {
        let samples = [
            face(0.65, 0.5, Some(0.0)),
            face(0.55, 0.48, Some(0.1)),
            face(0.7, 0.52, Some(-0.1)),
        ];
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(detected);
        assert_eq!(attn, "present");
    }

    #[test]
    fn aggregate_looking_down_lower_40() {
        let samples = [
            face(0.25, 0.5, Some(0.0)),
            face(0.3, 0.5, Some(0.0)),
            face(0.35, 0.5, Some(0.0)),
            no_face(),
        ];
        // 3/4 face → present; 3/3 looking_down among face frames.
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(detected);
        assert_eq!(attn, "looking_down");
    }

    #[test]
    fn aggregate_looking_away_by_yaw() {
        let samples = [
            face(0.6, 0.5, Some(0.5)),
            face(0.55, 0.5, Some(-0.45)),
            face(0.65, 0.5, Some(0.4)),
        ];
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(detected);
        assert_eq!(attn, "looking_away");
    }

    #[test]
    fn aggregate_looking_away_by_center_x() {
        let samples = [
            face(0.6, 0.2, Some(0.0)),
            face(0.55, 0.18, None),
            face(0.65, 0.25, Some(0.1)),
        ];
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(detected);
        assert_eq!(attn, "looking_away");
    }

    #[test]
    fn looking_away_beats_looking_down() {
        let samples = [
            face(0.3, 0.5, Some(0.5)),
            face(0.25, 0.5, Some(0.4)),
            face(0.35, 0.5, Some(-0.45)),
        ];
        let (detected, attn) = aggregate_face_samples(&samples).unwrap();
        assert!(detected);
        assert_eq!(attn, "looking_away");
    }

    #[test]
    fn aggregate_empty_is_none() {
        assert!(aggregate_face_samples(&[]).is_none());
    }

    #[test]
    fn sidecar_path_matches_clip_stem() {
        let p = PathBuf::from("/tmp/sess/presage-clip.mp4");
        assert_eq!(
            client_meta_sidecar_path(&p),
            PathBuf::from("/tmp/sess/presage-clip.meta.json")
        );
    }
}
