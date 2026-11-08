//! Turn-taking state machine, arming hysteresis, adaptive end-of-turn, post-roll,
//! privacy-safe session metrics JSONL logger, and opt-in tuning WAV + sidecar recorder.
//!
//! All start/stop and arm/disarm decisions are pure functions over value structs so the
//! live audio worker (`audio.rs`), unit tests, and the offline replay tool (`bin/replay.rs`)
//! execute the exact same code path.

use crate::audio::VisionTelemetryPayload;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const VAD_FRAME_MS: u64 = 32;
pub const DEFAULT_ARM_YAW_ENTER_DEG: f32 = 20.0;
pub const DEFAULT_ARM_YAW_EXIT_DEG: f32 = 28.0;
pub const DEFAULT_ARM_PITCH_ENTER_DEG: f32 = 18.0;
pub const DEFAULT_ARM_PITCH_EXIT_DEG: f32 = 25.0;
pub const DEFAULT_ARM_DWELL_MS: u64 = 180;
pub const DEFAULT_DISARM_DWELL_MS: u64 = 300;
pub const DEFAULT_MIN_SPEECH_MS: u64 = 250;
pub const DEFAULT_MIN_SPEECH_HOTKEY_MS: u64 = 96;
pub const DEFAULT_MAX_SILENCE_TIMEOUT_MS: u64 = 1350;
pub const DEFAULT_POST_ROLL_MS: u64 = 200;
pub const DEFAULT_CALIBRATED_RESTING_MAR: f32 = 0.016;
pub const VISION_TICK_ACTIVE_MS: u64 = 66; // ~15.1 FPS when armed, listening, or facing screen
pub const VISION_TICK_IDLE_AWAY_MS: u64 = 125; // 8.0 FPS when face present but turned away > 3s
pub const VISION_TICK_IDLE_NO_FACE_MS: u64 = 200; // 5.0 FPS when no face in frame > 3s (66.7% fewer inferences)

