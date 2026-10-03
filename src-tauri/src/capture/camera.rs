use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

use super::{encode_clip_from_jpegs, jpeg_from_rgb};

pub async fn request_permission() -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    nokhwa::nokhwa_initialize(move |granted| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(granted);
        }
    });
    match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(
            "Allow Waypoint camera access in System Settings → Privacy & Security → Camera, then try again."
                .into(),
        ),
        _ => Err("Camera permission request timed out. Please try again.".into()),
    }
}

pub fn permission_granted() -> bool {
    nokhwa::nokhwa_check()
}

/// Grab a single JPEG frame. Opens and closes the device each call so the
/// capture handle never needs to be `Send` across Tauri async tasks.
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
    let frame = cam.frame().map_err(|e| format!("camera frame: {e}"))?;
    let decoded = frame
        .decode_image::<RgbFormat>()
        .map_err(|e| format!("decode: {e}"))?;
    let jpeg = jpeg_from_rgb(decoded.width(), decoded.height(), decoded.as_raw())?;
    let _ = cam.stop_stream();
    Ok(jpeg)
}

/// Record a continuous webcam clip for Presage (needs ~20s, >10 fps, face visible).
/// Runs entirely on the calling thread so the camera handle stays local.
pub fn record_presage_clip(
    dir: &Path,
    duration_secs: u64,
    fps: u32,
) -> Result<PathBuf, String> {
    if !nokhwa::nokhwa_check() {
        return Err("Camera permission is unavailable.".into());
    }
    let fps = fps.clamp(10, 20);
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

    let frame_interval = Duration::from_millis(1000 / u64::from(fps));
    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let mut paths = Vec::new();
    let mut i = 0u32;

    while Instant::now() < deadline {
        let started = Instant::now();
        match cam.frame() {
            Ok(frame) => match frame.decode_image::<RgbFormat>() {
                Ok(decoded) => {
                    match jpeg_from_rgb(decoded.width(), decoded.height(), decoded.as_raw()) {
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
        let elapsed = started.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }
    let _ = cam.stop_stream();

    let min_frames = (fps as usize).saturating_mul(8);
    if paths.len() < min_frames {
        return Err(format!(
            "Only captured {} webcam frames (need ~{min_frames}). Sit facing the camera in good light.",
            paths.len()
        ));
    }

    let out = dir.join("presage-clip.mp4");
    encode_clip_from_jpegs(&paths, &out, fps)?;
    let _ = std::fs::remove_dir_all(&frames_dir);
    Ok(out)
}
