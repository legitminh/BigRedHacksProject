//! Lightweight client for server-side camera accountability (`POST /v1/camera/observe`).

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::Deserialize;

use crate::api;
use crate::config::AppConfig;
use crate::presage::VitalsSnapshot;

#[derive(Debug, Deserialize)]
pub struct ObserveNudge {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct ObserveVitals {
    heart_rate: Option<f64>,
    breathing_rate: Option<f64>,
    stress_index: Option<f64>,
    stressed: Option<bool>,
    focus_ok: Option<bool>,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ObserveResponse {
    #[allow(dead_code)]
    pub ok: Option<bool>,
    pub presence: Option<String>,
    pub face_detected: Option<bool>,
    vitals: Option<ObserveVitals>,
    pub nudge: Option<ObserveNudge>,
    pub watching_note: Option<String>,
}

impl ObserveResponse {
    /// Build a session vitals snapshot from Presage scalars and/or presence alone.
    /// Presence-only responses still record accountability so summaries don't claim camera was off.
    pub fn to_vitals(&self) -> Option<VitalsSnapshot> {
        let presence = self.presence.as_deref().unwrap_or("uncertain");
        let summary = format!("presence={presence} face={:?}", self.face_detected);
        if let Some(v) = self.vitals.as_ref() {
            return Some(VitalsSnapshot {
                heart_rate: v.heart_rate,
                breathing_rate: v.breathing_rate,
                hrv_rmssd: None,
                stress_index: v.stress_index,
                stressed: v.stressed.unwrap_or(false),
                focus_ok: v.focus_ok.unwrap_or(true),
                raw_summary: summary,
                source: v.source.clone().unwrap_or_else(|| "presage".into()),
            });
        }
        if self.presence.is_some() || self.watching_note.is_some() {
            return Some(VitalsSnapshot {
                heart_rate: None,
                breathing_rate: None,
                hrv_rmssd: None,
                stress_index: None,
                stressed: false,
                focus_ok: presence == "present",
                raw_summary: summary,
                source: "presence".into(),
            });
        }
        None
    }
}

/// Upload a short webcam clip to the API. Server runs Presage + presence; bytes are not kept.
pub async fn observe_clip(
    cfg: &AppConfig,
    session_id: &str,
    phase: &str,
    video_path: &Path,
) -> Result<ObserveResponse, String> {
    let bytes = tokio::fs::read(video_path)
        .await
        .map_err(|e| format!("read clip: {e}"))?;
    if bytes.is_empty() {
        return Err("empty clip".into());
    }
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("clip exceeds 8MB upload limit".into());
    }
    let data_base64 = B64.encode(&bytes);
    let body = serde_json::json!({
        "session_id": session_id,
        "phase": phase,
        "mime": "video/mp4",
        "data_base64": data_base64,
    });
    api::authed_json::<ObserveResponse>(
        cfg,
        reqwest::Method::POST,
        "/v1/camera/observe",
        Some(&body),
    )
    .await
}