fn default_resting_mar() -> f32 {
    DEFAULT_CALIBRATED_RESTING_MAR
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnTakingConfig {
    pub hands_free_enabled: bool,
    pub vad_threshold: f32,
    pub silence_timeout_ms: u64,
    pub max_silence_timeout_ms: u64,
    pub auto_stop_on_silence: bool,
    pub arm_yaw_enter_deg: f32,
    pub arm_yaw_exit_deg: f32,
    pub arm_pitch_enter_deg: f32,
    pub arm_pitch_exit_deg: f32,
    pub arm_dwell_ms: u64,
    pub disarm_dwell_ms: u64,
    pub min_speech_ms: u64,
    pub min_speech_hotkey_ms: u64,
    pub post_roll_ms: u64,
    #[serde(default)]
    pub calibrated_yaw_offset_deg: f32,
    #[serde(default)]
    pub calibrated_pitch_offset_deg: f32,
    #[serde(default = "default_resting_mar")]
    pub calibrated_resting_mar: f32,
    #[serde(default = "default_true")]
    pub adaptive_fps_enabled: bool,
    /// The webcam sits off to the side of the screen the user talks to, so the head is
    /// often turned 30 to 65 degrees from the camera. Widens the yaw cone around the
    /// calibrated center.
    #[serde(default)]
    pub side_camera_mode: bool,
}

impl Default for TurnTakingConfig {
    fn default() -> Self {
        Self {
            hands_free_enabled: true,
            vad_threshold: 0.45,
            silence_timeout_ms: 650,
            max_silence_timeout_ms: DEFAULT_MAX_SILENCE_TIMEOUT_MS,
            auto_stop_on_silence: true,
            arm_yaw_enter_deg: DEFAULT_ARM_YAW_ENTER_DEG,
            arm_yaw_exit_deg: DEFAULT_ARM_YAW_EXIT_DEG,
            arm_pitch_enter_deg: DEFAULT_ARM_PITCH_ENTER_DEG,
            arm_pitch_exit_deg: DEFAULT_ARM_PITCH_EXIT_DEG,
            arm_dwell_ms: DEFAULT_ARM_DWELL_MS,
            disarm_dwell_ms: DEFAULT_DISARM_DWELL_MS,
            min_speech_ms: DEFAULT_MIN_SPEECH_MS,
            min_speech_hotkey_ms: DEFAULT_MIN_SPEECH_HOTKEY_MS,
            post_roll_ms: DEFAULT_POST_ROLL_MS,
            calibrated_yaw_offset_deg: 0.0,
            calibrated_pitch_offset_deg: 0.0,
            calibrated_resting_mar: DEFAULT_CALIBRATED_RESTING_MAR,
            adaptive_fps_enabled: true,
            side_camera_mode: false,
        }
    }
}

/// Minimum yaw cone (enter, exit) in degrees when the camera is off to the side.
/// Head pose from a side angle is noisier, and people glance across a wide monitor.
pub const SIDE_CAMERA_YAW_ENTER_DEG: f32 = 35.0;
pub const SIDE_CAMERA_YAW_EXIT_DEG: f32 = 48.0;
/// Head pitch read from a side angle mixes with roll, so pitch gets a little more room too.
pub const SIDE_CAMERA_PITCH_ENTER_DEG: f32 = 24.0;
pub const SIDE_CAMERA_PITCH_EXIT_DEG: f32 = 32.0;
/// Calibration may move the "looking at the screen" center this far from the camera axis.
/// MediaPipe keeps tracking a face up to roughly 70 degrees of real head turn, and its
/// yaw estimate tends to read low near that limit, so allow some headroom.
pub const MAX_CALIBRATED_YAW_OFFSET_DEG: f32 = 75.0;

/// Effective (enter, exit) pitch cone half-widths in degrees.
pub fn effective_pitch_cone(cfg: &TurnTakingConfig) -> (f32, f32) {
    if cfg.side_camera_mode {
        (
            cfg.arm_pitch_enter_deg.max(SIDE_CAMERA_PITCH_ENTER_DEG),
            cfg.arm_pitch_exit_deg.max(SIDE_CAMERA_PITCH_EXIT_DEG),
        )
    } else {
        (cfg.arm_pitch_enter_deg, cfg.arm_pitch_exit_deg)
    }
}

/// Effective (enter, exit) yaw cone half-widths in degrees.
pub fn effective_yaw_cone(cfg: &TurnTakingConfig) -> (f32, f32) {
    if cfg.side_camera_mode {
        (
            cfg.arm_yaw_enter_deg.max(SIDE_CAMERA_YAW_ENTER_DEG),
            cfg.arm_yaw_exit_deg.max(SIDE_CAMERA_YAW_EXIT_DEG),
        )
    } else {
        (cfg.arm_yaw_enter_deg, cfg.arm_yaw_exit_deg)
    }
}

/// Tracks enter/exit dwell timestamps for `IDLE <-> ARMED` hysteresis and lip-to-listening timing.
#[derive(Debug, Clone, Default)]
pub struct ArmingHysteresisState {
    pub enter_candidate_since_ms: Option<u64>,
    pub exit_candidate_since_ms: Option<u64>,
    pub first_lip_motion_ms: Option<u64>,
    pub last_lip_motion_ms: Option<u64>,
}

impl ArmingHysteresisState {
    pub fn reset(&mut self) {
        self.enter_candidate_since_ms = None;
        self.exit_candidate_since_ms = None;
        self.first_lip_motion_ms = None;
        self.last_lip_motion_ms = None;
    }

    pub fn note_lip_motion(&mut self, lips_moving: bool, now_ms: u64) {
        if lips_moving {
            if self.first_lip_motion_ms.is_none() {
                self.first_lip_motion_ms = Some(now_ms);
            }
            self.last_lip_motion_ms = Some(now_ms);
        } else if let Some(last) = self.last_lip_motion_ms {
            if now_ms.saturating_sub(last) > 450 {
                self.first_lip_motion_ms = None;
                self.last_lip_motion_ms = None;
            }
        }
    }

    pub fn take_lip_to_listening_ms(&mut self, now_ms: u64) -> Option<u64> {
        let first = self.first_lip_motion_ms.take();
        self.last_lip_motion_ms = None;
        first.map(|t| now_ms.saturating_sub(t))
    }
}

/// Pure check whether a head pose (adjusted by the user's calibrated monitor center offset)
/// is inside the tighter arm-enter cone.
pub fn is_within_arm_enter_cone(
    vision: &VisionTelemetryPayload,
    cfg: &TurnTakingConfig,
) -> bool {
    if !vision.face_present {
        return false;
    }
    // If legacy/synthetic telemetry only set `facing_screen` with zero angles, honor it.
    if vision.yaw == 0.0
        && vision.pitch == 0.0
        && cfg.calibrated_yaw_offset_deg == 0.0
        && cfg.calibrated_pitch_offset_deg == 0.0
        && !vision.facing_screen
    {
        return false;
    }
    let rel_yaw = vision.yaw - cfg.calibrated_yaw_offset_deg;
    let rel_pitch = vision.pitch - cfg.calibrated_pitch_offset_deg;
    let (yaw_enter, _) = effective_yaw_cone(cfg);
    let (pitch_enter, _) = effective_pitch_cone(cfg);
    rel_yaw.abs() <= yaw_enter && rel_pitch.abs() <= pitch_enter
}

/// Pure check whether a head pose (adjusted by the user's calibrated monitor center offset)
/// is still inside the wider arm-exit cone.
pub fn is_within_arm_exit_cone(
    vision: &VisionTelemetryPayload,
    cfg: &TurnTakingConfig,
) -> bool {
    if !vision.face_present {
        return false;
    }
    if vision.yaw == 0.0
        && vision.pitch == 0.0
        && cfg.calibrated_yaw_offset_deg == 0.0
        && cfg.calibrated_pitch_offset_deg == 0.0
        && !vision.facing_screen
    {
        return false;
    }
    let rel_yaw = vision.yaw - cfg.calibrated_yaw_offset_deg;
    let rel_pitch = vision.pitch - cfg.calibrated_pitch_offset_deg;
    let (_, yaw_exit) = effective_yaw_cone(cfg);
    let (_, pitch_exit) = effective_pitch_cone(cfg);
    rel_yaw.abs() <= yaw_exit && rel_pitch.abs() <= pitch_exit
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LipActivityKind {
    Speaking,
    Yawning,
    Smiling,
    Chewing,
    OpenQuiet,
    Closed,
}

impl LipActivityKind {
    pub fn label(&self) -> &'static str {
        match self {
            LipActivityKind::Speaking => "LIPS: MOVING",
            LipActivityKind::Yawning => "LIPS: YAWN CEILING",
            LipActivityKind::Smiling => "LIPS: SMILE FILTER",
            LipActivityKind::Chewing => "LIPS: CHEW FILTER",
            LipActivityKind::OpenQuiet => "LIPS: OPEN (THINKING)",
            LipActivityKind::Closed => "LIPS: STILL",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LipActivityInputs {
    pub mar: f32,
    pub mar_variance: f32,
    pub jaw_open: f32,
    pub smile_score: f32,
    pub mouth_close_score: f32,
    pub speech_articulation_score: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct LipActivityClassification {
    pub kind: LipActivityKind,
    pub active_speech_motion: bool,
    pub mouth_closed: bool,
}

/// Pure classifier separating real speech lip articulation from yawning, smiling,
/// closed-lip chewing, and static open-mouth thinking pauses.
pub fn classify_lip_activity(
    inputs: &LipActivityInputs,
    cfg: &TurnTakingConfig,
) -> LipActivityClassification {
    let resting_mar = cfg.calibrated_resting_mar.clamp(0.006, 0.045);

    // 1. Yawn ceiling: wide anatomical stretch of mouth/jaw.
    if inputs.mar > 0.45 || inputs.jaw_open > 0.35 {
        return LipActivityClassification {
            kind: LipActivityKind::Yawning,
            active_speech_motion: false,
            mouth_closed: false,
        };
    }

    // 2. Smile filter: corners pulled wide (high mouthSmileLeft/Right) without vertical
    //    vowel/consonant articulation (funnel/pucker/lower-lip depression).
    let is_smiling = inputs.smile_score >= 0.45
        && inputs.speech_articulation_score < 0.14
        && inputs.mar < resting_mar + 0.068;
    if is_smiling {
        return LipActivityClassification {
            kind: LipActivityKind::Smiling,
            active_speech_motion: false,
            mouth_closed: inputs.mar < resting_mar + 0.020,
        };
    }

    // 3. Closed-lip / rhythmic chewing filter: jaw moves while lips stay pressed/sealed
    //    (`mouthClose` blendshape high relative to inner lip aperture `mar`).
    let is_chewing = inputs.jaw_open >= 0.055
        && (inputs.mouth_close_score >= 0.20 || inputs.mar < resting_mar + 0.011)
        && inputs.speech_articulation_score < 0.15;
    if is_chewing {
        return LipActivityClassification {
            kind: LipActivityKind::Chewing,
            active_speech_motion: false,
            mouth_closed: true,
        };
    }

    let min_speech_mar = (resting_mar + 0.002).max(0.016);
    let closed_mar_ceiling = (resting_mar + 0.016).max(0.030);

    // 4. Active speech articulation: inner lip aperture above resting baseline + temporal variance.
    if inputs.mar >= min_speech_mar && inputs.mar_variance >= 0.000012 {
        return LipActivityClassification {
            kind: LipActivityKind::Speaking,
            active_speech_motion: true,
            mouth_closed: false,
        };
    }

    // 5. Mouth held open statically (for example mid-sentence thinking pause).
    if inputs.mar >= closed_mar_ceiling || inputs.mar_variance >= 0.00004 {
        return LipActivityClassification {
            kind: LipActivityKind::OpenQuiet,
            active_speech_motion: false,
            mouth_closed: false,
        };
    }

    LipActivityClassification {
        kind: LipActivityKind::Closed,
        active_speech_motion: false,
        mouth_closed: true,
    }
}

/// Computes median `(yaw_offset_deg, pitch_offset_deg, resting_mar)` from a window of
/// calibration frames captured while the user looks naturally at their target screen
/// with a relaxed closed mouth.
pub fn compute_calibration_baseline(samples: &[(f32, f32, f32)]) -> Option<(f32, f32, f32)> {
    if samples.len() < 5 {
        return None;
    }
    let mut yaws: Vec<f32> = samples.iter().map(|s| s.0).collect();
    let mut pitches: Vec<f32> = samples.iter().map(|s| s.1).collect();
    let mut mars: Vec<f32> = samples.iter().map(|s| s.2).collect();
    yaws.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    pitches.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    mars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = samples.len() / 2;
    Some((
        yaws[mid].clamp(-45.0, 45.0),
        pitches[mid].clamp(-35.0, 35.0),
        mars[mid].clamp(0.006, 0.045),
    ))
}

/// Selects the Worker vision timer interval (ms) to save CPU/GPU when idle while keeping
/// full `15 FPS` (`66 ms`) responsiveness whenever armed, listening, or facing the screen.
pub fn choose_adaptive_vision_tick_ms(
    adaptive_fps_enabled: bool,
    pipeline_active: bool,
    face_present: bool,
    facing_screen: bool,
    ms_since_last_on_screen: u64,
) -> u64 {
    if !adaptive_fps_enabled || pipeline_active || facing_screen || ms_since_last_on_screen < 3000 {
        return VISION_TICK_ACTIVE_MS;
    }
    if face_present {
        VISION_TICK_IDLE_AWAY_MS
    } else {
        VISION_TICK_IDLE_NO_FACE_MS
    }
}

/// Evaluate whether the pipeline should be `ARMED` (`true`) or `IDLE` (`false`) at `now_ms`.
///
/// - Entering `ARMED` from `IDLE` requires staying inside `arm_yaw_enter_deg` / `arm_pitch_enter_deg`
///   for at least `arm_dwell_ms`, preventing a quick glance across the monitor from arming.
/// - Leaving `ARMED` back to `IDLE` requires staying outside `arm_yaw_exit_deg` / `arm_pitch_exit_deg`
///   (or losing the face) continuously for `disarm_dwell_ms`, preventing boundary flicker.
pub fn evaluate_arming_hysteresis(
    hysteresis: &mut ArmingHysteresisState,
    currently_armed: bool,
    vision_fresh: bool,
    vision: &VisionTelemetryPayload,
    cfg: &TurnTakingConfig,
    now_ms: u64,
) -> bool {
    if !cfg.hands_free_enabled || !vision_fresh {
        hysteresis.reset();
        return false;
    }

    if !currently_armed {
        if is_within_arm_enter_cone(vision, cfg) {
            hysteresis.exit_candidate_since_ms = None;
            let since = *hysteresis.enter_candidate_since_ms.get_or_insert(now_ms);
            if now_ms.saturating_sub(since) >= cfg.arm_dwell_ms {
                hysteresis.note_lip_motion(vision.lips_moving, now_ms);
                return true;
            }
        } else {
            hysteresis.enter_candidate_since_ms = None;
            hysteresis.first_lip_motion_ms = None;
        }
        false
    } else {
        hysteresis.note_lip_motion(vision.lips_moving, now_ms);
        if is_within_arm_exit_cone(vision, cfg) {
            hysteresis.exit_candidate_since_ms = None;
            true
        } else {
            let since = *hysteresis.exit_candidate_since_ms.get_or_insert(now_ms);
            if now_ms.saturating_sub(since) >= cfg.disarm_dwell_ms {
                hysteresis.reset();
                false
            } else {
                true
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GateInputs {
    pub is_armed: bool,
    pub hands_free_enabled: bool,
    pub vision_fresh: bool,
    pub face_present: bool,
    pub within_exit_cone: bool,
    pub lips_moving: bool,
    pub vad_prob: f32,
    pub recent_voice_ms: Option<u64>,
    pub consecutive_vad_frames: usize,
}

/// Pure multimodal auto-start decision (`ARMED -> LISTENING`).
///
/// Requires:
/// 1. Pipeline is in `ARMED` with fresh vision inside the active screen cone and `lips_moving`.
/// 2. Either at least 2 consecutive VAD speech frames (`64 ms`) have arrived, OR a strong VAD frame
///    arrived while a recent speech frame was already observed within `320 ms`. Because the `500 ms`
///    pre-roll ring buffer is prepended on start, requiring 2 frames (`64 ms`) never clips the first
///    syllable while rejecting single-frame (`32 ms`) keyboard clicks and mic pops.
pub fn should_auto_start(inputs: &GateInputs, cfg: &TurnTakingConfig) -> bool {
    if !inputs.is_armed
        || !inputs.hands_free_enabled
        || !inputs.vision_fresh
        || !inputs.face_present
        || !inputs.within_exit_cone
        || !inputs.lips_moving
    {
        return false;
    }

    if inputs.consecutive_vad_frames >= 2 && inputs.vad_prob >= cfg.vad_threshold {
        return true;
    }

    let recent_voice = inputs
        .recent_voice_ms
        .map(|elapsed_ms| elapsed_ms <= 320 && elapsed_ms >= VAD_FRAME_MS)
        .unwrap_or(false);

    inputs.vad_prob >= cfg.vad_threshold && recent_voice
}

pub fn is_manual_or_hotkey_trigger(trigger_source: &str) -> bool {
    let lower = trigger_source.to_ascii_lowercase();
    lower.contains("hotkey") || lower.contains("manual") || lower.contains("button")
}

pub fn required_min_speech_ms(trigger_source: &str, cfg: &TurnTakingConfig) -> u64 {
    if is_manual_or_hotkey_trigger(trigger_source) {
        cfg.min_speech_hotkey_ms
    } else {
        cfg.min_speech_ms
    }
}

/// Checks whether the captured utterance has enough voiced VAD evidence to send to Whisper.
pub fn has_minimum_speech_evidence(
    speech_frames: usize,
    trigger_source: &str,
    cfg: &TurnTakingConfig,
) -> Result<(), String> {
    let speech_ms = (speech_frames as u64) * VAD_FRAME_MS;
    let required_ms = required_min_speech_ms(trigger_source, cfg);
    if speech_ms >= required_ms {
        Ok(())
    } else {
        Err(format!(
            "Ignored brief sound ({} ms speech < {} ms minimum)",
            speech_ms, required_ms
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Silence,
    EndPhrase,
    Nod,
    Hotkey,
    ManualStop,
    MaxLength,
    Canceled,
}

impl EndReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            EndReason::Silence => "silence",
            EndReason::EndPhrase => "end_phrase",
            EndReason::Nod => "nod",
            EndReason::Hotkey => "hotkey",
            EndReason::ManualStop => "manual_stop",
            EndReason::MaxLength => "max_length",
            EndReason::Canceled => "canceled",
        }
    }
}

/// How a finished utterance gets sent. Chosen once in Settings so people do not have to
/// reason about separate preview, Enter and gesture toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SendMode {
    /// Stop talking, a short countdown runs, then it sends. Talk, shake or press Esc to cancel.
    #[default]
    Pause,
    /// Nothing sends until you nod. Shake or press Esc to cancel.
    Nod,
    /// Only sends (presses Enter) when you end with "send it". Otherwise it just types the text.
    Phrase,
    /// Always types the text and never presses Enter.
    PasteOnly,
}

/// How long a preview waits for a nod in `SendMode::Nod` before giving up without sending.
pub const NOD_WAIT_TIMEOUT_MS: u64 = 15_000;
/// Speech in the first part of a preview is usually the tail of the user's own sentence.
pub const SPEECH_CANCEL_GRACE_MS: u64 = 350;
/// About 190 ms of continuous speech is needed before talking cancels a preview.
pub const SPEECH_CANCEL_MIN_FRAMES: usize = 6;

impl SendMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SendMode::Pause => "pause",
            SendMode::Nod => "nod",
            SendMode::Phrase => "phrase",
            SendMode::PasteOnly => "paste_only",
        }
    }

    /// Whether the user asked for Enter to be pressed. App rules and the destructive command
    /// guard can still downgrade this to paste only afterwards.
    pub fn wants_enter(&self, auto_press_enter: bool, end_phrase_triggered: bool) -> bool {
        match self {
            SendMode::Pause | SendMode::Nod => auto_press_enter || end_phrase_triggered,
            SendMode::Phrase => end_phrase_triggered,
            SendMode::PasteOnly => false,
        }
    }

    /// Whether this utterance's preview must wait for a nod instead of counting down.
    /// A nod that already ended the turn counts as the go-ahead, so it gets the normal
    /// short countdown (still cancelable) instead of asking for a second nod.
    pub fn preview_waits_for_nod(&self, end_reason: EndReason) -> bool {
        *self == SendMode::Nod && end_reason != EndReason::Nod
    }
}

/// What happens when a preview's timer runs out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewDeadline {
    /// Countdown finished: send it.
    Send,
    /// Nobody nodded in time: drop it and say so.
    GiveUp,
}

pub fn preview_deadline(
    waits_for_nod: bool,
    elapsed_ms: u64,
    countdown_ms: u64,
) -> Option<PreviewDeadline> {
    if elapsed_ms < countdown_ms {
        return None;
    }
    Some(if waits_for_nod {
        PreviewDeadline::GiveUp
    } else {
        PreviewDeadline::Send
    })
}

/// Talking during the preview cancels it ("no, wait" or "cancel"). When the camera is on we
/// also require moving lips, so someone else talking in the room does not cancel your send.
pub fn should_cancel_preview_on_speech(
    enabled: bool,
    preview_elapsed_ms: u64,
    consecutive_vad_frames: usize,
    vision_fresh: bool,
    lips_moving: bool,
) -> bool {
    enabled
        && preview_elapsed_ms >= SPEECH_CANCEL_GRACE_MS
        && consecutive_vad_frames >= SPEECH_CANCEL_MIN_FRAMES
        && (!vision_fresh || lips_moving)
}

#[derive(Debug, Clone)]
pub struct TurnState {
    pub started_at_ms: u64,
    pub last_speech_at_ms: u64,
    pub last_lip_motion_at_ms: Option<u64>,
    pub speech_frames: usize,
    pub utterance_samples: usize,
    pub max_utterance_samples: usize,
    pub hands_free_enabled: bool,
    pub vision_fresh: bool,
    pub lips_moving: bool,
    pub mouth_closed: bool,
    pub gesture: String,
    pub ends_with_connective: bool,
}

/// True when a partial transcript ends in a comma or a trailing connective/preposition/article
/// that indicates the speaker is pausing mid-clause rather than finishing a command.
pub fn ends_with_connective(partial_text: &str) -> bool {
    let trimmed = partial_text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with(',') || trimmed.ends_with(':') || trimmed.ends_with('-') {
        return true;
    }
    let last_word = trimmed
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase();

    const CONNECTIVES: &[&str] = &[
        "and", "or", "but", "the", "a", "an", "to", "for", "with", "in", "on", "at", "from",
        "by", "of", "as", "into", "like", "that", "which", "who", "when", "where", "while",
        "if", "because", "since", "so", "then", "than",
    ];
    CONNECTIVES.contains(&last_word.as_str())
}

/// Computes the adaptive silence timeout (ms) for the current turn.
///
/// - Short commands (`<= 900 ms` of speech) with a closed mouth use `cfg.silence_timeout_ms` (`650 ms`).
/// - Longer utterances scale up by `(speech_ms - 900) / 3` (up to `+450 ms`) because developers
///   dictating multi-clause prompts pause longer between clauses.
/// - If the speaker's mouth is held open mid-thought (`!mouth_closed` while vision is active),
///   adds `+350 ms` so mid-sentence thinking pauses are not cut off.
/// - If a partial transcript ends with a connective (`"and"`, `"the"`, `"to"`, or `,`), adds `+350 ms`.
/// - Capped at `cfg.max_silence_timeout_ms`.
pub fn compute_adaptive_silence_timeout_ms(turn: &TurnState, cfg: &TurnTakingConfig) -> u64 {
    let base = cfg.silence_timeout_ms;
    let max_cap = cfg.max_silence_timeout_ms.max(base);
    let speech_ms = (turn.speech_frames as u64) * VAD_FRAME_MS;

    let length_bonus = if speech_ms > 900 {
        ((speech_ms - 900) / 3).min(450)
    } else {
        0
    };

    let open_mouth_bonus = if turn.vision_fresh && turn.hands_free_enabled && !turn.mouth_closed {
        350
    } else {
        0
    };

    let connective_bonus = if turn.ends_with_connective { 350 } else { 0 };

    (base + length_bonus + open_mouth_bonus + connective_bonus).min(max_cap)
}

/// Pure end-of-turn decision (`LISTENING -> post-roll / TRANSCRIBING`).
///
/// Returns `Some(EndReason)` when the utterance should end, or `None` to keep listening.
pub fn end_of_turn(
    turn: &TurnState,
    cfg: &TurnTakingConfig,
    now_ms: u64,
) -> Option<EndReason> {
    if turn.gesture == "nod" {
        return Some(EndReason::Nod);
    }

    if turn.utterance_samples >= turn.max_utterance_samples {
        return Some(EndReason::MaxLength);
    }

    if !cfg.auto_stop_on_silence {
        return None;
    }

    // Never cut off the speaker while their lips are actively moving.
    if turn.vision_fresh && turn.hands_free_enabled && turn.lips_moving {
        return None;
    }

    let last_active_ms = turn
        .last_lip_motion_at_ms
        .map(|lip_t| turn.last_speech_at_ms.max(lip_t))
        .unwrap_or(turn.last_speech_at_ms);

    let timeout_ms = compute_adaptive_silence_timeout_ms(turn, cfg);
    if now_ms.saturating_sub(last_active_ms) >= timeout_ms {
        return Some(EndReason::Silence);
    }

    None
}

/// Returns `(remaining_ms, total_timeout_ms)` for the HUD end-of-turn silence countdown.
/// When the speaker is actively speaking or moving their lips, `remaining_ms == total_timeout_ms`.
pub fn silence_countdown_ms(
    turn: &TurnState,
    cfg: &TurnTakingConfig,
    now_ms: u64,
) -> (u64, u64) {
    let total_ms = compute_adaptive_silence_timeout_ms(turn, cfg);
    if !cfg.auto_stop_on_silence
        || (turn.vision_fresh && turn.hands_free_enabled && turn.lips_moving)
    {
        return (total_ms, total_ms);
    }
    let last_active_ms = turn
        .last_lip_motion_at_ms
        .map(|lip_t| turn.last_speech_at_ms.max(lip_t))
        .unwrap_or(turn.last_speech_at_ms);
    let elapsed = now_ms.saturating_sub(last_active_ms);
    (total_ms.saturating_sub(elapsed), total_ms)
}

/// Privacy-safe per-utterance metrics line written to `<app_data_dir>/session-metrics.jsonl`.
/// Contains NO transcript text and NO audio samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtteranceMetricsRecord {
    pub utterance_id: u64,
    pub timestamp_ms: u64,
    pub trigger_source: String,
    pub lip_to_listening_ms: Option<u64>,
    pub utterance_length_ms: u64,
    pub speech_frames: usize,
    pub speech_duration_ms: u64,
    pub peak_vad: f32,
    pub end_reason: String,
    pub post_roll_ms: u64,
    pub transcript_length: usize,
    pub discarded_reason: Option<String>,
    pub outcome: String,
    pub flag_false_trigger_clicks: u32,
}

/// Converts any human-facing discard message into a privacy-safe category tag that never
/// embeds spoken words.
pub fn sanitize_discarded_reason_for_metrics(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("minimum") || lower.contains("brief sound") {
        "insufficient_speech_evidence".to_string()
    } else if lower.contains("hallucination") {
        "whisper_silence_hallucination".to_string()
    } else if lower.contains("no speech") || lower.contains("empty") {
        "empty_transcript".to_string()
    } else {
        "discarded".to_string()
    }
}

/// Append one JSON line to `<app_data_dir>/session-metrics.jsonl`.
pub fn append_session_metrics_line(
    app_data_dir: &Path,
    record: &UtteranceMetricsRecord,
) -> Result<PathBuf, String> {
    fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("Failed to create app data dir for metrics: {}", e))?;
    let path = app_data_dir.join("session-metrics.jsonl");
    let mut line = serde_json::to_string(record)
        .map_err(|e| format!("Failed to serialize session metrics: {}", e))?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    file.write_all(line.as_bytes())
        .map_err(|e| format!("Failed to write session metrics: {}", e))?;
    Ok(path)
}

/// Update `flag_false_trigger_clicks` for an existing `utterance_id` in `session-metrics.jsonl`.
pub fn update_false_trigger_flag_in_metrics(
    app_data_dir: &Path,
    utterance_id: u64,
    flagged: bool,
) -> Result<(), String> {
    let path = app_data_dir.join("session-metrics.jsonl");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut out = String::with_capacity(content.len() + 64);
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(mut rec) = serde_json::from_str::<UtteranceMetricsRecord>(line) {
            if rec.utterance_id == utterance_id {
                if flagged {
                    rec.flag_false_trigger_clicks = rec.flag_false_trigger_clicks.saturating_add(1);
                } else {
                    rec.flag_false_trigger_clicks = rec.flag_false_trigger_clicks.saturating_sub(1);
                }
                if let Ok(updated) = serde_json::to_string(&rec) {
                    out.push_str(&updated);
                    out.push('\n');
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    fs::write(&path, out).map_err(|e| format!("Failed to update {}: {}", path.display(), e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedVisionSample {
    pub offset_ms: u64,
    pub face_present: bool,
    pub facing_screen: bool,
    pub lips_moving: bool,
    pub mouth_closed: bool,
    pub mar: f32,
    pub mar_variance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub jaw_open: f32,
    pub gesture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuningRecordingSidecar {
    pub sample_rate: u32,
    pub metrics: UtteranceMetricsRecord,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub expected_phrase: Option<String>,
    #[serde(default)]
    pub expected_should_transcribe: Option<bool>,
    pub vision_frames: Vec<RecordedVisionSample>,
}

pub fn resolve_recordings_dir() -> PathBuf {
    let candidates = [PathBuf::from("../recordings"), PathBuf::from("recordings")];
    for c in &candidates {
        if c.exists() && c.is_dir() {
            return c.clone();
        }
    }
    if PathBuf::from("../src-tauri").exists() {
        PathBuf::from("../recordings")
    } else {
        PathBuf::from("recordings")
    }
}

/// Write a 16 kHz mono 16-bit PCM WAV file and its matching JSON sidecar to `recordings/`.
pub fn save_tuning_utterance_wav_and_sidecar(
    recordings_dir: &Path,
    samples_16k: &[f32],
    sidecar: &TuningRecordingSidecar,
) -> Result<(PathBuf, PathBuf), String> {
    fs::create_dir_all(recordings_dir)
        .map_err(|e| format!("Failed to create recordings dir: {}", e))?;
    let stem = format!(
        "utterance-{}-{}",
        sidecar.metrics.timestamp_ms, sidecar.metrics.utterance_id
    );
    let wav_path = recordings_dir.join(format!("{}.wav", stem));
    let json_path = recordings_dir.join(format!("{}.json", stem));

    write_wav_16k_mono_i16(&wav_path, samples_16k)?;
    let json = serde_json::to_string_pretty(sidecar)
        .map_err(|e| format!("Failed to serialize tuning sidecar: {}", e))?;
    fs::write(&json_path, json)
        .map_err(|e| format!("Failed to write {}: {}", json_path.display(), e))?;
    Ok((wav_path, json_path))
}

/// Write 16 kHz mono `f32` samples (`-1.0..1.0`) to a standard 44-byte RIFF PCM 16-bit WAV file.
pub fn write_wav_16k_mono_i16(path: &Path, samples: &[f32]) -> Result<(), String> {
    let sample_rate: u32 = 16_000;
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
    let block_align: u16 = num_channels * (bits_per_sample / 8);
    let data_bytes: u32 = (samples.len() * 2) as u32;
    let riff_size: u32 = 36 + data_bytes;

    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());

    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let pcm = (clamped * 32767.0).round() as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }

    fs::write(path, buf).map_err(|e| format!("Failed to write WAV {}: {}", path.display(), e))
}

/// Read a 16-bit PCM or 32-bit float WAV file into mono `f32` samples.
pub fn read_wav_mono_f32(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Failed to open WAV {}: {}", path.display(), e))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read WAV {}: {}", path.display(), e))?;

    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("{} is not a valid RIFF WAVE file", path.display()));
    }

    let mut pos = 12usize;
    let mut format_tag = 1u16;
    let mut channels = 1u16;
    let mut sample_rate = 16_000u32;
    let mut bits_per_sample = 16u16;
    let mut data_slice: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        let end = (pos + chunk_size).min(bytes.len());
        if chunk_id == b"fmt " && end - pos >= 16 {
            format_tag = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
            channels = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
            sample_rate = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]);
            bits_per_sample = u16::from_le_bytes([bytes[pos + 14], bytes[pos + 15]]);
        } else if chunk_id == b"data" {
            data_slice = Some(&bytes[pos..end]);
            break;
        }
        // Chunks are word-aligned
        pos = end + (chunk_size & 1);
    }

    let raw = data_slice.ok_or_else(|| format!("No data chunk in {}", path.display()))?;
    let ch = (channels as usize).max(1);
    let mut out = Vec::new();

    if format_tag == 1 && bits_per_sample == 16 {
        let frame_bytes = 2 * ch;
        for frame in raw.chunks_exact(frame_bytes) {
            let mut sum = 0.0f32;
            for c in 0..ch {
                let sample = i16::from_le_bytes([frame[c * 2], frame[c * 2 + 1]]);
                sum += sample as f32 / 32768.0;
            }
            out.push(sum / ch as f32);
        }
    } else if format_tag == 3 && bits_per_sample == 32 {
        let frame_bytes = 4 * ch;
        for frame in raw.chunks_exact(frame_bytes) {
            let mut sum = 0.0f32;
            for c in 0..ch {
                let sample = f32::from_le_bytes([
                    frame[c * 4],
                    frame[c * 4 + 1],
                    frame[c * 4 + 2],
                    frame[c * 4 + 3],
                ]);
                sum += sample;
            }
            out.push(sum / ch as f32);
        }
    } else {
        return Err(format!(
            "Unsupported WAV format (tag={}, bits={}) in {}",
            format_tag,
            bits_per_sample,
            path.display()
        ));
    }

    Ok((out, sample_rate))
}

