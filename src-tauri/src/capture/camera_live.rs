//! Continuous webcam presence sampling (VIDEOINPUT WebcamSource spirit).
//!
//! Opens nokhwa once, keeps the stream open, and emits committed
//! [`LiveCameraSample`]s at ~`ANALYSIS_HZ` after `MIN_PERSISTENCE_S` of the
//! same candidate attention state.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use image::imageops::{self, FilterType};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

use super::camera::{aggregate_face_samples, detect_face_in_jpeg, mean_luma_rgb};
use super::jpeg_from_rgb;

/// Analysis cadence matching VIDEOINPUT `analysis_hz: 2.0`.
pub const ANALYSIS_HZ: f64 = 2.0;
/// VIDEOINPUT `min_persistence_s` — require ~4 samples at 2 Hz before commit.
pub const MIN_PERSISTENCE_S: f64 = 2.0;
const TARGET_WIDTH: u32 = 640;

/// One committed live reading for presence inference.
#[derive(Debug, Clone)]
pub struct LiveCameraSample {
    pub face_detected: bool,
    /// `"absent"` | `"present"` | `"looking_down"` | `"looking_away"`.
    pub attention: String,
    pub brightness: f64,
    /// Unix epoch seconds (fractional).
    pub ts: f64,
}

struct PersistenceGate {
    candidate_key: Option<String>,
    candidate_face: bool,
    candidate_attention: String,
    candidate_since: Instant,
    committed_face: bool,
    committed_attention: String,
    has_commit: bool,
}

impl PersistenceGate {
    fn new() -> Self {
        Self {
            candidate_key: None,
            candidate_face: false,
            candidate_attention: "absent".into(),
            candidate_since: Instant::now(),
            committed_face: false,
            committed_attention: "absent".into(),
            has_commit: false,
        }
    }

    fn key(face: bool, attention: &str) -> String {
        format!("{face}:{attention}")
    }

    /// Feed one raw candidate. Returns a committed sample when persistence allows.
    fn push(
        &mut self,
        face: bool,
        attention: &str,
        brightness: f64,
        ts: f64,
    ) -> Option<LiveCameraSample> {
        let key = Self::key(face, attention);
        if self.candidate_key.as_deref() != Some(key.as_str()) {
            self.candidate_key = Some(key);
            self.candidate_face = face;
            self.candidate_attention = attention.to_string();
            self.candidate_since = Instant::now();
        } else if self.candidate_since.elapsed()
            >= Duration::from_secs_f64(MIN_PERSISTENCE_S)
        {
            self.committed_face = self.candidate_face;
            self.committed_attention = self.candidate_attention.clone();
            self.has_commit = true;
        }

        if !self.has_commit {
            return None;
        }
        Some(LiveCameraSample {
            face_detected: self.committed_face,
            attention: self.committed_attention.clone(),
            brightness,
            ts,
        })
    }
}

fn now_epoch_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn classify_frame_jpeg(path: &std::path::Path) -> (bool, &'static str) {
    match detect_face_in_jpeg(path) {
        Some(sample) => match aggregate_face_samples(&[sample]) {
            Some((face, attn)) => (face, attn),
            None => (false, "absent"),
        },
        None => (false, "absent"),
    }
}

