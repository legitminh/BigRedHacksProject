//! Local voice stack for Waypoint.
//!
//! - **TTS (fallback)**: macOS `say`
//! - **TTS (study heads-ups)**: xAI / Grok audio played via `afplay` (fetched by coach)
//! - **STT**: macOS Speech framework via a small Swift helper (AVAudioRecorder +
//!   SFSpeechRecognizer). `Transcriber` remains the swap point for Whisper later.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bumps whenever a new utterance starts or stop is requested — stale afplay exits early.
static PLAYBACK_GENERATION: AtomicU64 = AtomicU64::new(1);

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

#[derive(Debug, Deserialize)]
struct HelperPayload {
    text: String,
    engine: String,
    audio_path: Option<String>,
    note: String,
    error: Option<String>,
}

/// Keep coach lines short so `say` starts quickly and overlays stay snappy.
fn speak_snippet(text: &str) -> String {
    text.trim().chars().take(160).collect()
}

/// Local macOS `say` TTS — Settings “Test speak” and heads-up fallback.
/// Non-blocking. Always cuts off any previous utterance so lines never stack.
pub fn speak(text: &str) -> Result<()> {
    let snippet = speak_snippet(text);
    if snippet.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        // Wait for killall to finish so we don't SIGKILL the new `say`.
        stop_speaking_sync();
        let _ = PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst);
        Command::new("say")
            .args(["-r", "200", &snippet])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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

/// Bundled soft two-note chime (not a macOS system sound).
const REFOCUS_DING_WAV: &[u8] = include_bytes!("../assets/refocus-ding.wav");

fn refocus_ding_path() -> Result<PathBuf> {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    if let Some(p) = PATH.get() {
        return Ok(p.clone());
    }
    let dir = std::env::temp_dir().join("waypoint-voice");
    std::fs::create_dir_all(&dir)
        .map_err(|e| VoiceError::Message(format!("ding cache dir: {e}")))?;
    let path = dir.join("refocus-ding.wav");
    // Always rewrite so app updates replace a stale cached file.
    std::fs::write(&path, REFOCUS_DING_WAV)
        .map_err(|e| VoiceError::Message(format!("write ding wav: {e}")))?;
    let _ = PATH.set(path.clone());
    Ok(path)
}

/// Short positive chime when the student returns to task — no speech, no overlay.
/// Custom baked-in WAV (respect silent mode at the call site in coach).
pub fn play_positive_ding() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Cut spoken heads-ups only; don't killall afplay (that would race this ding).
        let _ = Command::new("killall")
            .args(["-9", "say"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let sound = refocus_ding_path()?;
        if !sound.is_file() {
            return Err(VoiceError::Message("refocus ding sound missing".into()));
        }
        Command::new("afplay")
            .args(["-v", "0.55"])
            .arg(&sound)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| VoiceError::Message(format!("ding afplay failed: {e}")))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(VoiceError::Message(
            "Positive ding is only wired for macOS right now.".into(),
        ))
    }
}

/// Play raw audio bytes from xAI / Grok TTS (typically MP3). Non-blocking.
/// Used only by study heads-up / nudge audio — not Settings test speak.
pub fn play_audio_bytes(bytes: &[u8], extension: &str) -> Result<()> {
    if bytes.is_empty() {
        return Err(VoiceError::Message("empty audio payload".into()));
    }

    #[cfg(target_os = "macos")]
    {
        stop_speaking_sync();
        let generation = PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let ext = if extension.trim().is_empty() {
            "mp3"
        } else {
            extension.trim().trim_start_matches('.')
        };
        let path = temp_audio_path(ext)?;
        std::fs::write(&path, bytes)
            .map_err(|e| VoiceError::Message(format!("write TTS audio: {e}")))?;
        // Block so a bad payload (JSON envelope, wrong codec) fails this call
        // and the coach can fall back to `say`. A newer utterance bumps the
        // generation and must not be treated as a playback error.
        let status = Command::new("afplay")
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| VoiceError::Message(format!("afplay failed to start: {e}")))?;
        let _ = std::fs::remove_file(&path);
        if PLAYBACK_GENERATION.load(Ordering::SeqCst) != generation {
            return Ok(());
        }
        if !status.success() {
            return Err(VoiceError::Message(format!(
                "afplay could not play {ext} audio ({status})"
            )));
        }
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = extension;
        Err(VoiceError::Message(
            "Grok TTS playback is only wired for macOS `afplay` right now.".into(),
        ))
    }
}

