use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use image::imageops::{self, FilterType};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

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

/// Record a short webcam clip for server-side camera observe (presence / stress).
/// Grabs as fast as the camera/JPEG path allows, then encodes at the measured fps.
/// Duration is clamped so uploads stay under the ~8MB observe limit.
pub fn record_presage_clip(
    dir: &Path,
    duration_secs: u64,
    _target_fps: u32,
) -> Result<PathBuf, String> {
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
    // Presage prefers >10fps; accept fewer frames if we got a usable burst.
    let min_frames = 48usize;
    if paths.len() < min_frames {
        return Err(format!(
            "Only captured {} webcam frames (need ~{min_frames}). Close other apps using the camera, sit facing it in good light, and try again.",
            paths.len()
        ));
    }

    let out = dir.join("presage-clip.mp4");
    encode_clip_from_jpegs(&paths, &out, measured_fps.max(10))?;
    let _ = std::fs::remove_dir_all(&frames_dir);
    Ok(out)
}
