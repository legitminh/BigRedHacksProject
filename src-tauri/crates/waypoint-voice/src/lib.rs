//! Rudimentary local voice stack for Waypoint.
//!
//! - **TTS**: macOS `say` (swap for Grok Voice later)
//! - **STT**: short mic clip via `ffmpeg` + placeholder transcript
//!   (swap `StubTranscriber` for a Grok Voice / cloud STT impl)

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, VoiceError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    pub engine: String,
    pub audio_path: Option<String>,
    pub note: String,
}

/// Speak coach / UI text out loud. Non-blocking.
/// Always cuts off any previous `say` so lines never stack / talk over each other.
pub fn speak(text: &str) -> Result<()> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Ok(());
    }
    // Keep utterances short so popups stay snappy.
    let snippet: String = cleaned.chars().take(220).collect();

    #[cfg(target_os = "macos")]
    {
        stop_speaking();
        Command::new("say")
            .args(["-v", "Samantha", &snippet])
            .spawn()
            .map_err(|e| VoiceError::Message(format!("macOS say failed to start: {e}")))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = snippet;
        Err(VoiceError::Message(
            "Local TTS is only wired for macOS `say` right now.".into(),
        ))
    }
}

pub fn stop_speaking() {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("killall").arg("say").status();
    }
}

/// Trait so Grok Voice (or Whisper, etc.) can replace the stub later.
pub trait Transcriber: Send + Sync {
    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcript>;
}

/// Placeholder STT — proves the record → transcript pipeline before Grok Voice.
pub struct StubTranscriber;

impl Transcriber for StubTranscriber {
    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcript> {
        let meta = std::fs::metadata(audio_path)
            .map_err(|e| VoiceError::Message(format!("read audio: {e}")))?;
        Ok(Transcript {
            text: String::new(),
            engine: "stub".into(),
            audio_path: Some(audio_path.display().to_string()),
            note: format!(
                "Recorded {} bytes. Stub STT — replace with Grok Voice / real transcription.",
                meta.len()
            ),
        })
    }
}

/// Record a short mic clip (macOS avfoundation) then run the transcriber.
pub fn listen_once(seconds: u64, transcriber: &dyn Transcriber) -> Result<Transcript> {
    let seconds = seconds.clamp(2, 12);
    let path = temp_wav_path()?;
    record_mic_wav(&path, seconds)?;
    transcriber.transcribe_file(&path)
}

fn temp_wav_path() -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dir = std::env::temp_dir().join("waypoint-voice");
    std::fs::create_dir_all(&dir).map_err(|e| VoiceError::Message(e.to_string()))?;
    Ok(dir.join(format!("listen-{ts}.wav")))
}

fn record_mic_wav(path: &Path, seconds: u64) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Default mic (`:0`) via ffmpeg avfoundation.
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "avfoundation",
                "-i",
                ":0",
                "-t",
                &seconds.to_string(),
                "-ac",
                "1",
                "-ar",
                "16000",
                path.to_str().unwrap_or("clip.wav"),
            ])
            .status()
            .map_err(|e| {
                VoiceError::Message(format!(
                    "ffmpeg mic capture failed ({e}). Install ffmpeg and allow Microphone access."
                ))
            })?;
        if !status.success() {
            return Err(VoiceError::Message(
                "Mic recording failed. Allow Microphone for Waypoint / Terminal and ensure ffmpeg works."
                    .into(),
            ));
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, seconds);
        Err(VoiceError::Message(
            "Mic STT recording is only wired for macOS + ffmpeg right now.".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_notes_missing_file() {
        let t = StubTranscriber;
        assert!(t.transcribe_file(Path::new("/tmp/definitely-missing-waypoint.wav")).is_err());
    }
}