fn temp_audio_path(ext: &str) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dir = std::env::temp_dir().join("waypoint-voice");
    std::fs::create_dir_all(&dir).map_err(|e| VoiceError::Message(e.to_string()))?;
    Ok(dir.join(format!("heads-up-{ts}.{ext}")))
}

/// Blocking TTS — used by the Settings “Test speak” button so success means audio finished.
pub fn speak_wait(text: &str) -> Result<()> {
    let snippet = speak_snippet(text);
    if snippet.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        stop_speaking_sync();
        let output = Command::new("say")
            .args(["-r", "200", &snippet])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| VoiceError::Message(format!("macOS say failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                format!("exit {}", output.status)
            } else {
                stderr
            };
            return Err(VoiceError::Message(format!(
                "macOS say failed ({detail}). Check System Settings → Accessibility → Spoken Content."
            )));
        }
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

/// Stop any in-flight `say` / `afplay` and wait until killall returns (so a new speak is safe).
pub fn stop_speaking_sync() {
    #[cfg(target_os = "macos")]
    {
        let _ = PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst);
        for bin in ["say", "afplay"] {
            let _ = Command::new("killall")
                .args(["-9", bin])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        // Brief settle — killall can return before the process table updates.
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
}

pub fn stop_speaking() {
    // Coach path: same sync stop so we never race-kill the replacement utterance.
    stop_speaking_sync();
}

/// Trait so Grok Voice (or Whisper, etc.) can replace the default later.
pub trait Transcriber: Send + Sync {
    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcript>;
}

/// Legacy stub — kept for tests / offline unit checks. Prefer [`MacSpeechTranscriber`].
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
                "Recorded {} bytes. Stub STT — production uses MacSpeechTranscriber.",
                meta.len()
            ),
        })
    }
}

/// Production STT on macOS: Speech framework (on-device when available).
pub struct MacSpeechTranscriber;

impl Transcriber for MacSpeechTranscriber {
    fn transcribe_file(&self, audio_path: &Path) -> Result<Transcript> {
        #[cfg(target_os = "macos")]
        {
            let helper = ensure_listen_helper()?;
            let path = audio_path
                .to_str()
                .ok_or_else(|| VoiceError::Message("audio path is not UTF-8".into()))?;
            let output = Command::new(&helper)
                .args(["--transcribe-file", path])
                .output()
                .map_err(|e| {
                    VoiceError::Message(format!("macOS speech helper failed to start: {e}"))
                })?;
            parse_helper_output(&output)
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = audio_path;
            Err(VoiceError::Message(
                "MacSpeechTranscriber is only available on macOS.".into(),
            ))
        }
    }
}

/// Production listen: record + recognize (macOS Speech helper, no ffmpeg).
pub fn listen_once_default(seconds: u64) -> Result<Transcript> {
    let seconds = seconds.clamp(2, 12);
    #[cfg(target_os = "macos")]
    {
        listen_once_macos(seconds)
    }
    #[cfg(not(target_os = "macos"))]
    {
        listen_once(seconds, &StubTranscriber)
    }
}

/// Record a short mic clip then run the given transcriber.
/// Prefer [`listen_once_default`] on macOS (integrated Speech helper).
pub fn listen_once(seconds: u64, transcriber: &dyn Transcriber) -> Result<Transcript> {
    let seconds = seconds.clamp(2, 12);
    let path = temp_wav_path()?;
    record_mic_wav(&path, seconds)?;
    transcriber.transcribe_file(&path)
}