/// Timeout (ms) without any `cpal` input callback before the audio worker treats the active
/// microphone stream as stalled/unplugged and attempts automatic hot-plug recovery.
pub const AUDIO_STREAM_STALL_TIMEOUT_MS: u64 = 2_500;

/// Wall-clock jump threshold (ms) between consecutive 12 ms worker ticks that indicates the OS
/// slept, locked, or suspended the process.
pub const SYSTEM_SLEEP_JUMP_THRESHOLD_MS: u64 = 3_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioStreamRecoveryAction {
    Healthy,
    ReconnectAfterFaultOrStall { reason: String },
    SwitchBackToPreferred { preferred: String },
}

/// Pure decision function for microphone hot-plug recovery and preferred-device restoration.
pub fn evaluate_audio_stream_health(
    stream_present: bool,
    active_device_name: &str,
    preferred_device: Option<&str>,
    available_devices: &[String],
    stream_faulted: bool,
    ms_since_last_callback: Option<u64>,
) -> AudioStreamRecoveryAction {
    if !stream_present {
        return AudioStreamRecoveryAction::ReconnectAfterFaultOrStall {
            reason: "Microphone stream is not active".to_string(),
        };
    }
    if stream_faulted {
        return AudioStreamRecoveryAction::ReconnectAfterFaultOrStall {
            reason: format!(
                "Microphone '{}' reported a stream error or was unplugged",
                active_device_name
            ),
        };
    }
    if let Some(elapsed_ms) = ms_since_last_callback {
        if elapsed_ms >= AUDIO_STREAM_STALL_TIMEOUT_MS {
            return AudioStreamRecoveryAction::ReconnectAfterFaultOrStall {
                reason: format!(
                    "Microphone '{}' stopped delivering audio for {} ms",
                    active_device_name, elapsed_ms
                ),
            };
        }
    }
    if let Some(pref) = preferred_device {
        let pref_trimmed = pref.trim();
        if !pref_trimmed.is_empty()
            && active_device_name != pref_trimmed
            && available_devices.iter().any(|d| d == pref_trimmed)
        {
            return AudioStreamRecoveryAction::SwitchBackToPreferred {
                preferred: pref_trimmed.to_string(),
            };
        }
    }
    AudioStreamRecoveryAction::Healthy
}

