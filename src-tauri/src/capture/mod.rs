pub mod camera;
pub mod screen;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use image::{ImageBuffer, ImageFormat, Rgb};

pub fn jpeg_from_rgb(width: u32, height: u32, rgb: &[u8]) -> Result<Vec<u8>, String> {
    let img: ImageBuffer<Rgb<u8>, _> =
        ImageBuffer::from_raw(width, height, rgb.to_vec()).ok_or("invalid rgb buffer")?;
    let mut buf = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

pub fn temp_session_dir(session_id: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("waypoint").join(session_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn save_jpeg(dir: &Path, label: &str, jpeg: &[u8]) -> Result<PathBuf, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = dir.join(format!("{label}-{ts}.jpg"));
    std::fs::write(&path, jpeg).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Encode recent JPEGs into a short mp4 via ffmpeg when available.
pub fn encode_clip_from_jpegs(jpeg_paths: &[PathBuf], out_mp4: &Path) -> Result<(), String> {
    if jpeg_paths.is_empty() {
        return Err("no frames to encode".into());
    }
    let list_path = out_mp4.with_extension("txt");
    let mut list = String::new();
    for p in jpeg_paths {
        list.push_str(&format!(
            "file '{}'\nduration 0.2\n",
            p.to_string_lossy().replace('\'', "'\\''")
        ));
    }
    // last file needs to be listed again for concat demuxer
    if let Some(last) = jpeg_paths.last() {
        list.push_str(&format!("file '{}'\n", last.to_string_lossy().replace('\'', "'\\''")));
    }
    std::fs::write(&list_path, list).map_err(|e| e.to_string())?;

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            list_path.to_str().unwrap_or(""),
            "-vf",
            "fps=5,scale=640:-2",
            "-pix_fmt",
            "yuv420p",
            out_mp4.to_str().unwrap_or("clip.mp4"),
        ])
        .status()
        .map_err(|e| format!("ffmpeg missing or failed to start: {e}"))?;

    if !status.success() {
        return Err("ffmpeg encode failed".into());
    }
    Ok(())
}
