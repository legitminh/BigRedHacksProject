use image::ImageFormat;
use xcap::Monitor;

const SCREEN_HELP: &str = "Allow Screen Recording for Waypoint in System Settings → Privacy & Security → Screen Recording, then quit and reopen the app.";

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
    let resized = image::imageops::resize(
        &image,
        (image.width() / 2).max(640),
        (image.height() / 2).max(360),
        image::imageops::FilterType::Triangle,
    );
    let mut buf = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut buf);
        resized
            .write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?;
    }
    Ok(buf)
}
