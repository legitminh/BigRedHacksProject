use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, DynamicImage, ImageEncoder};
use xcap::Monitor;

const SCREEN_HELP: &str = "Allow Screen Recording for Waypoint in System Settings → Privacy & Security → Screen Recording, then quit and reopen the app.";

/// Smaller JPEG for Gemini — cuts tokens / quota pressure.
const MAX_WIDTH: u32 = 960;
const JPEG_QUALITY: u8 = 55;

pub fn grab_primary_jpeg() -> Result<Vec<u8>, String> {
    let monitors = Monitor::all().map_err(|e| format!("Couldn’t list displays ({e}). {SCREEN_HELP}"))?;
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| Monitor::all().ok().and_then(|mut m| m.pop()))
        .ok_or_else(|| format!("No monitors found. {SCREEN_HELP}"))?;

    let image = monitor
        .capture_image()
        .map_err(|e| format!("Screen capture failed ({e}). {SCREEN_HELP}"))?;

    let scale = (MAX_WIDTH as f32 / image.width() as f32).min(1.0);
    let tw = ((image.width() as f32) * scale).round().max(320.0) as u32;
    let th = ((image.height() as f32) * scale).round().max(180.0) as u32;
    let resized = image::imageops::resize(&image, tw, th, image::imageops::FilterType::Triangle);

    // xcap returns RGBA; JPEG only accepts RGB.
    let rgb = DynamicImage::ImageRgba8(resized).to_rgb8();
    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ColorType::Rgb8.into())
        .map_err(|e| e.to_string())?;
    Ok(buf)
}