/// Open the webcam once and stream samples until `stop` is set.
///
/// Frames are grabbed continuously; only every `1/ANALYSIS_HZ` seconds is a
/// frame analyzed (backlog dropped). Missing frames log and continue — never
/// block the session forever.
pub fn run_live_camera_loop(
    stop: Arc<AtomicBool>,
    mut on_sample: impl FnMut(LiveCameraSample),
) -> Result<(), String> {
    if !nokhwa::nokhwa_check() {
        return Err("Camera permission is unavailable.".into());
    }

    let index = CameraIndex::Index(0);
    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut cam = Camera::new(index, requested).map_err(|e| format!("camera open: {e}"))?;
    cam.open_stream()
        .map_err(|e| format!("camera stream: {e}"))?;

    for _ in 0..5 {
        if stop.load(Ordering::SeqCst) {
            let _ = cam.stop_stream();
            return Ok(());
        }
        let _ = cam.frame();
    }

    let interval = Duration::from_secs_f64(1.0 / ANALYSIS_HZ);
    let mut next_emit = Instant::now();
    let mut gate = PersistenceGate::new();
    let mut misses = 0u32;
    let frame_path: PathBuf = std::env::temp_dir().join("waypoint-live-frame.jpg");

    tracing::info!(
        "live camera open · analysis_hz={ANALYSIS_HZ} persistence={MIN_PERSISTENCE_S}s"
    );

    while !stop.load(Ordering::SeqCst) {
        let frame = match cam.frame() {
            Ok(f) => f,
            Err(e) => {
                misses += 1;
                if misses == 1 || misses % 20 == 0 {
                    tracing::warn!("live camera frame miss ({misses}): {e}");
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        };
        misses = 0;

        let now = Instant::now();
        if now < next_emit {
            // Drop backlog — grab newest next iteration without analyzing.
            continue;
        }
        next_emit = now + interval;

        let decoded = match frame.decode_image::<RgbFormat>() {
            Ok(img) => img,
            Err(e) => {
                tracing::warn!("live camera decode: {e}");
                continue;
            }
        };

        let w = decoded.width();
        let h = decoded.height();
        let target_w = TARGET_WIDTH.min(w);
        let target_h = ((h as f32) * (target_w as f32 / w as f32)).round() as u32;
        let small = if target_w < w {
            imageops::resize(&decoded, target_w, target_h.max(1), FilterType::Triangle)
        } else {
            decoded
        };

        let brightness = mean_luma_rgb(small.as_raw(), 8).unwrap_or(128.0);
        let jpeg = match jpeg_from_rgb(small.width(), small.height(), small.as_raw()) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("live camera jpeg: {e}");
                continue;
            }
        };
        if let Err(e) = std::fs::write(&frame_path, &jpeg) {
            tracing::warn!("live camera write frame: {e}");
            continue;
        }

        let (face, attention) = classify_frame_jpeg(&frame_path);
        let ts = now_epoch_secs();
        if let Some(sample) = gate.push(face, attention, brightness, ts) {
            on_sample(sample);
        }
    }

    let _ = cam.stop_stream();
    let _ = std::fs::remove_file(&frame_path);
    tracing::info!("live camera stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_requires_stable_candidate() {
        let mut gate = PersistenceGate::new();
        // First tick — no commit yet.
        assert!(gate
            .push(true, "present", 80.0, 0.0)
            .is_none());
        // Same candidate but under 2s — still none.
        assert!(gate
            .push(true, "present", 81.0, 0.5)
            .is_none());
        // Force elapsed by rewriting since (unit test of commit path).
        gate.candidate_since = Instant::now() - Duration::from_secs(3);
        let sample = gate
            .push(true, "present", 82.0, 3.0)
            .expect("should commit after persistence");
        assert!(sample.face_detected);
        assert_eq!(sample.attention, "present");
        assert_eq!(sample.brightness, 82.0);
    }

    #[test]
    fn persistence_resets_on_attention_change() {
        let mut gate = PersistenceGate::new();
        // Arm candidate, then age it so the next same-key push commits.
        let _ = gate.push(true, "present", 80.0, 0.0);
        gate.candidate_since = Instant::now() - Duration::from_secs(3);
        let committed = gate
            .push(true, "present", 80.0, 3.0)
            .expect("commit present");
        assert_eq!(committed.attention, "present");
        // Flip to looking_away — resets timer; previous commit stays until new persists.
        let mid = gate
            .push(true, "looking_away", 80.0, 4.0)
            .expect("still emits last commit");
        assert_eq!(mid.attention, "present");
        gate.candidate_since = Instant::now() - Duration::from_secs(3);
        let next = gate
            .push(true, "looking_away", 80.0, 7.0)
            .expect("new commit");
        assert_eq!(next.attention, "looking_away");
    }

    #[test]
    fn analysis_interval_is_half_second() {
        let interval = Duration::from_secs_f64(1.0 / ANALYSIS_HZ);
        assert_eq!(interval, Duration::from_millis(500));
        assert!((MIN_PERSISTENCE_S * ANALYSIS_HZ - 4.0).abs() < 0.01);
    }
}
