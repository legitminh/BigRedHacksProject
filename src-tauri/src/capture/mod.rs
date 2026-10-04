pub mod camera;
pub mod frontmost;
pub mod ocr;
pub mod screen;

use std::env;
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

#[allow(dead_code)]
pub fn save_jpeg(dir: &Path, label: &str, jpeg: &[u8]) -> Result<PathBuf, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = dir.join(format!("{label}-{ts}.jpg"));
    std::fs::write(&path, jpeg).map_err(|e| e.to_string())?;
    Ok(path)
}

fn encode_clip_binary() -> Option<PathBuf> {
    let baked = env!("WAYPOINT_ENCODE_CLIP_BIN");
    let baked_path = PathBuf::from(baked);
    if baked_path.is_file() {
        return Some(baked_path);
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("waypoint-encode-clip");
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/waypoint-encode-clip");
    if manifest.is_file() {
        return Some(manifest);
    }
    None
}

fn encode_with_avfoundation(
    jpeg_paths: &[PathBuf],
    out_mp4: &Path,
    fps: u32,
) -> Result<(), String> {
    let bin = encode_clip_binary().ok_or_else(|| {
        "bundled clip encoder missing — rebuild on macOS so waypoint-encode-clip is compiled"
            .to_string()
    })?;
    let mut args: Vec<String> = vec![
        fps.to_string(),
        out_mp4.to_string_lossy().into_owned(),
    ];
    for p in jpeg_paths {
        args.push(p.to_string_lossy().into_owned());
    }
    let output = Command::new(&bin)
        .args(&args)
        .output()
        .map_err(|e| format!("clip encoder failed to start ({e})"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("clip encoder failed: {err}"));
    }
    Ok(())
}

fn encode_with_ffmpeg(jpeg_paths: &[PathBuf], out_mp4: &Path, fps: u32) -> Result<(), String> {
    let fps = fps.max(10);
    let duration = format!("{:.6}", 1.0 / f64::from(fps));
    let list_path = out_mp4.with_extension("txt");
    let mut list = String::new();
    for p in jpeg_paths {
        list.push_str(&format!(
            "file '{}'\nduration {duration}\n",
            p.to_string_lossy().replace('\'', "'\\''")
        ));
    }
    if let Some(last) = jpeg_paths.last() {
        list.push_str(&format!(
            "file '{}'\n",
            last.to_string_lossy().replace('\'', "'\\''")
        ));
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
            &format!("fps={fps},scale=640:-2"),
            "-pix_fmt",
            "yuv420p",
            out_mp4.to_str().unwrap_or("clip.mp4"),
        ])
        .status()
        .map_err(|e| format!("ffmpeg missing or failed to start ({e})"))?;

    if !status.success() {
        return Err("ffmpeg encode failed".into());
    }
    Ok(())
}

/// Encode JPEG frames into an mp4. Prefers bundled AVFoundation helper (every Mac);
/// falls back to ffmpeg only if that helper is missing (dev/CI).
pub fn encode_clip_from_jpegs(
    jpeg_paths: &[PathBuf],
    out_mp4: &Path,
    fps: u32,
) -> Result<(), String> {
    if jpeg_paths.is_empty() {
        return Err("no frames to encode".into());
    }
    let fps = fps.max(10);
    match encode_with_avfoundation(jpeg_paths, out_mp4, fps) {
        Ok(()) => Ok(()),
        Err(av_err) => {
            tracing::warn!("AVFoundation clip encode failed ({av_err}); trying ffmpeg");
            encode_with_ffmpeg(jpeg_paths, out_mp4, fps).map_err(|ff_err| {
                format!("{av_err}; ffmpeg fallback also failed: {ff_err}")
            })
        }
    }
}