/// Compile the macOS speech helper ahead of the first mic test (no recording).
pub fn warm_speech_helper() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        ensure_listen_helper().map(|_| ())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// Microphone TCC status: `authorized` | `denied` | `restricted` | `notDetermined` | `unknown`.
/// Does not compile the speech helper — only probes when the binary already exists,
/// so Permissions refresh stays snappy before the first Voice test.
pub fn microphone_permission_status() -> String {
    #[cfg(target_os = "macos")]
    {
        let bin = std::env::temp_dir()
            .join("waypoint-voice")
            .join("waypoint-listen-once");
        if !bin.exists() {
            return "notDetermined".into();
        }
        match Command::new(&bin).arg("--check-mic").output() {
            Ok(out) if out.status.success() => {
                let label = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if label.is_empty() {
                    "unknown".into()
                } else {
                    label
                }
            }
            _ => "unknown".into(),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        "unknown".into()
    }
}

#[cfg(target_os = "macos")]
fn listen_once_macos(seconds: u64) -> Result<Transcript> {
    let helper = ensure_listen_helper()?;
    let output = Command::new(&helper)
        .arg(seconds.to_string())
        .output()
        .map_err(|e| {
            VoiceError::Message(format!(
                "macOS speech helper failed to start ({e}). Is the Swift toolchain installed?"
            ))
        })?;
    parse_helper_output(&output)
}

fn parse_helper_output(output: &std::process::Output) -> Result<Transcript> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if stdout.is_empty() {
        let detail = if stderr.is_empty() {
            format!("exit {}", output.status)
        } else {
            stderr
        };
        return Err(VoiceError::Message(format!(
            "Speech helper returned no result ({detail})."
        )));
    }

    let payload: HelperPayload = serde_json::from_str(&stdout).map_err(|e| {
        VoiceError::Message(format!(
            "Speech helper returned invalid JSON ({e}): {stdout}"
        ))
    })?;

    if let Some(err) = payload.error.filter(|e| !e.trim().is_empty()) {
        return Err(VoiceError::Message(err));
    }

    Ok(Transcript {
        text: payload.text,
        engine: payload.engine,
        audio_path: payload.audio_path,
        note: payload.note,
    })
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
        // Fallback for custom Transcriber pipelines only.
        if which("rec") {
            let status = Command::new("rec")
                .args([
                    "-q",
                    "-r",
                    "16000",
                    "-c",
                    "1",
                    path.to_str().unwrap_or("clip.wav"),
                    "trim",
                    "0",
                    &seconds.to_string(),
                ])
                .status()
                .map_err(|e| VoiceError::Message(format!("rec (sox) failed: {e}")))?;
            if status.success() {
                return Ok(());
            }
        }
        if which("ffmpeg") {
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
                        "ffmpeg mic capture failed ({e}). Allow Microphone access."
                    ))
                })?;
            if status.success() {
                return Ok(());
            }
            return Err(VoiceError::Message(
                "Mic recording failed. Allow Microphone for Waypoint in System Settings.".into(),
            ));
        }
        Err(VoiceError::Message(
            "No mic recorder available for custom transcribers (install ffmpeg or sox).".into(),
        ))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, seconds);
        Err(VoiceError::Message(
            "Mic STT recording is only wired for macOS right now.".into(),
        ))
    }
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn ensure_listen_helper() -> Result<PathBuf> {
    static HELPER: OnceLock<std::result::Result<PathBuf, String>> = OnceLock::new();
    match HELPER.get_or_init(|| compile_listen_helper().map_err(|e| e.to_string())) {
        Ok(path) => Ok(path.clone()),
        Err(msg) => Err(VoiceError::Message(msg.clone())),
    }
}

#[cfg(target_os = "macos")]
fn compile_listen_helper() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("waypoint-voice");
    std::fs::create_dir_all(&dir).map_err(|e| VoiceError::Message(e.to_string()))?;
    let src_path = dir.join("ListenOnce.swift");
    let bin_path = dir.join("waypoint-listen-once");
    let source = include_str!("../macos/ListenOnce.swift");

    let needs_write = match std::fs::read_to_string(&src_path) {
        Ok(existing) => existing != source,
        Err(_) => true,
    };
    if needs_write {
        let mut f = std::fs::File::create(&src_path)
            .map_err(|e| VoiceError::Message(format!("write helper source: {e}")))?;
        f.write_all(source.as_bytes())
            .map_err(|e| VoiceError::Message(format!("write helper source: {e}")))?;
    }

    let needs_compile = needs_write || !bin_path.exists();
    if needs_compile {
        let status = Command::new("swiftc")
            .args([
                "-O",
                "-framework",
                "Speech",
                "-framework",
                "AVFoundation",
                "-framework",
                "Foundation",
                "-o",
            ])
            .arg(&bin_path)
            .arg(&src_path)
            .status()
            .map_err(|e| {
                VoiceError::Message(format!(
                    "swiftc failed ({e}). Install Xcode CLT to enable mic speech tests."
                ))
            })?;
        if !status.success() {
            return Err(VoiceError::Message(
                "Failed to compile macOS speech helper. Install Xcode Command Line Tools (`xcode-select --install`)."
                    .into(),
            ));
        }
    }

    Ok(bin_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_notes_missing_file() {
        let t = StubTranscriber;
        assert!(t
            .transcribe_file(Path::new("/tmp/definitely-missing-waypoint.wav"))
            .is_err());
    }
}
