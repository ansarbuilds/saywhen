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
        }
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

/// Pure check whether a head pose is inside the tighter arm-enter cone.
pub fn is_within_arm_enter_cone(
    vision: &VisionTelemetryPayload,
    cfg: &TurnTakingConfig,
) -> bool {
    if !vision.face_present {
        return false;
    }
    // If legacy/synthetic telemetry only set `facing_screen` with zero angles, honor it.
    if vision.yaw == 0.0 && vision.pitch == 0.0 && !vision.facing_screen {
        return false;
    }
    vision.yaw.abs() <= cfg.arm_yaw_enter_deg && vision.pitch.abs() <= cfg.arm_pitch_enter_deg
}

/// Pure check whether a head pose is still inside the wider arm-exit cone.
pub fn is_within_arm_exit_cone(
    vision: &VisionTelemetryPayload,
    cfg: &TurnTakingConfig,
) -> bool {
    if !vision.face_present {
        return false;
    }
    if vision.yaw == 0.0 && vision.pitch == 0.0 && !vision.facing_screen {
        return false;
    }
    vision.yaw.abs() <= cfg.arm_yaw_exit_deg && vision.pitch.abs() <= cfg.arm_pitch_exit_deg
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