/// Detects when wall-clock time between two loop ticks jumped by more than `threshold_ms`
/// (`SYSTEM_SLEEP_JUMP_THRESHOLD_MS`), indicating system sleep, suspend, or screen lock.
pub fn detect_system_sleep_or_stall_jump(
    prev_wall_ms: u64,
    now_wall_ms: u64,
    threshold_ms: u64,
) -> Option<u64> {
    let delta = now_wall_ms.saturating_sub(prev_wall_ms);
    if delta > threshold_ms {
        Some(delta)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_camera_widens_yaw_cone() {
        let mut cfg = TurnTakingConfig {
            calibrated_yaw_offset_deg: 50.0,
            ..TurnTakingConfig::default()
        };
        let vision = |yaw: f32| VisionTelemetryPayload {
            face_present: true,
            facing_screen: false,
            yaw,
            ..VisionTelemetryPayload::default()
        };
        // Looking straight at the calibrated screen counts in both modes.
        assert!(is_within_arm_enter_cone(&vision(50.0), &cfg));
        // A 30 degree glance across a wide monitor only counts with the side camera switch.
        assert!(!is_within_arm_enter_cone(&vision(80.0), &cfg));
        assert!(!is_within_arm_exit_cone(&vision(90.0), &cfg));
        cfg.side_camera_mode = true;
        assert!(is_within_arm_enter_cone(&vision(80.0), &cfg));
        assert!(is_within_arm_exit_cone(&vision(90.0), &cfg));
        // Turning fully away (to talk to someone) still disarms.
        assert!(!is_within_arm_exit_cone(&vision(-10.0), &cfg));
        // Pitch gets more room from the side as well.
        let tilted = VisionTelemetryPayload {
            pitch: 22.0,
            ..vision(50.0)
        };
        assert!(is_within_arm_enter_cone(&tilted, &cfg));
        // User-set cones wider than the side-camera minimum are kept.
        cfg.arm_yaw_enter_deg = 40.0;
        assert_eq!(effective_yaw_cone(&cfg), (40.0, SIDE_CAMERA_YAW_EXIT_DEG));
    }

    #[test]
    fn test_send_mode_enter_rules() {
        assert!(SendMode::Pause.wants_enter(true, false));
        assert!(SendMode::Nod.wants_enter(true, false));
        assert!(!SendMode::Phrase.wants_enter(true, false));
        assert!(SendMode::Phrase.wants_enter(true, true));
        assert!(!SendMode::PasteOnly.wants_enter(true, true));
        // Legacy auto_press_enter = false still respected for pause and nod modes.
        assert!(!SendMode::Pause.wants_enter(false, false));
        assert!(SendMode::Pause.wants_enter(false, true));
    }

    #[test]
    fn test_send_mode_nod_waiting() {
        assert!(SendMode::Nod.preview_waits_for_nod(EndReason::Silence));
        assert!(SendMode::Nod.preview_waits_for_nod(EndReason::Hotkey));
        assert!(!SendMode::Nod.preview_waits_for_nod(EndReason::Nod));
        assert!(!SendMode::Pause.preview_waits_for_nod(EndReason::Silence));
        assert_eq!(
            serde_json::from_str::<SendMode>("\"paste_only\"").unwrap(),
            SendMode::PasteOnly
        );
        assert_eq!(SendMode::default(), SendMode::Pause);
    }

    #[test]
    fn test_preview_deadline() {
        assert_eq!(preview_deadline(false, 100, 1800), None);
        assert_eq!(
            preview_deadline(false, 1800, 1800),
            Some(PreviewDeadline::Send)
        );
        assert_eq!(preview_deadline(true, 1800, NOD_WAIT_TIMEOUT_MS), None);
        assert_eq!(
            preview_deadline(true, NOD_WAIT_TIMEOUT_MS, NOD_WAIT_TIMEOUT_MS),
            Some(PreviewDeadline::GiveUp)
        );
    }

    #[test]
    fn test_speech_cancels_preview() {
        // Grace period: the tail of the user's own sentence must not cancel.
        assert!(!should_cancel_preview_on_speech(true, 100, 10, true, true));
        // Too short a burst (a cough or click).
        assert!(!should_cancel_preview_on_speech(true, 800, 3, true, true));
        // Someone else talking while the user's lips are still.
        assert!(!should_cancel_preview_on_speech(true, 800, 10, true, false));
        // The user says "cancel".
        assert!(should_cancel_preview_on_speech(true, 800, 6, true, true));
        // Camera off (hotkey use): voice alone is enough.
        assert!(should_cancel_preview_on_speech(true, 800, 6, false, false));
        // Turned off in settings.
        assert!(!should_cancel_preview_on_speech(false, 800, 10, true, true));
    }

    #[test]
    fn test_arming_hysteresis_prevents_flicker_and_glance_arming() {
        let cfg = TurnTakingConfig::default();
        let mut h = ArmingHysteresisState::default();
        let mut armed = false;

        // Glancing across the screen (inside 20 deg yaw for only 100 ms < 180 ms dwell) must NOT arm.
        let on_screen = VisionTelemetryPayload {
            face_present: true,
            facing_screen: true,
            yaw: 12.0,
            pitch: 4.0,
            ..Default::default()
        };
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &on_screen, &cfg, 1000);
        assert!(!armed);
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &on_screen, &cfg, 1100);
        assert!(!armed, "100ms glance must not arm");

        // Looked away at 1150ms before dwell completed -> resets candidate
        let way_off = VisionTelemetryPayload {
            face_present: true,
            facing_screen: false,
            yaw: 35.0,
            pitch: 5.0,
            ..Default::default()
        };
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &way_off, &cfg, 1150);
        assert!(!armed);

        // Steady gaze for >= 180ms arms the gate
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &on_screen, &cfg, 2000);
        assert!(!armed);
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &on_screen, &cfg, 2190);
        assert!(armed, "Steady gaze >= 180ms should arm");

        // Head drifts to yaw = 24 deg (outside 20 deg enter, but inside 28 deg exit): stays ARMED indefinitely
        let edge_pose = VisionTelemetryPayload {
            face_present: true,
            facing_screen: false,
            yaw: 24.5,
            pitch: 19.0,
            ..Default::default()
        };
        for t in (2250..4000).step_by(66) {
            armed = evaluate_arming_hysteresis(&mut h, armed, true, &edge_pose, &cfg, t);
            assert!(armed, "Must stay ARMED inside exit hysteresis band at t={}", t);
        }

        // Brief spike to yaw = 31 deg for 200 ms (< 300 ms disarm dwell) stays ARMED
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &way_off, &cfg, 4100);
        assert!(armed);
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &way_off, &cfg, 4300);
        assert!(armed, "200ms excursion outside exit cone must not disarm yet");

        // Sustained look-away >= 300 ms disarms to IDLE
        armed = evaluate_arming_hysteresis(&mut h, armed, true, &way_off, &cfg, 4410);
        assert!(!armed, "Sustained look-away >= 300ms must disarm");
    }

    #[test]
    fn test_should_auto_start_requires_sustained_speech_and_lip_motion() {
        let cfg = TurnTakingConfig::default();
        // Single 32ms click frame with no recent voice must not trigger auto-start
        let single_click = GateInputs {
            is_armed: true,
            hands_free_enabled: true,
            vision_fresh: true,
            face_present: true,
            within_exit_cone: true,
            lips_moving: true,
            vad_prob: 0.72,
            recent_voice_ms: None,
            consecutive_vad_frames: 1,
        };
        assert!(!should_auto_start(&single_click, &cfg));

        // Two consecutive speech frames with lip motion triggers auto-start
        let sustained = GateInputs {
            consecutive_vad_frames: 2,
            ..single_click
        };
        assert!(should_auto_start(&sustained, &cfg));

        // Talking off-screen (outside exit cone) never triggers
        let off_screen = GateInputs {
            within_exit_cone: false,
            ..sustained
        };
        assert!(!should_auto_start(&off_screen, &cfg));
    }

    #[test]
    fn test_minimum_speech_evidence_hands_free_vs_hotkey() {
        let cfg = TurnTakingConfig::default();
        // 3 frames (96 ms) cough in hands-free mode is rejected (< 250 ms)
        assert!(has_minimum_speech_evidence(3, "hands_free_multimodal", &cfg).is_err());
        // 8 frames (256 ms) in hands-free mode passes (>= 250 ms)
        assert!(has_minimum_speech_evidence(8, "hands_free_multimodal", &cfg).is_ok());
        // 3 frames (96 ms) via hotkey passes (>= 96 ms)
        assert!(has_minimum_speech_evidence(3, "hotkey (Option+Space)", &cfg).is_ok());
    }

    #[test]
    fn test_adaptive_end_of_turn_waits_for_lips_and_long_clauses() {
        let cfg = TurnTakingConfig::default();
        let mut turn = TurnState {
            started_at_ms: 0,
            last_speech_at_ms: 1000,
            last_lip_motion_at_ms: Some(1000),
            speech_frames: 10, // 320 ms short utterance
            utterance_samples: 16_000 * 2,
            max_utterance_samples: 16_000 * 60,
            hands_free_enabled: true,
            vision_fresh: true,
            lips_moving: true,
            mouth_closed: false,
            gesture: "none".to_string(),
            ends_with_connective: false,
        };

        // Even after 1200 ms of VAD silence, while lips_moving is true we never end the turn
        assert_eq!(end_of_turn(&turn, &cfg, 2200), None);

        // Short command with closed mouth ends right at 650 ms
        turn.lips_moving = false;
        turn.mouth_closed = true;
        assert_eq!(end_of_turn(&turn, &cfg, 1600), None);
        assert_eq!(end_of_turn(&turn, &cfg, 1650), Some(EndReason::Silence));

        // Long multi-clause utterance (60 frames = 1920 ms) with open mouth thinking pause
        // gets extended timeout (650 + 340 + 350 = 1340 ms) so 900 ms pause does not cut off!
        turn.speech_frames = 60;
        turn.mouth_closed = false;
        let adaptive_ms = compute_adaptive_silence_timeout_ms(&turn, &cfg);
        assert!(adaptive_ms >= 1300, "adaptive timeout was {}", adaptive_ms);
        assert_eq!(end_of_turn(&turn, &cfg, 1950), None);
        assert_eq!(end_of_turn(&turn, &cfg, 2360), Some(EndReason::Silence));
    }

    #[test]
    fn test_session_metrics_log_never_contains_transcript_text() {
        let tmp = std::env::temp_dir().join(format!("saywhen-metrics-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let secret_transcript = "git commit -m super_secret_token_12345";
        let rec = UtteranceMetricsRecord {
            utterance_id: 1,
            timestamp_ms: 1700000000000,
            trigger_source: "hands_free_multimodal".to_string(),
            lip_to_listening_ms: Some(140),
            utterance_length_ms: 2100,
            speech_frames: 42,
            speech_duration_ms: 1344,
            peak_vad: 0.93,
            end_reason: EndReason::Silence.as_str().to_string(),
            post_roll_ms: 200,
            transcript_length: secret_transcript.len(),
            discarded_reason: None,
            outcome: "sent".to_string(),
            flag_false_trigger_clicks: 0,
        };
        let path = append_session_metrics_line(&tmp, &rec).unwrap();
        update_false_trigger_flag_in_metrics(&tmp, 1, true).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("super_secret_token"));
        assert!(raw.contains("\"flagFalseTriggerClicks\":1"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_wav_write_and_read_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("saywhen-wav-test-{}.wav", std::process::id()));
        let orig: Vec<f32> = (0..1600)
            .map(|i| ((i as f32) * 0.05).sin() * 0.5)
            .collect();
        write_wav_16k_mono_i16(&tmp, &orig).unwrap();
        let (decoded, rate) = read_wav_mono_f32(&tmp).unwrap();
        assert_eq!(rate, 16_000);
        assert_eq!(decoded.len(), orig.len());
        for (a, b) in orig.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.001);
        }
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_calibrated_off_center_monitor_cone_and_median_baseline() {
        // Developer looking at an external monitor 24 deg to the left and 12 deg up,
        // with one noisy outlier frame.
        let samples = vec![
            (-24.2, 11.8, 0.017),
            (-23.8, 12.1, 0.018),
            (-24.0, 12.0, 0.017),
            (-24.1, 12.2, 0.018),
            (-5.0, 0.0, 0.040), // single-frame outlier
            (-23.9, 11.9, 0.017),
            (-24.0, 12.0, 0.017),
        ];
        let (yaw0, pitch0, mar0) = compute_calibration_baseline(&samples).unwrap();
        assert!((yaw0 - (-24.0)).abs() < 0.25);
        assert!((pitch0 - 12.0).abs() < 0.25);
        assert!((mar0 - 0.017).abs() < 0.002);

        let uncalibrated_cfg = TurnTakingConfig::default();
        let mut calibrated_cfg = TurnTakingConfig::default();
        calibrated_cfg.calibrated_yaw_offset_deg = yaw0;
        calibrated_cfg.calibrated_pitch_offset_deg = pitch0;
        calibrated_cfg.calibrated_resting_mar = mar0;

        let side_monitor_pose = VisionTelemetryPayload {
            face_present: true,
            facing_screen: false,
            yaw: -25.0,
            pitch: 13.0,
            ..Default::default()
        };

        // Without calibration, yaw = -25 deg is outside the 20 deg enter cone.
        assert!(!is_within_arm_enter_cone(&side_monitor_pose, &uncalibrated_cfg));
        // With calibration (yaw0 = -24 deg), relative yaw is -1.0 deg -> inside enter cone!
        assert!(is_within_arm_enter_cone(&side_monitor_pose, &calibrated_cfg));
        assert!(is_within_arm_exit_cone(&side_monitor_pose, &calibrated_cfg));
    }

    #[test]
    fn test_classify_lip_activity_separates_speech_from_yawn_smile_and_chewing() {
        let cfg = TurnTakingConfig::default();

        // 1. Real speech articulation -> Speaking (`active_speech_motion = true`)
        let speech = LipActivityInputs {
            mar: 0.125,
            mar_variance: 0.00038,
            jaw_open: 0.14,
            smile_score: 0.08,
            mouth_close_score: 0.02,
            speech_articulation_score: 0.28,
        };
        let res_speech = classify_lip_activity(&speech, &cfg);
        assert_eq!(res_speech.kind, LipActivityKind::Speaking);
        assert!(res_speech.active_speech_motion);
        assert!(!res_speech.mouth_closed);

        // 2. Wide yawn -> Yawning (`active_speech_motion = false`)
        let yawn = LipActivityInputs {
            mar: 0.49,
            mar_variance: 0.0005,
            jaw_open: 0.42,
            smile_score: 0.02,
            mouth_close_score: 0.0,
            speech_articulation_score: 0.05,
        };
        let res_yawn = classify_lip_activity(&yawn, &cfg);
        assert_eq!(res_yawn.kind, LipActivityKind::Yawning);
        assert!(!res_yawn.active_speech_motion);

        // 3. Smiling while reading code (MAR slightly elevated with variance, high smile blendshape)
        let smile = LipActivityInputs {
            mar: 0.052,
            mar_variance: 0.000045,
            jaw_open: 0.04,
            smile_score: 0.64,
            mouth_close_score: 0.05,
            speech_articulation_score: 0.04,
        };
        let res_smile = classify_lip_activity(&smile, &cfg);
        assert_eq!(res_smile.kind, LipActivityKind::Smiling);
        assert!(!res_smile.active_speech_motion);

        // 4. Closed-lip chewing (jaw moving rhythmically while lips stay pressed/sealed)
        let chewing = LipActivityInputs {
            mar: 0.022,
            mar_variance: 0.000035,
            jaw_open: 0.11,
            smile_score: 0.05,
            mouth_close_score: 0.29,
            speech_articulation_score: 0.05,
        };
        let res_chew = classify_lip_activity(&chewing, &cfg);
        assert_eq!(res_chew.kind, LipActivityKind::Chewing);
        assert!(!res_chew.active_speech_motion);
        assert!(res_chew.mouth_closed);

        // 5. Open-mouth mid-thought pause (`mar = 0.055`, low variance)
        let open_pause = LipActivityInputs {
            mar: 0.055,
            mar_variance: 0.000004,
            jaw_open: 0.06,
            smile_score: 0.04,
            mouth_close_score: 0.01,
            speech_articulation_score: 0.02,
        };
        let res_pause = classify_lip_activity(&open_pause, &cfg);
        assert_eq!(res_pause.kind, LipActivityKind::OpenQuiet);
        assert!(!res_pause.active_speech_motion);
        assert!(!res_pause.mouth_closed);
    }

    #[test]
    fn test_adaptive_vision_fps_scheduling_reduces_idle_duty_cycle() {
        // Active pipeline or facing screen -> 66 ms (~15.1 FPS)
        assert_eq!(
            choose_adaptive_vision_tick_ms(true, true, true, true, 0),
            VISION_TICK_ACTIVE_MS
        );
        // Within 3s grace window after looking away -> stays at 66 ms (~15.1 FPS)
        assert_eq!(
            choose_adaptive_vision_tick_ms(true, false, true, false, 1800),
            VISION_TICK_ACTIVE_MS
        );
        // Face present but turned away > 3s in IDLE -> 125 ms (8.0 FPS, 47.2% fewer inferences)
        assert_eq!(
            choose_adaptive_vision_tick_ms(true, false, true, false, 4500),
            VISION_TICK_IDLE_AWAY_MS
        );
        // No face in frame > 3s in IDLE -> 200 ms (5.0 FPS, 66.7% fewer inferences)
        assert_eq!(
            choose_adaptive_vision_tick_ms(true, false, false, false, 5000),
            VISION_TICK_IDLE_NO_FACE_MS
        );
        // When adaptive FPS is disabled -> always 66 ms
        assert_eq!(
            choose_adaptive_vision_tick_ms(false, false, false, false, 10_000),
            VISION_TICK_ACTIVE_MS
        );
    }

    #[test]
    fn test_audio_hotplug_recovery_and_preferred_device_restore() {
        let devices = vec![
            "MacBook Pro Microphone".to_string(),
            "AirPods Pro".to_string(),
        ];

        // 1. Healthy stream on preferred device
        assert_eq!(
            evaluate_audio_stream_health(
                true,
                "AirPods Pro",
                Some("AirPods Pro"),
                &devices,
                false,
                Some(24),
            ),
            AudioStreamRecoveryAction::Healthy
        );

        // 2. Unplugged / faulted stream triggers ReconnectAfterFaultOrStall
        assert!(matches!(
            evaluate_audio_stream_health(
                true,
                "AirPods Pro",
                Some("AirPods Pro"),
                &devices[..1],
                true,
                Some(120),
            ),
            AudioStreamRecoveryAction::ReconnectAfterFaultOrStall { .. }
        ));

        // 3. Stalled callback (> 2,500 ms) triggers ReconnectAfterFaultOrStall
        assert!(matches!(
            evaluate_audio_stream_health(
                true,
                "AirPods Pro",
                Some("AirPods Pro"),
                &devices,
                false,
                Some(2_800),
            ),
            AudioStreamRecoveryAction::ReconnectAfterFaultOrStall { .. }
        ));

        // 4. Currently on fallback ("MacBook Pro Microphone"), and "AirPods Pro" reappears -> SwitchBackToPreferred
        assert_eq!(
            evaluate_audio_stream_health(
                true,
                "MacBook Pro Microphone",
                Some("AirPods Pro"),
                &devices,
                false,
                Some(20),
            ),
            AudioStreamRecoveryAction::SwitchBackToPreferred {
                preferred: "AirPods Pro".to_string()
            }
        );
    }

    #[test]
    fn test_system_sleep_wall_clock_jump_detection() {
        assert_eq!(
            detect_system_sleep_or_stall_jump(1_000_000, 1_000_015, SYSTEM_SLEEP_JUMP_THRESHOLD_MS),
            None
        );
        assert_eq!(
            detect_system_sleep_or_stall_jump(1_000_000, 1_045_000, SYSTEM_SLEEP_JUMP_THRESHOLD_MS),
            Some(45_000)
        );
    }
}
