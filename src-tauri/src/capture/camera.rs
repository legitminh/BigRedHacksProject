use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

use super::jpeg_from_rgb;

/// Grab a single JPEG frame. Opens and closes the device each call so the
/// capture handle never needs to be `Send` across Tauri async tasks.
pub fn grab_jpeg() -> Result<Vec<u8>, String> {
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
