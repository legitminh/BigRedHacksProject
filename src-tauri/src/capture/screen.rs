use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{ColorType, DynamicImage, ImageBuffer, ImageEncoder, Rgba, RgbaImage};
use xcap::{Monitor, Window};

const SCREEN_HELP: &str = "Allow Screen Recording for Waypoint in System Settings → Privacy & Security → Screen Recording, then quit and reopen the app.";

/// Wide enough for Gemini to read UI chrome across the desktop (tabs, side panels, PiP).
const MAX_WIDTH: u32 = 1600;
const JPEG_QUALITY: u8 = 62;
/// Compact frame for local OCR / tiny VLM — keeps CPU and tokens down.
const OCR_MAX_WIDTH: u32 = 960;
const OCR_JPEG_QUALITY: u8 = 50;

/// Capture the whole desktop (all monitors stitched) as one JPEG for coaching.
pub fn grab_primary_jpeg() -> Result<Vec<u8>, String> {
    grab_desktop_jpeg()
}

pub fn grab_desktop_jpeg() -> Result<Vec<u8>, String> {
    let mut monitors = Monitor::all().map_err(|e| format!("Couldn’t list displays ({e}). {SCREEN_HELP}"))?;
    if monitors.is_empty() {
        return Err(format!("No monitors found. {SCREEN_HELP}"));
    }
    // Stable left-to-right order.
    monitors.sort_by_key(|m| m.x().unwrap_or(0));

    let mut captures: Vec<RgbaImage> = Vec::new();
    for monitor in &monitors {
        let image = monitor
            .capture_image()
            .map_err(|e| format!("Screen capture failed ({e}). {SCREEN_HELP}"))?;
        captures.push(image);
    }

    let desktop = stitch_horizontal(&captures);
    let scale = (MAX_WIDTH as f32 / desktop.width() as f32).min(1.0);
    let tw = ((desktop.width() as f32) * scale).round().max(640.0) as u32;
    let th = ((desktop.height() as f32) * scale).round().max(360.0) as u32;
    let resized = imageops::resize(&desktop, tw, th, FilterType::Triangle);
    let rgb = DynamicImage::ImageRgba8(resized).to_rgb8();

    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ColorType::Rgb8.into())
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Prefer the focused window; fall back to a downscaled primary display.
/// Small on purpose so OCR + local VLM stay light.
pub fn grab_ocr_jpeg() -> Result<Vec<u8>, String> {
    if let Ok(Some(img)) = focused_window_image() {
        return encode_resized_jpeg(img, OCR_MAX_WIDTH, OCR_JPEG_QUALITY);
    }
    let monitors =
        Monitor::all().map_err(|e| format!("Couldn’t list displays ({e}). {SCREEN_HELP}"))?;
    let monitor = monitors
        .into_iter()
        .next()
        .ok_or_else(|| format!("No monitors found. {SCREEN_HELP}"))?;
    let image = monitor
        .capture_image()
        .map_err(|e| format!("Screen capture failed ({e}). {SCREEN_HELP}"))?;
    encode_resized_jpeg(image, OCR_MAX_WIDTH, OCR_JPEG_QUALITY)
}

fn focused_window_image() -> Result<Option<RgbaImage>, String> {
    let windows = Window::all().map_err(|e| e.to_string())?;
    for window in windows {
        let focused = window.is_focused().unwrap_or(false);
        if !focused {
            continue;
        }
        let app = window.app_name().unwrap_or_default().to_lowercase();
        if app.contains("waypoint") {
            continue;
        }
        match window.capture_image() {
            Ok(img) if img.width() > 32 && img.height() > 32 => return Ok(Some(img)),
            _ => continue,
        }
    }
    Ok(None)
}

fn encode_resized_jpeg(image: RgbaImage, max_width: u32, quality: u8) -> Result<Vec<u8>, String> {
    let scale = (max_width as f32 / image.width() as f32).min(1.0);
    let tw = ((image.width() as f32) * scale).round().max(320.0) as u32;
    let th = ((image.height() as f32) * scale).round().max(180.0) as u32;
    let resized = imageops::resize(&image, tw, th, FilterType::Triangle);
    let rgb = DynamicImage::ImageRgba8(resized).to_rgb8();
    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ColorType::Rgb8.into())
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

fn stitch_horizontal(images: &[RgbaImage]) -> RgbaImage {
    if images.is_empty() {
        return ImageBuffer::new(1, 1);
    }
    if images.len() == 1 {
        return images[0].clone();
    }
    let total_w: u32 = images.iter().map(|i| i.width()).sum();
    let max_h = images.iter().map(|i| i.height()).max().unwrap_or(1);
    let mut out: RgbaImage = ImageBuffer::from_pixel(total_w, max_h, Rgba([0, 0, 0, 255]));
    let mut x = 0u32;
    for img in images {
        imageops::overlay(&mut out, img, x as i64, 0);
        x += img.width();
    }
    out
}
