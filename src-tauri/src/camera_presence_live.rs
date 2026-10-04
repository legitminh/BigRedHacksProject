//! Local PresenceInference port (VIDEOINPUT `presence.py` + demo timings).
//!
//! Consumes committed [`LiveCameraSample`]s at ~2 Hz and emits Waypoint nudge
//! kinds: `look_back`, `left_desk`, `left_desk_pause`, `suggest_break`,
//! `camera_obstructed`, `welcome_back`.

use std::collections::HashSet;

use crate::capture::camera_live::LiveCameraSample;

/// Desk / gaze presence for live webcam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivePresenceState {
    Present,
    LeftFrame,
    CameraObstructed,
    HeadTurned,
}

impl LivePresenceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::LeftFrame => "left_frame",
            Self::CameraObstructed => "camera_obstructed",
            Self::HeadTurned => "head_turned",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceNudge {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ObserveOutcome {
    pub presence: LivePresenceState,
    pub nudge: Option<PresenceNudge>,
    pub watching_note: String,
}

/// Demo-friendly timings close to VIDEOINPUT demo profile + user overrides.
#[derive(Debug, Clone)]
pub struct LivePresenceConfig {
    /// Brightness below this (measured) ⇒ camera_obstructed.
    pub obstructed_brightness: f64,
    /// Hold obstructed before speaking (user: 8s, not Dhanvi's 30s).
    pub obstructed_speak_s: f64,
    /// Ignore brief looking_away glances before first look_back.
    pub look_away_ignore_s: f64,
    /// looking_down hold before phone-hedge look_back.
    pub look_down_ignore_s: f64,
    /// Confirm left_frame after sustained absence (demo ~3s).
    pub away_confirm_s: f64,
    pub return_confirm_s: f64,
    pub first_callback_s: f64,
    pub second_callback_s: f64,
    pub pause_after_s: f64,
    pub welcome_back_min_absence_s: f64,
}

impl Default for LivePresenceConfig {
    fn default() -> Self {
        Self {
            obstructed_brightness: 25.0,
            obstructed_speak_s: 8.0,
            // Dhanvi PresenceInference speaks head_turned on the first committed
            // sample (AttentionModel's 8s glance ignore is unused on the live path).
            // Live loop already persists ~2s before emitting looking_away.
            look_away_ignore_s: 0.0,
            look_down_ignore_s: 5.0,
            away_confirm_s: 3.0,
            return_confirm_s: 2.0,
            // Demo-friendly (~10–25s band); closer to demo's 10s.
            first_callback_s: 12.0,
            second_callback_s: 30.0,
            pause_after_s: 90.0,
            welcome_back_min_absence_s: 8.0,
        }
    }
}

const LOOK_AWAY_TEXT: &str = "You're looking away. Turn back to the work.";
const LOOK_DOWN_TEXT: &str = "Eyes on the work — put the phone down if you're on it.";
const CAMERA_OBSTRUCTED_TEXT: &str =
    "I can't see you clearly. Check the camera or lighting.";
const LEFT_DESK_TEXT: &str = "You've stepped away. Come back to the work when you can.";
const SUGGEST_BREAK_TEXT: &str =
    "Still away — optional five-minute break so you know when to return?";
const LEFT_DESK_PAUSE_TEXT: &str = "I'll stay quiet until you're back at the desk.";
const WELCOME_BACK_TEXT: &str =
    "Welcome back — good to see you. Let's pick the work back up.";

fn watching_note(state: LivePresenceState) -> String {
    match state {
        LivePresenceState::Present => "Camera accountability · present".into(),
        LivePresenceState::LeftFrame => "Camera accountability · away from desk".into(),
        LivePresenceState::CameraObstructed => {
            "Camera accountability · camera unclear".into()
        }
        // Face still in frame — keep "present" so UI stays healthy.
        LivePresenceState::HeadTurned => {
            "Camera accountability · present · looking away".into()
        }
    }
}

fn is_silent_phase(phase: &str) -> bool {
    matches!(phase, "paused" | "break")
}

fn is_gaze_away(attention: &str) -> bool {
    matches!(attention, "looking_away" | "looking_down")
}

/// Local VIDEOINPUT-style presence ladder driven by live samples.
pub struct LivePresenceInference {
    pub config: LivePresenceConfig,
    pub state: LivePresenceState,
    absent_since: Option<f64>,
    away_candidate_since: Option<f64>,
    present_since: Option<f64>,
    obstructed_since: Option<f64>,
    obstructed_said: bool,
    turned_since: Option<f64>,
    turned_attention: Option<String>,
    left_confirmed: bool,
    welcomed_back: bool,
    callbacks: HashSet<String>,
    lines_spoken: u32,
    ladder_quiet: bool,
}

impl LivePresenceInference {
    pub fn new(config: LivePresenceConfig) -> Self {
        Self {
            config,
            state: LivePresenceState::Present,
            absent_since: None,
            away_candidate_since: None,
            present_since: None,
            obstructed_since: None,
            obstructed_said: false,
            turned_since: None,
            turned_attention: None,
            left_confirmed: false,
            welcomed_back: false,
            callbacks: HashSet::new(),
            lines_spoken: 0,
            ladder_quiet: false,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LivePresenceConfig::default())
    }

    fn classify(&self, sample: &LiveCameraSample) -> LivePresenceState {
        let face = sample.face_detected;
        let attn = sample.attention.as_str();
        // Face + gaze-away ⇒ head_turned (not left_desk).
        if face && is_gaze_away(attn) {
            return LivePresenceState::HeadTurned;
        }
        if face {
            return LivePresenceState::Present;
        }
        // Dark / covered lens when no face.
        if sample.brightness < self.config.obstructed_brightness {
            return LivePresenceState::CameraObstructed;
        }
        if attn == "absent" || !face {
            return LivePresenceState::LeftFrame;
        }
        LivePresenceState::Present
    }

    fn reset_absence(&mut self) {
        self.absent_since = None;
        self.away_candidate_since = None;
        self.present_since = None;
        self.left_confirmed = false;
        self.welcomed_back = false;
        self.callbacks.retain(|c| c == "turned" || c == "announced_turn");
        // Clear leave ladder callbacks only.
        for key in ["first", "second", "pause", "left"] {
            self.callbacks.remove(key);
        }
        self.lines_spoken = 0;
        self.ladder_quiet = false;
    }

    fn reset_turned(&mut self) {
        self.turned_since = None;
        self.turned_attention = None;
        self.callbacks.remove("turned");
        self.callbacks.remove("announced_turn");
    }

    /// Observe one committed live sample. Quiet on `paused` / `break`.
    pub fn observe(&mut self, sample: &LiveCameraSample, phase: &str) -> ObserveOutcome {
        let silent = is_silent_phase(phase);
        let ts = sample.ts;
        let raw = self.classify(sample);
        self.state = raw;

        let mut nudge: Option<PresenceNudge> = None;

        if raw == LivePresenceState::CameraObstructed {
            self.reset_turned();
            self.present_since = None;
            if self.obstructed_since.is_none() {
                self.obstructed_since = Some(ts);
            }
            let held = ts - self.obstructed_since.unwrap_or(ts);
            if !silent
                && !self.obstructed_said
                && held + 1e-9 >= self.config.obstructed_speak_s
            {
                self.obstructed_said = true;
                nudge = Some(PresenceNudge {
                    kind: "camera_obstructed".into(),
                    text: CAMERA_OBSTRUCTED_TEXT.into(),
                });
            }
            return ObserveOutcome {
                presence: self.state,
                nudge,
                watching_note: watching_note(self.state),
            };
        }
        self.obstructed_since = None;
        if raw == LivePresenceState::Present || raw == LivePresenceState::HeadTurned {
            self.obstructed_said = false;
        }

        if raw == LivePresenceState::HeadTurned {
            self.away_candidate_since = None;
            if self.turned_since.is_none() {
                self.turned_since = Some(ts);
                self.turned_attention = Some(sample.attention.clone());
            }
            // Keep the stricter ignore if attention flips mid-turn.
            if self.turned_attention.as_deref() != Some(sample.attention.as_str()) {
                // Prefer looking_away threshold when either side is looking_away.
                if sample.attention == "looking_away"
                    || self.turned_attention.as_deref() == Some("looking_away")
                {
                    self.turned_attention = Some("looking_away".into());
                } else {
                    self.turned_attention = Some(sample.attention.clone());
                }
            }
            let held = ts - self.turned_since.unwrap_or(ts);
            let need = if self.turned_attention.as_deref() == Some("looking_away") {
                self.config.look_away_ignore_s
            } else {
                self.config.look_down_ignore_s
            };
            if !silent
                && !self.callbacks.contains("announced_turn")
                && held + 1e-9 >= need
            {
                self.callbacks.insert("turned".into());
                self.callbacks.insert("announced_turn".into());
                let text = if self.turned_attention.as_deref() == Some("looking_away") {
                    LOOK_AWAY_TEXT
                } else {
                    LOOK_DOWN_TEXT
                };
                nudge = Some(PresenceNudge {
                    kind: "look_back".into(),
                    text: text.into(),
                });
            }
            return ObserveOutcome {
                presence: self.state,
                nudge,
                watching_note: watching_note(self.state),
            };
        }

        // Not head_turned — clear turn timers (present / left path below).
        if raw != LivePresenceState::HeadTurned {
            // Welcome-back for turn is handled when present after turn.
            if raw == LivePresenceState::Present {
                if self.callbacks.contains("turned") {
                    if self.present_since.is_none() {
                        self.present_since = Some(ts);
                    }
                    let back = ts - self.present_since.unwrap_or(ts);
                    if back + 1e-9 >= self.config.return_confirm_s {
                        self.reset_turned();
                        self.present_since = None;
                    }
                } else {
                    self.turned_since = None;
                    self.turned_attention = None;
                }
            } else {
                self.turned_since = None;
                self.turned_attention = None;
            }
        }

        if raw == LivePresenceState::LeftFrame {
            if self.away_candidate_since.is_none() {
                self.away_candidate_since = Some(ts);
            }
            let candidate_held = ts - self.away_candidate_since.unwrap_or(ts);
            if candidate_held + 1e-9 < self.config.away_confirm_s {
                // Not confirmed yet — hold prior watching note as checking.
                return ObserveOutcome {
                    presence: LivePresenceState::Present,
                    nudge: None,
                    watching_note: "Camera accountability · checking".into(),
                };
            }

            if self.absent_since.is_none() {
                self.absent_since = self.away_candidate_since.or(Some(ts));
                self.left_confirmed = true;
                self.welcomed_back = false;
                self.callbacks.insert("left".into());
            }
            self.present_since = None;
            self.state = LivePresenceState::LeftFrame;
            let held = ts - self.absent_since.unwrap_or(ts);

            if !silent && !self.ladder_quiet {
                nudge = self.ladder_nudge(held);
            }
            return ObserveOutcome {
                presence: self.state,
                nudge,
                watching_note: watching_note(self.state),
            };
        }

        // present
        self.away_candidate_since = None;

        if self.left_confirmed && self.absent_since.is_some() {
            if self.present_since.is_none() {
                self.present_since = Some(ts);
            }
            let back = ts - self.present_since.unwrap_or(ts);
            if back + 1e-9 >= self.config.return_confirm_s {
                let gone = self.present_since.unwrap_or(ts) - self.absent_since.unwrap_or(ts);
                if !silent
                    && !self.welcomed_back
                    && gone + 1e-9 >= self.config.welcome_back_min_absence_s
                {
                    self.welcomed_back = true;
                    nudge = Some(PresenceNudge {
                        kind: "welcome_back".into(),
                        text: WELCOME_BACK_TEXT.into(),
                    });
                }
                self.reset_absence();
                self.reset_turned();
            }
        } else {
            self.absent_since = None;
            if !self.callbacks.contains("turned") {
                self.present_since = None;
            }
        }

        self.state = LivePresenceState::Present;
        ObserveOutcome {
            presence: self.state,
            nudge,
            watching_note: watching_note(self.state),
        }
    }

    fn ladder_nudge(&mut self, held: f64) -> Option<PresenceNudge> {
        if self.lines_spoken >= 3 {
            return None;
        }
        let steps: [(&str, f64, &str, &str); 3] = [
            (
                "first",
                self.config.first_callback_s,
                "left_desk",
                LEFT_DESK_TEXT,
            ),
            (
                "second",
                self.config.second_callback_s,
                "suggest_break",
                SUGGEST_BREAK_TEXT,
            ),
            (
                "pause",
                self.config.pause_after_s,
                "left_desk_pause",
                LEFT_DESK_PAUSE_TEXT,
            ),
        ];
        for (name, delay, kind, text) in steps {
            if self.callbacks.contains(name) || held + 1e-9 < delay {
                continue;
            }
            self.callbacks.insert(name.into());
            self.lines_spoken += 1;
            if name == "pause" {
                self.ladder_quiet = true;
            }
            return Some(PresenceNudge {
                kind: kind.into(),
                text: text.into(),
            });
        }
        None
    }
}

/// Build a sample for tests (epoch seconds).
pub fn test_sample(ts: f64, face: bool, attention: &str, brightness: f64) -> LiveCameraSample {
    LiveCameraSample {
        face_detected: face,
        attention: attention.into(),
        brightness,
        ts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_fast() -> LivePresenceConfig {
        LivePresenceConfig {
            look_away_ignore_s: 0.0,
            look_down_ignore_s: 5.0,
            obstructed_speak_s: 8.0,
            away_confirm_s: 3.0,
            return_confirm_s: 2.0,
            first_callback_s: 12.0,
            second_callback_s: 30.0,
            pause_after_s: 90.0,
            welcome_back_min_absence_s: 8.0,
            ..LivePresenceConfig::default()
        }
    }

    #[test]
    fn looking_away_emits_look_back_on_first_committed_sample() {
        // Matches VIDEOINPUT PresenceInference: speak head_turned immediately
        // (2s persistence already happened in camera_live before the sample arrives).
        let mut p = LivePresenceInference::new(cfg_fast());
        let fired = p.observe(&test_sample(0.0, true, "looking_away", 80.0), "active");
        assert_eq!(fired.presence, LivePresenceState::HeadTurned);
        let nudge = fired.nudge.expect("look_back on first looking_away sample");
        assert_eq!(nudge.kind, "look_back");
        assert!(nudge.text.contains("looking away"));

        // Speak once.
        let again = p.observe(&test_sample(20.0, true, "looking_away", 80.0), "active");
        assert!(again.nudge.is_none());
    }

    #[test]
    fn cover_emits_camera_obstructed_after_hold() {
        let mut p = LivePresenceInference::new(cfg_fast());
        let early = p.observe(&test_sample(0.0, false, "absent", 5.0), "active");
        assert_eq!(early.presence, LivePresenceState::CameraObstructed);
        assert!(early.nudge.is_none());

        let fired = p.observe(&test_sample(8.0, false, "absent", 5.0), "active");
        let nudge = fired.nudge.expect("obstructed speak");
        assert_eq!(nudge.kind, "camera_obstructed");
        assert!(nudge.text.contains("can't see you clearly"));
    }

    #[test]
    fn leave_emits_left_desk_on_ladder() {
        let mut p = LivePresenceInference::new(cfg_fast());
        // Confirm away (3s).
        let _ = p.observe(&test_sample(0.0, false, "absent", 80.0), "active");
        let confirming = p.observe(&test_sample(3.0, false, "absent", 80.0), "active");
        assert_eq!(confirming.presence, LivePresenceState::LeftFrame);
        assert!(confirming.nudge.is_none(), "first ladder at 12s held");

        let first = p.observe(&test_sample(12.0, false, "absent", 80.0), "active");
        let nudge = first.nudge.expect("left_desk");
        assert_eq!(nudge.kind, "left_desk");
        assert!(nudge.text.contains("stepped away"));
    }

    #[test]
    fn quiet_on_paused_phase() {
        let mut looking = LivePresenceInference::new(cfg_fast());
        let _ = looking.observe(&test_sample(0.0, true, "looking_away", 80.0), "paused");
        let fired = looking.observe(&test_sample(10.0, true, "looking_away", 80.0), "paused");
        assert!(fired.nudge.is_none());

        let mut covered = LivePresenceInference::new(cfg_fast());
        let _ = covered.observe(&test_sample(0.0, false, "absent", 5.0), "break");
        let obstructed = covered.observe(&test_sample(10.0, false, "absent", 5.0), "break");
        assert!(obstructed.nudge.is_none());
    }

    #[test]
    fn looking_down_uses_phone_hedge_text() {
        let mut p = LivePresenceInference::new(cfg_fast());
        let _ = p.observe(&test_sample(0.0, true, "looking_down", 80.0), "active");
        let fired = p.observe(&test_sample(5.0, true, "looking_down", 80.0), "active");
        let nudge = fired.nudge.expect("look_back");
        assert_eq!(nudge.kind, "look_back");
        assert!(nudge.text.to_lowercase().contains("phone"));
    }

    #[test]
    fn return_welcome_after_absence() {
        let mut p = LivePresenceInference::new(cfg_fast());
        let _ = p.observe(&test_sample(0.0, false, "absent", 80.0), "active");
        let _ = p.observe(&test_sample(3.0, false, "absent", 80.0), "active");
        let _ = p.observe(&test_sample(12.0, false, "absent", 80.0), "active");
        // Back for return_confirm.
        let _ = p.observe(&test_sample(20.0, true, "present", 80.0), "active");
        let welcome = p.observe(&test_sample(22.0, true, "present", 80.0), "active");
        let nudge = welcome.nudge.expect("welcome_back");
        assert_eq!(nudge.kind, "welcome_back");
    }

    #[test]
    fn face_present_beats_dark_brightness() {
        let mut p = LivePresenceInference::new(cfg_fast());
        let out = p.observe(&test_sample(0.0, true, "present", 5.0), "active");
        assert_eq!(out.presence, LivePresenceState::Present);
    }
}
