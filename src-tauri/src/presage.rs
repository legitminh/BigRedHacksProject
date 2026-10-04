use std::path::Path;
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::sleep;

use crate::config::AppConfig;

const BASE: &str = "https://api.physiology.presagetech.com";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VitalsSnapshot {
    pub heart_rate: Option<f64>,
    pub breathing_rate: Option<f64>,
    pub hrv_rmssd: Option<f64>,
    pub stress_index: Option<f64>,
    pub stressed: bool,
    pub focus_ok: bool,
    pub raw_summary: String,
    pub source: String,
}

pub struct PresageClient {
    http: Client,
    api_key: String,
}

impl PresageClient {
    pub fn from_config(cfg: &AppConfig) -> Result<Self, String> {
        let api_key = cfg
            .presage_api_key
            .clone()
            .ok_or_else(|| "PRESAGE_API_KEY is not set".to_string())?;
        Ok(Self {
            http: Client::new(),
            api_key,
        })
    }

    pub fn configured(cfg: &AppConfig) -> bool {
        cfg.presage_api_key.is_some()
    }

    /// Upload a short webcam video for HR/RR (and related) processing.
    pub async fn queue_video_hr_rr(&self, video_path: &Path) -> Result<String, String> {
        let file_bytes = tokio::fs::read(video_path)
            .await
            .map_err(|e| format!("read video: {e}"))?;
        let file_size = file_bytes.len();

        let headers = [("x-api-key", self.api_key.as_str())];
        // Presage retired /v1/upload-url (API Gateway 403 Missing Authentication Token).
        let start = self
            .http
            .post(format!("{BASE}/v2/upload-url"))
            .headers(
                headers
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            reqwest::header::HeaderName::from_static(k),
                            reqwest::header::HeaderValue::from_str(v).unwrap(),
                        )
                    })
                    .collect(),
            )
            .json(&json!({
                "file_size": file_size,
                "hr_br": { "to_process": true }
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if start.status().as_u16() == 401 {
            return Err("Presage unauthorized — check PRESAGE_API_KEY".into());
        }
        if !start.status().is_success() {
            return Err(format!(
                "Presage upload-url failed: {} {}",
                start.status(),
                start.text().await.unwrap_or_default()
            ));
        }

        let start_json: Value = start.json().await.map_err(|e| e.to_string())?;
        let vid_id = start_json["id"]
            .as_str()
            .ok_or("missing id")?
            .to_string();
        let upload_id = start_json["upload_id"]
            .as_str()
            .ok_or("missing upload_id")?
            .to_string();
        let urls = start_json["urls"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut parts = Vec::new();
        let chunk = 5 * 1024 * 1024;
        let mut offset = 0usize;
        for (idx, url_val) in urls.iter().enumerate() {
            let url = url_val.as_str().ok_or("bad upload url")?;
            let end = (offset + chunk).min(file_bytes.len());
            let slice = &file_bytes[offset..end];
            let put = self
                .http
                .put(url)
                .body(slice.to_vec())
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !put.status().is_success() {
                return Err(format!("Presage part upload failed: {}", put.status()));
            }
            let etag = put
                .headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("\"\"")
                .to_string();
            parts.push(json!({ "ETag": etag, "PartNumber": idx + 1 }));
            offset = end;
            if offset >= file_bytes.len() {
                break;
            }
        }

        let complete = self
            .http
            .post(format!("{BASE}/v2/complete"))
            .header("x-api-key", &self.api_key)
            .json(&json!({
                "id": vid_id,
                "upload_id": upload_id,
                "parts": parts
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !complete.status().is_success() {
            return Err(format!(
                "Presage complete failed: {} {}",
                complete.status(),
                complete.text().await.unwrap_or_default()
            ));
        }
        Ok(vid_id)
    }

    /// Queue a preprocessed JSON trace (gzipped) via v2 API.
    #[allow(dead_code)]
    pub async fn queue_trace(
        &self,
        trace: &Value,
        metrics: &[&str],
    ) -> Result<String, String> {
        let raw = serde_json::to_vec(trace).map_err(|e| e.to_string())?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        use std::io::Write;
        encoder.write_all(&raw).map_err(|e| e.to_string())?;
        let compressed = encoder.finish().map_err(|e| e.to_string())?;

        let start = self
            .http
            .post(format!("{BASE}/v2/upload-url"))
            .header("x-api-key", &self.api_key)
            .json(&json!({
                "file_size": compressed.len(),
                "metrics": metrics
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !start.status().is_success() {
            return Err(format!(
                "Presage v2 upload-url failed: {} {}",
                start.status(),
                start.text().await.unwrap_or_default()
            ));
        }
        let start_json: Value = start.json().await.map_err(|e| e.to_string())?;
        let vid_id = start_json["id"].as_str().ok_or("missing id")?.to_string();
        let upload_id = start_json["upload_id"]
            .as_str()
            .ok_or("missing upload_id")?
            .to_string();
        let urls = start_json["urls"].as_array().cloned().unwrap_or_default();

        let mut parts = Vec::new();
        let chunk = 5 * 1024 * 1024;
        let mut offset = 0usize;
        for (idx, url_val) in urls.iter().enumerate() {
            let url = url_val.as_str().ok_or("bad upload url")?;
            let end = (offset + chunk).min(compressed.len());
            let put = self
                .http
                .put(url)
                .body(compressed[offset..end].to_vec())
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !put.status().is_success() {
                return Err(format!("Presage part upload failed: {}", put.status()));
            }
            let etag = put
                .headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("\"\"")
                .to_string();
            parts.push(json!({ "ETag": etag, "PartNumber": idx + 1 }));
            offset = end;
            if offset >= compressed.len() {
                break;
            }
        }

        self.http
            .post(format!("{BASE}/v2/complete"))
            .header("x-api-key", &self.api_key)
            .json(&json!({
                "id": vid_id,
                "upload_id": upload_id,
                "parts": parts
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(vid_id)
    }

    pub async fn retrieve_result(
        &self,
        id: &str,
        timeout_secs: u64,
    ) -> Result<Value, String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if std::time::Instant::now() >= deadline {
                return Err("Presage retrieve timeout".into());
            }
            let res = self
                .http
                .post(format!("{BASE}/retrieve-data"))
                .header("x-api-key", &self.api_key)
                .json(&json!({ "id": id, "reshape": false }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            match res.status().as_u16() {
                200 => {
                    return res.json().await.map_err(|e| e.to_string());
                }
                201 => sleep(Duration::from_secs(1)).await,
                401 => return Err("Presage unauthorized".into()),
                _ => sleep(Duration::from_secs(1)).await,
            }
        }
    }

    pub fn vitals_from_result(data: &Value) -> VitalsSnapshot {
        // Presage payloads vary; probe common shapes.
        let hr = first_f64(data, &["hr", "heart_rate", "pulse_rate", "pulse"]);
        let rr = first_f64(data, &["rr", "br", "breathing_rate", "respiration_rate"]);
        let hrv = first_f64(data, &["hrv", "rmssd", "hrv_rmssd"]);
        let stress = first_f64(data, &["stress", "stress_index", "baevsky", "baevsky_stress_index"]);

        let stressed = stress.map(|s| s > 150.0).unwrap_or(false)
            || hrv.map(|h| h < 20.0).unwrap_or(false);
        let focus_ok = !stressed && hr.map(|h| (50.0..=110.0).contains(&h)).unwrap_or(true);

        VitalsSnapshot {
            heart_rate: hr,
            breathing_rate: rr,
            hrv_rmssd: hrv,
            stress_index: stress,
            stressed,
            focus_ok,
            raw_summary: summarize(hr, rr, hrv, stress),
            source: "presage".into(),
        }
    }
}

fn summarize(
    hr: Option<f64>,
    rr: Option<f64>,
    hrv: Option<f64>,
    stress: Option<f64>,
) -> String {
    format!(
        "HR={:?} RR={:?} HRV/RMSSD={:?} stress_index={:?}",
        hr, rr, hrv, stress
    )
}

fn first_f64(v: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(n) = v.get(*key).and_then(|x| x.as_f64()) {
            return Some(n);
        }
        if let Some(n) = v.get(*key).and_then(|x| x.as_i64()) {
            return Some(n as f64);
        }
        if let Some(arr) = v.get(*key).and_then(|x| x.as_array()) {
            if let Some(n) = arr.last().and_then(|x| x.as_f64()) {
                return Some(n);
            }
        }
    }
    // Deep-ish scan of object values
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            let lk = k.to_lowercase();
            if keys.iter().any(|want| lk.contains(want)) {
                if let Some(n) = val.as_f64() {
                    return Some(n);
                }
                if let Some(arr) = val.as_array() {
                    if let Some(n) = arr.iter().rev().find_map(|x| x.as_f64()) {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

/// Local fallback when Presage key/video unavailable — informational only.
pub fn fallback_vitals(stress_cue: bool) -> VitalsSnapshot {
    VitalsSnapshot {
        heart_rate: Some(if stress_cue { 92.0 } else { 72.0 }),
        breathing_rate: Some(if stress_cue { 18.0 } else { 14.0 }),
        hrv_rmssd: Some(if stress_cue { 18.0 } else { 42.0 }),
        stress_index: Some(if stress_cue { 180.0 } else { 80.0 }),
        stressed: stress_cue,
        focus_ok: !stress_cue,
        raw_summary: if stress_cue {
            "fallback: elevated stress cues from vision".into()
        } else {
            "fallback: calm baseline (Presage pending/unavailable)".into()
        },
        source: "fallback".into(),
    }
}
