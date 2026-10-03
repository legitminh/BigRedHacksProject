use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

use super::jpeg_from_rgb;

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
        Ok(Ok(false)) => Err("Allow Waypoint camera access in System Settings → Privacy & Security → Camera, then try again.".into()),
        _ => Err("Camera permission request timed out. Please try again.".into()),
    }
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
