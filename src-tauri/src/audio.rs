use crate::cleanup::{
    check_destructive_command, classify_llm_endpoint_privacy, is_preview_spoken_cancel_command,
    run_local_developer_cleanup_with_vocab, run_opt_in_llm_cleanup, stitch_continued_transcript,
    CleanupResult, LlmCleanupConfig,
};
use crate::history::append_history_entry;
use crate::inject::{
    default_auto_send_allowlist, detect_frontmost_app, evaluate_app_injection_policy,
    evaluate_gaze_window_alignment, inject_text_into_focused_app, AppTargetPolicy,
    InjectionOutcome, InjectionTimingConfig, TargetChangePolicy,
    DEFAULT_CLIPBOARD_RESTORE_DELAY_MS, DEFAULT_ENTER_DELAY_MS,
};
use crate::profiles::{match_app_profile, merge_vocab};
use crate::secrets::{
    take_legacy_llm_api_key, validate_secret_value, DefaultSecretStore, SecretStore,
    LLM_CLEANUP_API_KEY,
};
use crate::snippets::{apply_snippets, sanitize_snippets, Snippet};
use crate::transcribe::{TranscriberEngine, TranscriptionResult};
use crate::voice_commands::{
    describe_voice_command, parse_voice_command, VoiceCommand, VoiceCommandEvent,
};
/// Wake word decision log (terminal, plus a temp file in development builds).
macro_rules! wake_log {
    ($($arg:tt)*) => {
        crate::wake_word::wake_log_line(&format!($($arg)*))
    };
}

use crate::turn_taking::{
    append_session_metrics_line, detect_system_sleep_or_stall_jump, end_of_turn,
    ends_with_connective, evaluate_arming_hysteresis, evaluate_audio_stream_health,
    has_minimum_speech_evidence, is_within_arm_exit_cone, preview_deadline, resolve_recordings_dir,
    sanitize_discarded_reason_for_metrics, save_tuning_utterance_wav_and_sidecar,
    should_auto_start, should_cancel_preview_on_speech, should_count_vad_frame_in_turn,
    silence_countdown_ms, ArmingHysteresisState, AudioStreamRecoveryAction, EndReason, GateInputs,
    PreviewDeadline, RecordedVisionSample, SendMode, TuningRecordingSidecar, TurnState,
    TurnTakingConfig, UtteranceMetricsRecord, VisionPowerMode, DEFAULT_ARM_DWELL_MS,
    DEFAULT_ARM_PITCH_ENTER_DEG, DEFAULT_ARM_PITCH_EXIT_DEG, DEFAULT_ARM_YAW_ENTER_DEG,
    DEFAULT_ARM_YAW_EXIT_DEG, DEFAULT_CALIBRATED_RESTING_MAR, DEFAULT_DISARM_DWELL_MS,
    DEFAULT_MAX_SILENCE_TIMEOUT_MS, DEFAULT_MIN_SPEECH_HOTKEY_MS, DEFAULT_MIN_SPEECH_MS,
    DEFAULT_POST_ROLL_MS, MAX_CALIBRATED_RESTING_MAR, MAX_CALIBRATED_YAW_OFFSET_DEG,
    NOD_WAIT_TIMEOUT_MS, SYSTEM_SLEEP_JUMP_THRESHOLD_MS, VAD_FRAME_MS,
};
use crate::wake_word::{
    early_wake_verdict_with_profile, effective_wake_word, extract_leading_wake_alias, first_word,
    matches_voice_profile, merge_into_voice_profile, should_clear_wake_quiet_wait,
    strip_wake_word_with_profile, wake_holds_turn, wake_vad_threshold, word_count,
    EarlyWakeVerdict, VoiceProfile, DEFAULT_WAKE_WORD, VOICE_ENROLL_COMMAND_SUFFIXES,
    WAKE_CHECK_MAX_SAMPLES, WAKE_CHECK_MIN_SAMPLES, WAKE_CHECK_MIN_SAMPLES_ON_PAUSE,
    WAKE_DIAG_SOUND_LEVEL, WAKE_REARM_QUIET_MS, WAKE_START_MIN_VAD_FRAMES, WAKE_WORD_TRIGGER,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use voice_activity_detector::VoiceActivityDetector;

const TARGET_SAMPLE_RATE: u32 = 16_000;
const PREROLL_SAMPLES: usize = 8_000; // 500 ms at 16 kHz
const VAD_CHUNK_SAMPLES: usize = 512; // 32 ms at 16 kHz (required by Silero VAD v5)
const MAX_UTTERANCE_SAMPLES: usize = 16_000 * 60; // 60 seconds cap

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    IDLE,
    ARMED,
    LISTENING,
    TRANSCRIBING,
    PREVIEW,
}

impl PipelineState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineState::IDLE => "IDLE",
            PipelineState::ARMED => "ARMED",
            PipelineState::LISTENING => "LISTENING",
            PipelineState::TRANSCRIBING => "TRANSCRIBING",
            PipelineState::PREVIEW => "PREVIEW",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionTelemetryPayload {
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

impl Default for VisionTelemetryPayload {
    fn default() -> Self {
        Self {
            face_present: false,
            facing_screen: false,
            lips_moving: false,
            mouth_closed: true,
            mar: 0.0,
            mar_variance: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            jaw_open: 0.0,
            gesture: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationConfig {
    pub hands_free_enabled: bool,
    pub vad_threshold: f32,
    pub silence_timeout_ms: u64,
    #[serde(default = "default_max_silence_timeout_ms")]
    pub max_silence_timeout_ms: u64,
    pub auto_stop_on_silence: bool,
    pub cleanup_enabled: bool,
    pub preview_enabled: bool,
    pub preview_countdown_ms: u64,
    /// How a finished utterance gets sent: "pause" | "nod" | "phrase" | "paste_only".
    #[serde(default)]
    pub send_mode: SendMode,
    /// Talking during the preview countdown cancels it.
    #[serde(default = "default_true")]
    pub cancel_on_speech: bool,
    pub cancel_on_look_away: bool,
    pub per_app_rules_enabled: bool,
    pub auto_send_allowlist: Vec<String>,
    pub auto_inject: bool,
    pub auto_press_enter: bool,
    pub use_clipboard_paste: bool,
    pub preferred_device: Option<String>,
    /// Show the floating always-on-top HUD pill. Defaulted so older settings files still load.
    #[serde(default = "default_true")]
    pub hud_enabled: bool,
    /// Global push-to-talk / hands-free shortcut, in global-hotkey syntax (for example "Alt+Space").
    #[serde(default = "default_hotkey_accelerator")]
    pub hotkey_accelerator: String,
    /// Action when the focused app changes during the preview countdown ("abort" | "paste_only").
    #[serde(default)]
    pub on_target_changed: TargetChangePolicy,
    /// Base delay (ms) between Cmd/Ctrl+V and pressing Return. Scaled with text length.
    #[serde(default = "default_enter_delay_ms")]
    pub enter_delay_ms: u64,
    /// Optional per-app base Enter delay overrides keyed by app name, bundle id, or exe.
    #[serde(default)]
    pub per_app_enter_delay_ms: HashMap<String, u64>,
    /// Delay (ms) before restoring the previous clipboard if unchanged.
    #[serde(default = "default_clipboard_restore_delay_ms")]
    pub clipboard_restore_delay_ms: u64,
    /// Arming hysteresis enter yaw threshold in degrees (|yaw| <= this to start arming dwell).
    #[serde(default = "default_arm_yaw_enter_deg")]
    pub arm_yaw_enter_deg: f32,
    /// Arming hysteresis exit yaw threshold in degrees (|yaw| > this for disarm_dwell_ms to disarm).
    #[serde(default = "default_arm_yaw_exit_deg")]
    pub arm_yaw_exit_deg: f32,
    /// Arming hysteresis enter pitch threshold in degrees.
    #[serde(default = "default_arm_pitch_enter_deg")]
    pub arm_pitch_enter_deg: f32,
    /// Arming hysteresis exit pitch threshold in degrees.
    #[serde(default = "default_arm_pitch_exit_deg")]
    pub arm_pitch_exit_deg: f32,
    /// Dwell time (ms) inside the enter cone before IDLE -> ARMED so glancing past does not arm.
    #[serde(default = "default_arm_dwell_ms")]
    pub arm_dwell_ms: u64,
    /// Dwell time (ms) outside the exit cone before ARMED -> IDLE so edge poses do not flicker.
    #[serde(default = "default_disarm_dwell_ms")]
    pub disarm_dwell_ms: u64,
    /// Minimum voiced speech duration (ms) in hands-free mode before transcribing.
    #[serde(default = "default_min_speech_ms")]
    pub min_speech_ms: u64,
    /// Minimum voiced speech duration (ms) for hotkey/manual starts.
    #[serde(default = "default_min_speech_hotkey_ms")]
    pub min_speech_hotkey_ms: u64,
    /// Post-roll audio tail (ms) captured after end-of-turn so final words are not clipped.
    #[serde(default = "default_post_roll_ms")]
    pub post_roll_ms: u64,
    /// Opt-in session metrics JSONL log in the OS app data dir (never contains transcript or audio).
    #[serde(default)]
    pub session_metrics_enabled: bool,
    /// Opt-in utterance WAV + JSON sidecar saving to local `recordings/` for tuning.
    /// Always resets to `false` when the app restarts.
    #[serde(default)]
    pub save_tuning_audio_enabled: bool,
    /// Per-user calibrated neutral head yaw offset in degrees (0.0 = centered on webcam).
    #[serde(default)]
    pub calibrated_yaw_offset_deg: f32,
    /// Per-user calibrated neutral head pitch offset in degrees.
    #[serde(default)]
    pub calibrated_pitch_offset_deg: f32,
    /// Per-user calibrated resting closed-mouth MAR baseline.
    #[serde(default = "default_calibrated_resting_mar")]
    pub calibrated_resting_mar: f32,
    /// Voice-only fallback: when the camera does not see you facing the screen, speech that
    /// starts with the wake word still goes through. Off by default.
    #[serde(default)]
    pub wake_word_enabled: bool,
    /// The wake word (default "eevee"; common spellings are matched too).
    #[serde(default = "default_wake_word")]
    pub wake_word: String,
    /// Play a short chime when the wake word is heard (played by the main window).
    #[serde(default = "default_true")]
    pub wake_chime_enabled: bool,
    /// Colour palette for the main window and HUD (presentation only).
    #[serde(default = "default_ui_palette")]
    pub ui_palette: String,
    /// Each busy pipeline state borrows its own palette (presentation only).
    #[serde(default = "default_true")]
    pub mood_colors: bool,
    /// Webcam is off to the side of the screen the user talks to (widens the head-turn allowance).
    #[serde(default)]
    pub side_camera_mode: bool,
    /// Adaptive vision frame rate (low FPS in IDLE when away/no-face, 15 FPS when facing screen or armed).
    #[serde(default = "default_true")]
    pub adaptive_fps_enabled: bool,
    /// Optional user-configured developer vocabulary terms (comma- or newline-separated).
    #[serde(default)]
    pub custom_vocab: String,
    /// Spoken phrase replacements, applied after clean-up.
    #[serde(default)]
    pub snippets: Vec<Snippet>,
    /// Send style, extra words, or language for one app.
    #[serde(default)]
    pub app_profiles: Vec<crate::profiles::AppProfile>,
    /// Camera frame budget: auto, full, or low.
    #[serde(default)]
    pub vision_power_mode: VisionPowerMode,
    /// Whisper language. English-only models stay on English.
    #[serde(default = "default_transcription_language")]
    pub transcription_language: String,
    /// Persist recent utterances locally in `<app_data_dir>/utterance-history.jsonl`, auto-purged after 7 days.
    #[serde(default = "default_true")]
    pub history_persistence_enabled: bool,
    /// Opt-in secondary AI cleanup pass via local Ollama (`127.0.0.1:11434`) or OpenAI-compatible BYOK endpoint.
    #[serde(default)]
    pub llm_cleanup_enabled: bool,
    /// OpenAI-compatible chat completions URL (defaults to local Ollama loopback).
    #[serde(default = "default_llm_endpoint_url")]
    pub llm_endpoint_url: String,
    /// Model identifier passed to the LLM cleanup endpoint.
    #[serde(default = "default_llm_model")]
    pub llm_model: String,
    /// Optional 3-phrase Siri-style personal voice profile (pitch, spectral shape, near-field RMS,
    /// and learned Whisper wake-word spellings).
    #[serde(default)]
    pub voice_profile: Option<VoiceProfile>,
}

fn default_hotkey_accelerator() -> String {
    "Alt+Space".to_string()
}

fn default_llm_endpoint_url() -> String {
    "http://127.0.0.1:11434/v1/chat/completions".to_string()
}

fn default_llm_model() -> String {
    "qwen2.5-coder:1.5b".to_string()
}

fn default_transcription_language() -> String {
    "en".to_string()
}

fn default_wake_word() -> String {
    DEFAULT_WAKE_WORD.to_string()
}

fn default_ui_palette() -> String {
    "saywhen".to_string()
}

fn default_calibrated_resting_mar() -> f32 {
    DEFAULT_CALIBRATED_RESTING_MAR
}

fn default_true() -> bool {
    true
}

fn default_enter_delay_ms() -> u64 {
    DEFAULT_ENTER_DELAY_MS
}

fn default_clipboard_restore_delay_ms() -> u64 {
    DEFAULT_CLIPBOARD_RESTORE_DELAY_MS
}

fn default_max_silence_timeout_ms() -> u64 {
    DEFAULT_MAX_SILENCE_TIMEOUT_MS
}

fn default_arm_yaw_enter_deg() -> f32 {
    DEFAULT_ARM_YAW_ENTER_DEG
}

fn default_arm_yaw_exit_deg() -> f32 {
    DEFAULT_ARM_YAW_EXIT_DEG
}

fn default_arm_pitch_enter_deg() -> f32 {
    DEFAULT_ARM_PITCH_ENTER_DEG
}

fn default_arm_pitch_exit_deg() -> f32 {
    DEFAULT_ARM_PITCH_EXIT_DEG
}

fn default_arm_dwell_ms() -> u64 {
    DEFAULT_ARM_DWELL_MS
}

fn default_disarm_dwell_ms() -> u64 {
    DEFAULT_DISARM_DWELL_MS
}

fn default_min_speech_ms() -> u64 {
    DEFAULT_MIN_SPEECH_MS
}

fn default_min_speech_hotkey_ms() -> u64 {
    DEFAULT_MIN_SPEECH_HOTKEY_MS
}

fn default_post_roll_ms() -> u64 {
    DEFAULT_POST_ROLL_MS
}

impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            hands_free_enabled: true,
            vad_threshold: 0.45,
            silence_timeout_ms: 650,
            max_silence_timeout_ms: DEFAULT_MAX_SILENCE_TIMEOUT_MS,
            auto_stop_on_silence: true,
            cleanup_enabled: true,
            preview_enabled: true,
            preview_countdown_ms: 1800,
            send_mode: SendMode::Pause,
            cancel_on_speech: true,
            cancel_on_look_away: false,
            per_app_rules_enabled: true,
            auto_send_allowlist: default_auto_send_allowlist(),
            auto_inject: true,
            auto_press_enter: true,
            use_clipboard_paste: true,
            preferred_device: None,
            hud_enabled: true,
            hotkey_accelerator: default_hotkey_accelerator(),
            on_target_changed: TargetChangePolicy::Abort,
            enter_delay_ms: DEFAULT_ENTER_DELAY_MS,
            per_app_enter_delay_ms: HashMap::new(),
            clipboard_restore_delay_ms: DEFAULT_CLIPBOARD_RESTORE_DELAY_MS,
            arm_yaw_enter_deg: DEFAULT_ARM_YAW_ENTER_DEG,
            arm_yaw_exit_deg: DEFAULT_ARM_YAW_EXIT_DEG,
            arm_pitch_enter_deg: DEFAULT_ARM_PITCH_ENTER_DEG,
            arm_pitch_exit_deg: DEFAULT_ARM_PITCH_EXIT_DEG,
            arm_dwell_ms: DEFAULT_ARM_DWELL_MS,
            disarm_dwell_ms: DEFAULT_DISARM_DWELL_MS,
            min_speech_ms: DEFAULT_MIN_SPEECH_MS,
            min_speech_hotkey_ms: DEFAULT_MIN_SPEECH_HOTKEY_MS,
            post_roll_ms: DEFAULT_POST_ROLL_MS,
            session_metrics_enabled: false,
            save_tuning_audio_enabled: false,
            calibrated_yaw_offset_deg: 0.0,
            calibrated_pitch_offset_deg: 0.0,
            calibrated_resting_mar: DEFAULT_CALIBRATED_RESTING_MAR,
            wake_word_enabled: false,
            wake_word: default_wake_word(),
            wake_chime_enabled: true,
            ui_palette: default_ui_palette(),
            mood_colors: true,
            side_camera_mode: false,
            adaptive_fps_enabled: true,
            custom_vocab: String::new(),
            snippets: Vec::new(),
            app_profiles: Vec::new(),
            vision_power_mode: VisionPowerMode::Auto,
            transcription_language: default_transcription_language(),
            history_persistence_enabled: true,
            llm_cleanup_enabled: false,
            llm_endpoint_url: default_llm_endpoint_url(),
            llm_model: default_llm_model(),
            voice_profile: None,
        }
    }
}

impl DictationConfig {
    pub fn to_injection_timing(&self) -> InjectionTimingConfig {
        InjectionTimingConfig {
            use_clipboard_paste: self.use_clipboard_paste,
            enter_delay_ms: self.enter_delay_ms,
            per_app_enter_delay_ms: self.per_app_enter_delay_ms.clone(),
            clipboard_restore_delay_ms: self.clipboard_restore_delay_ms,
            on_target_changed: self.on_target_changed,
        }
    }

    pub fn to_turn_taking_config(&self) -> TurnTakingConfig {
        TurnTakingConfig {
            hands_free_enabled: self.hands_free_enabled,
            vad_threshold: self.vad_threshold,
            silence_timeout_ms: self.silence_timeout_ms,
            max_silence_timeout_ms: self.max_silence_timeout_ms,
            auto_stop_on_silence: self.auto_stop_on_silence,
            arm_yaw_enter_deg: self.arm_yaw_enter_deg,
            arm_yaw_exit_deg: self.arm_yaw_exit_deg,
            arm_pitch_enter_deg: self.arm_pitch_enter_deg,
            arm_pitch_exit_deg: self.arm_pitch_exit_deg,
            arm_dwell_ms: self.arm_dwell_ms,
            disarm_dwell_ms: self.disarm_dwell_ms,
            min_speech_ms: self.min_speech_ms,
            min_speech_hotkey_ms: self.min_speech_hotkey_ms,
            post_roll_ms: self.post_roll_ms,
            calibrated_yaw_offset_deg: self.calibrated_yaw_offset_deg,
            calibrated_pitch_offset_deg: self.calibrated_pitch_offset_deg,
            calibrated_resting_mar: self.calibrated_resting_mar,
            adaptive_fps_enabled: self.adaptive_fps_enabled,
            side_camera_mode: self.side_camera_mode,
        }
    }

    pub fn sanitize_after_parse(&mut self) {
        self.auto_send_allowlist
            .retain(|entry| !entry.trim().eq_ignore_ascii_case("saywhen"));
        // Saving utterance audio for tuning must always turn itself off when the app restarts.
        self.save_tuning_audio_enabled = false;
        self.calibrated_yaw_offset_deg = self.calibrated_yaw_offset_deg.clamp(
            -MAX_CALIBRATED_YAW_OFFSET_DEG,
            MAX_CALIBRATED_YAW_OFFSET_DEG,
        );
        self.calibrated_resting_mar = self
            .calibrated_resting_mar
            .clamp(0.006, MAX_CALIBRATED_RESTING_MAR);
        sanitize_snippets(&mut self.snippets);
        crate::profiles::sanitize_profiles(&mut self.app_profiles);
        self.transcription_language =
            crate::transcribe::normalize_transcription_language(&self.transcription_language);
    }

    /// Writable settings file. Lives next to utterance history so saves do not depend on
    /// the process working directory (a `.app` launch uses `/`, which is read-only).
    pub fn settings_file_path(app_data_dir: &std::path::Path) -> std::path::PathBuf {
        app_data_dir.join("settings.local.json")
    }

    /// Older builds stored settings next to the repo, relative to the working directory.
    fn legacy_settings_candidates() -> Vec<std::path::PathBuf> {
        let mut paths = vec![
            std::path::PathBuf::from("../settings.local.json"),
            std::path::PathBuf::from("settings.local.json"),
        ];
        // Dev binaries are often opened with cwd `/`. This compile-time path still finds
        // the repo copy on the machine that built the app, and is a no-op if it is absent.
        paths.push(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../settings.local.json"),
        );
        paths
    }

    fn read_settings_file(path: &std::path::Path) -> Option<Self> {
        let raw = std::fs::read_to_string(path).ok()?;
        let mut parsed = serde_json::from_str::<DictationConfig>(&raw).ok()?;
        parsed.sanitize_after_parse();
        Some(parsed)
    }

    /// Loads `<app_data_dir>/settings.local.json`. If that file does not exist yet, copies
    /// the first readable legacy `settings.local.json` into the app data directory.
    pub fn load_with_legacy(app_data_dir: &std::path::Path, legacy: &[std::path::PathBuf]) -> Self {
        let path = Self::settings_file_path(app_data_dir);
        if let Some(parsed) = Self::read_settings_file(&path) {
            return parsed;
        }
        for candidate in legacy {
            if let Some(parsed) = Self::read_settings_file(candidate) {
                if let Err(err) = parsed.save_to_path(&path) {
                    eprintln!(
                        "saywhen: could not migrate settings from {} to {}: {err}",
                        candidate.display(),
                        path.display()
                    );
                }
                return parsed;
            }
        }
        Self::default()
    }

    pub fn load_for_app_data(app_data_dir: &std::path::Path) -> Self {
        Self::load_with_legacy(app_data_dir, &Self::legacy_settings_candidates())
    }

    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Could not create settings directory {}: {e}",
                        parent.display()
                    )
                })?;
            }
        }
        let mut persisted = self.clone();
        // Never persist `save_tuning_audio_enabled: true` across restarts.
        persisted.save_tuning_audio_enabled = false;
        let json = serde_json::to_string_pretty(&persisted)
            .map_err(|e| format!("Could not save settings: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Could not save settings to {}: {e}", path.display()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTelemetryEvent {
    pub state: String,
    pub rms: f32,
    pub vad_prob: f32,
    pub is_speech: bool,
    pub pre_roll_ms: u64,
    pub listening_duration_ms: u64,
    pub preview_remaining_ms: u64,
    pub silence_remaining_ms: u64,
    pub silence_total_ms: u64,
    pub post_roll_active: bool,
    pub device_name: String,
    pub speech_detected_in_utterance: bool,
    pub vision_active: bool,
    pub vision_armed: bool,
    pub vision_lips_moving: bool,
    /// The user turned the camera gate on in the UI. Combined with `vision_active == false`
    /// this means vision frames stopped arriving (for example a throttled hidden webview).
    pub camera_enabled: bool,
    pub hud_enabled: bool,
    pub save_tuning_audio_enabled: bool,
    pub session_metrics_enabled: bool,
    #[serde(default)]
    pub partial_transcript: String,
    #[serde(default)]
    pub partial_ends_with_connective: bool,
    /// Active send mode, so the HUD can show the right hint while you talk.
    #[serde(default)]
    pub send_mode: String,
    /// Listening was started by voice alone and the wake word is not confirmed yet.
    #[serde(default)]
    pub wake_pending: bool,
    /// The engine has the wake word switched on (shows the setting actually took effect).
    #[serde(default)]
    pub wake_word_active: bool,
    /// Chosen colour palette, so the HUD matches the main window from the first frame.
    #[serde(default)]
    pub ui_palette: String,
    #[serde(default)]
    pub mood_colors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStartedEvent {
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
    pub target_policy: AppTargetPolicy,
    pub countdown_ms: u64,
    pub trigger_source: String,
    /// The send mode that was active when this preview started.
    pub send_mode: String,
    /// True when this preview waits for a nod (or Enter) instead of counting down.
    pub waits_for_nod: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtteranceCompleteEvent {
    pub utterance_id: u64,
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
    pub target_policy: AppTargetPolicy,
    pub injection: Option<InjectionOutcome>,
    pub injection_error: Option<String>,
    pub trigger_source: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub flagged_false_trigger: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PracticeCompleteEvent {
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEnrollStepEvent {
    pub step: usize,
    pub total_steps: usize,
    pub heard_text: String,
    pub learned_alias: Option<String>,
    pub profile: VoiceProfile,
}

#[derive(Debug, Clone)]
pub struct PendingPreviewItem {
    pub utterance_id: u64,
    pub started_at: Instant,
    pub countdown_ms: u64,
    pub send_mode: SendMode,
    pub waits_for_nod: bool,
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
    pub target_policy: AppTargetPolicy,
    pub trigger_source: String,
    pub lip_to_listening_ms: Option<u64>,
    pub speech_frames: usize,
    pub end_reason: EndReason,
    pub post_roll_ms: u64,
    pub tuning_capture: Option<(Vec<f32>, Vec<RecordedVisionSample>)>,
}

struct CapturedUtterance {
    utterance_id: u64,
    samples: Vec<f32>,
    peak_vad: f32,
    trigger_source: String,
    lip_to_listening_ms: Option<u64>,
    speech_frames: usize,
    end_reason: EndReason,
    post_roll_ms: u64,
    vision_trace: Vec<RecordedVisionSample>,
    vision_fresh: bool,
    latest_yaw_deg: f32,
    carried_transcript_prefix: Option<String>,
}

struct SharedCaptureBuffer {
    epoch: Instant,
    next_utterance_id: u64,
    pre_roll: VecDeque<f32>,
    vad_pending: VecDeque<f32>,
    utterance_pcm: Vec<f32>,
    /// Samples captured during TRANSCRIBING / PREVIEW so continuing speech never drops a syllable.
    continuation_pcm: Vec<f32>,
    state: PipelineState,
    listening_started_at: Option<Instant>,
    listening_started_at_ms: u64,
    pending_preview: Option<PendingPreviewItem>,
    carried_transcript_prefix: Option<String>,
    speech_frames_in_utterance: usize,
    consecutive_vad_frames: usize,
    last_speech_at_ms: u64,
    last_vad_voice_at_ms: Option<u64>,
    last_lip_motion_in_turn_ms: Option<u64>,
    lip_to_listening_ms: Option<u64>,
    post_roll_started_at_ms: Option<u64>,
    pending_end_reason: Option<EndReason>,
    peak_vad_prob: f32,
    peak_speech_rms: f32,
    latest_rms: f32,
    latest_vad_prob: f32,
    trigger_source: String,
    vision: VisionTelemetryPayload,
    last_vision_update: Option<Instant>,
    arming_hysteresis: ArmingHysteresisState,
    vision_trace: VecDeque<(u64, VisionTelemetryPayload)>,
    camera_enabled: bool,
    last_audio_callback_at: Option<Instant>,
    stream_faulted: bool,
    partial_transcript: String,
    partial_ends_with_connective: bool,
    partial_checked_speech_frames: usize,
    partial_check_in_flight: bool,
    /// Wake word candidate: the early check found the wake word at the start.
    wake_confirmed: bool,
    wake_check_in_flight: bool,
    /// The early check runs once per candidate.
    wake_check_done: bool,
    /// Only the wake word has been heard so far; give the user time to say the command.
    wake_awaiting_command: bool,
    wake_confirmed_at_ms: u64,
    /// After a dropped wake candidate, wait for a short quiet gap before starting another.
    wake_needs_quiet: bool,
    wake_dropped_at_ms: Option<u64>,
    wake_floor_rms: f32,
    /// Consecutive frames over the (lower) wake word voice threshold.
    wake_consecutive_frames: usize,
    /// Diagnostics: a burst of sound while waiting that never became a wake candidate.
    wake_burst_frames: usize,
    wake_burst_peak: f32,
}

impl SharedCaptureBuffer {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            next_utterance_id: unix_timestamp_ms().max(1),
            pre_roll: VecDeque::with_capacity(PREROLL_SAMPLES + 1024),
            vad_pending: VecDeque::with_capacity(VAD_CHUNK_SAMPLES * 4),
            utterance_pcm: Vec::with_capacity(16_000 * 10),
            continuation_pcm: Vec::with_capacity(16_000 * 4),
            state: PipelineState::IDLE,
            listening_started_at: None,
            listening_started_at_ms: 0,
            pending_preview: None,
            carried_transcript_prefix: None,
            speech_frames_in_utterance: 0,
            consecutive_vad_frames: 0,
            last_speech_at_ms: 0,
            last_vad_voice_at_ms: None,
            last_lip_motion_in_turn_ms: None,
            lip_to_listening_ms: None,
            post_roll_started_at_ms: None,
            pending_end_reason: None,
            peak_vad_prob: 0.0,
            peak_speech_rms: 0.0,
            latest_rms: 0.0,
            latest_vad_prob: 0.0,
            trigger_source: "manual".to_string(),
            vision: VisionTelemetryPayload::default(),
            last_vision_update: None,
            arming_hysteresis: ArmingHysteresisState::default(),
            vision_trace: VecDeque::with_capacity(256),
            camera_enabled: false,
            last_audio_callback_at: None,
            stream_faulted: false,
            partial_transcript: String::new(),
            partial_ends_with_connective: false,
            partial_checked_speech_frames: 0,
            partial_check_in_flight: false,
            wake_confirmed: false,
            wake_check_in_flight: false,
            wake_check_done: false,
            wake_awaiting_command: false,
            wake_confirmed_at_ms: 0,
            wake_needs_quiet: false,
            wake_dropped_at_ms: None,
            wake_floor_rms: 0.0,
            wake_consecutive_frames: 0,
            wake_burst_frames: 0,
            wake_burst_peak: 0.0,
        }
    }

    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn is_vision_fresh(&self) -> bool {
        self.last_vision_update
            .map(|t| t.elapsed() < Duration::from_millis(650))
            .unwrap_or(false)
    }

    fn resting_state(&self, turn_cfg: &TurnTakingConfig) -> PipelineState {
        if turn_cfg.hands_free_enabled
            && self.is_vision_fresh()
            && is_within_arm_exit_cone(&self.vision, turn_cfg)
        {
            PipelineState::ARMED
        } else {
            PipelineState::IDLE
        }
    }

    fn begin_listening(&mut self, source: &str, seed_speech_frames: usize) {
        let now_ms = self.now_ms();
        self.utterance_pcm.clear();
        self.continuation_pcm.clear();
        // Prepend the 500ms circular pre-roll ring buffer so the first syllable is never clipped
        self.utterance_pcm.extend(self.pre_roll.iter().copied());
        self.state = PipelineState::LISTENING;
        self.listening_started_at = Some(Instant::now());
        self.listening_started_at_ms = now_ms;
        self.pending_preview = None;
        self.speech_frames_in_utterance = seed_speech_frames;
        self.last_speech_at_ms = now_ms;
        self.last_lip_motion_in_turn_ms = if self.vision.lips_moving {
            Some(now_ms)
        } else {
            None
        };
        self.lip_to_listening_ms = self.arming_hysteresis.take_lip_to_listening_ms(now_ms);
        self.post_roll_started_at_ms = None;
        self.pending_end_reason = None;
        self.peak_vad_prob = self.latest_vad_prob;
        self.peak_speech_rms = self.latest_rms;
        self.trigger_source = source.to_string();
        self.partial_transcript.clear();
        self.partial_ends_with_connective = false;
        self.partial_checked_speech_frames = 0;
        self.partial_check_in_flight = false;
        self.wake_confirmed = false;
        self.wake_check_in_flight = false;
        self.wake_check_done = false;
        self.wake_awaiting_command = false;
        self.wake_confirmed_at_ms = 0;
        if source == WAKE_WORD_TRIGGER {
            wake_log!("[saywhen wake] voice heard while not facing the screen: checking for the wake word");
        }
    }

    /// Upgrade an unconfirmed wake-word candidate to a confirmed trigger (camera or hotkey)
    /// without clearing `utterance_pcm` so speech already spoken is never lost.
    fn upgrade_wake_candidate_to(&mut self, source: &str) {
        self.trigger_source = source.to_string();
        self.wake_confirmed = false;
        self.wake_check_in_flight = false;
        self.wake_check_done = true;
        self.wake_awaiting_command = false;
        self.wake_confirmed_at_ms = 0;
        self.last_speech_at_ms = self.now_ms();
    }

    /// Resume listening after `TRANSCRIBING` / `PREVIEW`, keeping every sample captured in
    /// `continuation_pcm` so speech spoken while transcribing is never dropped.
    fn resume_listening_with_continuation(
        &mut self,
        source: &str,
        prefix: Option<String>,
        seed_speech_frames: usize,
        voice_profile: Option<&VoiceProfile>,
    ) {
        let carried_audio = std::mem::take(&mut self.continuation_pcm);
        self.begin_listening(source, seed_speech_frames);
        if !carried_audio.is_empty() {
            self.utterance_pcm.clear();
            let take_len = carried_audio.len().min(MAX_UTTERANCE_SAMPLES);
            self.utterance_pcm
                .extend_from_slice(&carried_audio[carried_audio.len() - take_len..]);
        }
        if let Some(vp) = voice_profile {
            self.peak_speech_rms = self.peak_speech_rms.max(vp.typical_rms * 0.70);
        }
        if source == WAKE_WORD_TRIGGER {
            self.wake_confirmed = true;
            self.wake_check_done = true;
        }
        if let Some(ref p) = prefix {
            self.partial_transcript = p.clone();
        }
        self.carried_transcript_prefix = prefix;
    }

    fn is_unconfirmed_wake_candidate(&self) -> bool {
        self.state == PipelineState::LISTENING
            && self.trigger_source == WAKE_WORD_TRIGGER
            && !self.wake_confirmed
    }

    fn build_turn_state(&self, hands_free_enabled: bool) -> TurnState {
        TurnState {
            started_at_ms: self.listening_started_at_ms,
            last_speech_at_ms: self.last_speech_at_ms,
            last_lip_motion_at_ms: self.last_lip_motion_in_turn_ms,
            speech_frames: self.speech_frames_in_utterance,
            utterance_samples: self.utterance_pcm.len(),
            max_utterance_samples: MAX_UTTERANCE_SAMPLES,
            hands_free_enabled,
            vision_fresh: self.is_vision_fresh(),
            lips_moving: self.vision.lips_moving,
            mouth_closed: self.vision.mouth_closed,
            gesture: self.vision.gesture.clone(),
            ends_with_connective: self.partial_ends_with_connective,
        }
    }

    fn take_utterance_for_transcription(
        &mut self,
        end_reason: EndReason,
        post_roll_ms: u64,
        capture_vision_trace: bool,
    ) -> CapturedUtterance {
        let utterance_id = self.next_utterance_id;
        self.next_utterance_id = self.next_utterance_id.saturating_add(1);
        self.state = PipelineState::TRANSCRIBING;
        self.listening_started_at = None;
        self.continuation_pcm.clear();
        let start_with_preroll_ms = self.listening_started_at_ms.saturating_sub(500);
        let samples = std::mem::take(&mut self.utterance_pcm);
        let peak_vad = self.peak_vad_prob;
        let trigger_source = self.trigger_source.clone();
        let lip_to_listening_ms = self.lip_to_listening_ms.take();
        let speech_frames = self.speech_frames_in_utterance;
        let vision_fresh = self.is_vision_fresh();
        let latest_yaw_deg = self.vision.yaw;
        let carried_transcript_prefix = self.carried_transcript_prefix.take();

        self.peak_vad_prob = 0.0;
        self.peak_speech_rms = 0.0;
        self.speech_frames_in_utterance = 0;
        self.last_lip_motion_in_turn_ms = None;
        self.post_roll_started_at_ms = None;
        self.pending_end_reason = None;
        self.partial_transcript.clear();
        self.partial_ends_with_connective = false;
        self.partial_checked_speech_frames = 0;
        self.partial_check_in_flight = false;

        let vision_trace = if capture_vision_trace {
            self.vision_trace
                .iter()
                .filter(|(t, _)| *t >= start_with_preroll_ms)
                .map(|(t, v)| RecordedVisionSample {
                    offset_ms: t.saturating_sub(start_with_preroll_ms),
                    face_present: v.face_present,
                    facing_screen: v.facing_screen,
                    lips_moving: v.lips_moving,
                    mouth_closed: v.mouth_closed,
                    mar: v.mar,
                    mar_variance: v.mar_variance,
                    yaw: v.yaw,
                    pitch: v.pitch,
                    jaw_open: v.jaw_open,
                    gesture: v.gesture.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };

        CapturedUtterance {
            utterance_id,
            samples,
            peak_vad,
            trigger_source,
            lip_to_listening_ms,
            speech_frames,
            end_reason,
            post_roll_ms,
            vision_trace,
            vision_fresh,
            latest_yaw_deg,
            carried_transcript_prefix,
        }
    }

    fn cancel_active_or_preview(&mut self, turn_cfg: &TurnTakingConfig) {
        self.state = self.resting_state(turn_cfg);
        self.utterance_pcm.clear();
        self.continuation_pcm.clear();
        self.listening_started_at = None;
        self.pending_preview = None;
        self.carried_transcript_prefix = None;
        self.speech_frames_in_utterance = 0;
        self.last_lip_motion_in_turn_ms = None;
        self.lip_to_listening_ms = None;
        self.post_roll_started_at_ms = None;
        self.pending_end_reason = None;
        self.peak_vad_prob = 0.0;
        self.peak_speech_rms = 0.0;
        self.partial_transcript.clear();
        self.partial_ends_with_connective = false;
        self.partial_checked_speech_frames = 0;
        self.partial_check_in_flight = false;
    }
}

pub enum AudioCommand {
    StartListening(String),
    StopAndTranscribe(Option<EndReason>),
    ToggleListening(String),
    CancelListening,
    ConfirmPreviewNow(Option<String>),
    CancelPreview(String),
    ResumeFromPreviewOnSpeech,
    BeginPreview(PendingPreviewItem),
    SwitchDevice(Option<String>),
    UpdateVision(VisionTelemetryPayload),
    SetCameraGateEnabled(bool),
    /// Silently drop a wake word candidate (identified by its listening start time) whose
    /// first words were not the wake word.
    DropWakeCandidate(u64),
}

pub struct AudioPipelineController {
    cmd_tx: Sender<AudioCommand>,
    config: Arc<Mutex<DictationConfig>>,
    shared: Arc<Mutex<SharedCaptureBuffer>>,
    settings_path: std::path::PathBuf,
    /// Opt-in clean-up API key. Never serialized with settings.
    llm_api_key: Arc<Mutex<String>>,
    app_data_dir: std::path::PathBuf,
}

impl AudioPipelineController {
    pub fn spawn(app_handle: AppHandle, transcriber: Arc<TranscriberEngine>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();
        let app_data_dir = resolve_settings_dir(&app_handle);
        let settings_path = DictationConfig::settings_file_path(&app_data_dir);
        let store = DefaultSecretStore::open(&app_data_dir);
        if let Some(legacy) = take_legacy_llm_api_key(&settings_path) {
            if validate_secret_value(&legacy).is_ok() {
                let _ = store.set(LLM_CLEANUP_API_KEY, &legacy);
            }
        }
        let loaded_key = store
            .get(LLM_CLEANUP_API_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();
        let llm_api_key = Arc::new(Mutex::new(loaded_key));
        let config = Arc::new(Mutex::new(DictationConfig::load_for_app_data(
            &app_data_dir,
        )));
        let shared = Arc::new(Mutex::new(SharedCaptureBuffer::new()));

        let config_clone = Arc::clone(&config);
        let shared_clone = Arc::clone(&shared);
        let cmd_tx_for_worker = cmd_tx.clone();
        let settings_path_for_worker = settings_path.clone();
        let llm_api_key_for_worker = Arc::clone(&llm_api_key);

        thread::spawn(move || {
            run_audio_worker(
                app_handle,
                transcriber,
                cmd_rx,
                cmd_tx_for_worker,
                config_clone,
                shared_clone,
                settings_path_for_worker,
                llm_api_key_for_worker,
            );
        });

        Self {
            cmd_tx,
            config,
            shared,
            settings_path,
            llm_api_key,
            app_data_dir,
        }
    }

    pub fn send_command(&self, cmd: AudioCommand) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|e| format!("Audio worker channel error: {}", e))
    }

    pub fn get_config(&self) -> DictationConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn update_config(&self, new_cfg: DictationConfig) -> Result<DictationConfig, String> {
        let mut device_changed = false;
        let preferred_device = new_cfg.preferred_device.clone();
        {
            let mut guard = self.config.lock().unwrap();
            if guard.preferred_device != preferred_device {
                device_changed = true;
            }
            *guard = new_cfg.clone();
        }
        let saved = new_cfg.save_to_path(&self.settings_path);
        if device_changed {
            let _ = self.send_command(AudioCommand::SwitchDevice(preferred_device));
        }
        // The new settings are live either way; a failed save is reported so it is not silent.
        saved.map(|_| new_cfg)
    }

    pub fn set_llm_api_key(&self, api_key: &str) -> Result<(), String> {
        let trimmed = api_key.trim();
        validate_secret_value(trimmed)?;
        DefaultSecretStore::open(&self.app_data_dir).set(LLM_CLEANUP_API_KEY, trimmed)?;
        *self.llm_api_key.lock().unwrap() = trimmed.to_string();
        Ok(())
    }

    pub fn clear_llm_api_key(&self) -> Result<(), String> {
        DefaultSecretStore::open(&self.app_data_dir).delete(LLM_CLEANUP_API_KEY)?;
        *self.llm_api_key.lock().unwrap() = String::new();
        Ok(())
    }

    pub fn llm_api_key_status(&self) -> (bool, String) {
        let store = DefaultSecretStore::open(&self.app_data_dir);
        let backend = store.backend(LLM_CLEANUP_API_KEY);
        let present = !self.llm_api_key.lock().unwrap().trim().is_empty() || backend != "none";
        (present, if present { backend } else { "none".to_string() })
    }

    pub fn reset_voice_profile(&self) -> Result<DictationConfig, String> {
        let updated = {
            let mut guard = self.config.lock().unwrap();
            guard.voice_profile = None;
            guard.clone()
        };
        updated.save_to_path(&self.settings_path)?;
        Ok(updated)
    }

    pub fn current_state(&self) -> String {
        self.shared.lock().unwrap().state.as_str().to_string()
    }

    pub fn is_in_preview(&self) -> bool {
        self.shared.lock().unwrap().state == PipelineState::PREVIEW
    }

    /// Flip the HUD pill visibility setting, persist it, and return the updated config.
    pub fn toggle_hud_enabled(&self) -> DictationConfig {
        let updated = {
            let mut guard = self.config.lock().unwrap();
            guard.hud_enabled = !guard.hud_enabled;
            guard.clone()
        };
        if let Err(e) = updated.save_to_path(&self.settings_path) {
            eprintln!("{e}");
        }
        updated
    }
}

/// In development builds only, the first word Whisper heard, so a misheard wake word can be
/// spotted in the terminal. Empty in release builds.
fn dev_first_word_note(text: &str) -> String {
    if !cfg!(debug_assertions) {
        return String::new();
    }
    match first_word(text) {
        Some(w) => format!(", first word \"{w}\""),
        None => ", nothing recognized".to_string(),
    }
}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for dev in devices {
            if let Ok(name) = dev.name() {
                names.push(name);
            }
        }
    }
    names
}

fn build_cpal_stream(
    preferred_device: Option<&str>,
    shared: Arc<Mutex<SharedCaptureBuffer>>,
    app_handle: AppHandle,
) -> Result<(cpal::Stream, String), String> {
    let host = cpal::default_host();
    let device = if let Some(pref) = preferred_device {
        host.input_devices()
            .map_err(|e| format!("Failed to enumerate audio input devices: {}", e))?
            .find(|d| d.name().map(|n| n == pref).unwrap_or(false))
            .or_else(|| host.default_input_device())
    } else {
        host.default_input_device()
    }
    .ok_or_else(|| "No microphone input device found on this system.".to_string())?;

    let device_name = device
        .name()
        .unwrap_or_else(|_| "Default Microphone".to_string());
    let supported_config = device.default_input_config().map_err(|e| {
        format!(
            "Failed to read microphone config for {}: {}",
            device_name, e
        )
    })?;

    let in_sample_rate = supported_config.sample_rate().0;
    let in_channels = supported_config.channels() as usize;
    let sample_format = supported_config.sample_format();
    let stream_config: cpal::StreamConfig = supported_config.into();

    let err_handle = app_handle.clone();
    let err_shared = Arc::clone(&shared);
    let err_fn = move |err: cpal::StreamError| {
        if let Ok(mut guard) = err_shared.lock() {
            guard.stream_faulted = true;
        }
        let _ = err_handle.emit(
            "pipeline-error",
            format!("Microphone stream error: {}", err),
        );
    };

    let process_samples = move |mono_input: &[f32]| {
        if mono_input.is_empty() {
            return;
        }
        let resampled = resample_to_16k(mono_input, in_sample_rate);
        if resampled.is_empty() {
            return;
        }

        let sum_sq: f32 = resampled.iter().map(|s| s * s).sum();
        let rms = (sum_sq / resampled.len() as f32).sqrt();

        let mut guard = shared.lock().unwrap();
        guard.latest_rms = rms;
        guard.last_audio_callback_at = Some(Instant::now());
        guard.stream_faulted = false;

        for &sample in &resampled {
            if guard.pre_roll.len() >= PREROLL_SAMPLES {
                guard.pre_roll.pop_front();
            }
            guard.pre_roll.push_back(sample);
            guard.vad_pending.push_back(sample);

            if guard.state == PipelineState::LISTENING
                && guard.utterance_pcm.len() < MAX_UTTERANCE_SAMPLES
            {
                guard.utterance_pcm.push(sample);
            } else if matches!(
                guard.state,
                PipelineState::TRANSCRIBING | PipelineState::PREVIEW
            ) && guard.continuation_pcm.len() < MAX_UTTERANCE_SAMPLES
            {
                guard.continuation_pcm.push(sample);
            }
        }
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let proc = process_samples;
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mono = downmix_to_mono_f32(data, in_channels);
                        proc(&mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to start F32 audio stream: {}", e))?
        }
        cpal::SampleFormat::I16 => {
            let proc = process_samples;
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mono: Vec<f32> = data
                            .chunks(in_channels.max(1))
                            .map(|frame| {
                                let sum: f32 = frame.iter().map(|&s| s as f32 / 32768.0).sum();
                                sum / frame.len() as f32
                            })
                            .collect();
                        proc(&mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to start I16 audio stream: {}", e))?
        }
        cpal::SampleFormat::U16 => {
            let proc = process_samples;
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let mono: Vec<f32> = data
                            .chunks(in_channels.max(1))
                            .map(|frame| {
                                let sum: f32 =
                                    frame.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).sum();
                                sum / frame.len() as f32
                            })
                            .collect();
                        proc(&mono);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to start U16 audio stream: {}", e))?
        }
        other => {
            return Err(format!("Unsupported microphone sample format: {:?}", other));
        }
    };

    stream
        .play()
        .map_err(|e| format!("Failed to play microphone stream: {}", e))?;

    Ok((stream, device_name))
}

fn downmix_to_mono_f32(data: &[f32], channels: usize) -> Vec<f32> {
    let ch = channels.max(1);
    if ch == 1 {
        return data.to_vec();
    }
    data.chunks(ch)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn resample_to_16k(input: &[f32], in_rate: u32) -> Vec<f32> {
    if in_rate == TARGET_SAMPLE_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = in_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let out_len = ((input.len() as f64) / ratio).floor() as usize;
    if out_len == 0 {
        return Vec::new();
    }
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = input[idx.min(input.len() - 1)];
        let s1 = input[(idx + 1).min(input.len() - 1)];
        output.push(s0 + frac * (s1 - s0));
    }
    output
}

fn resolve_settings_dir(app_handle: &AppHandle) -> std::path::PathBuf {
    match app_handle.path().app_data_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!(
                "saywhen: app data directory unavailable ({err}); settings will use a temporary directory"
            );
            std::env::temp_dir().join("saywhen")
        }
    }
}

pub fn unix_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn persist_metrics_and_optional_tuning_wav(
    app_handle: &AppHandle,
    cfg: &DictationConfig,
    record: &UtteranceMetricsRecord,
    tuning_capture: Option<(Vec<f32>, Vec<RecordedVisionSample>)>,
) {
    if cfg.session_metrics_enabled {
        if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
            if let Err(err) = append_session_metrics_line(&app_data_dir, record) {
                eprintln!("saywhen: session metrics write warning: {}", err);
            }
        }
    }

    if cfg.save_tuning_audio_enabled {
        if let Some((samples, vision_frames)) = tuning_capture {
            let sidecar = TuningRecordingSidecar {
                sample_rate: TARGET_SAMPLE_RATE,
                metrics: record.clone(),
                category: None,
                expected_phrase: None,
                expected_should_transcribe: None,
                vision_frames,
            };
            let recordings_dir = resolve_recordings_dir();
            if let Err(err) =
                save_tuning_utterance_wav_and_sidecar(&recordings_dir, &samples, &sidecar)
            {
                let _ = app_handle.emit("pipeline-error", err);
            }
        }
    }
}

fn finalize_preview_and_inject(
    app_handle: &AppHandle,
    mut item: PendingPreviewItem,
    override_text: Option<String>,
    cfg: &DictationConfig,
) {
    let final_text = override_text.unwrap_or_else(|| item.cleanup.cleaned_text.clone());
    let mut injection_outcome = None;
    let mut injection_error = None;
    let mut final_policy = item.target_policy.clone();

    if !final_text.trim().is_empty() && cfg.auto_inject {
        let timing = cfg.to_injection_timing();
        match inject_text_into_focused_app(app_handle, &final_text, &item.target_policy, &timing) {
            Ok(outcome) => {
                final_policy.app_name = outcome.target_app.clone();
                final_policy.policy = outcome.policy_applied.clone();
                final_policy.will_press_enter = outcome.pressed_enter;
                injection_outcome = Some(outcome);
            }
            Err(err) => {
                injection_error = Some(err.clone());
                let _ = app_handle.emit("pipeline-error", err);
            }
        }
    }

    let now_ms = unix_timestamp_ms();
    let outcome_label = if injection_error.is_some() {
        "aborted"
    } else if let Some(ref out) = injection_outcome {
        if out.pressed_enter {
            "sent"
        } else {
            "pasted"
        }
    } else {
        "transcribed_only"
    };

    let effective_end_reason = if item.transcription.end_phrase_triggered {
        EndReason::EndPhrase.as_str().to_string()
    } else {
        item.end_reason.as_str().to_string()
    };

    let metrics_rec = UtteranceMetricsRecord {
        utterance_id: item.utterance_id,
        timestamp_ms: now_ms,
        trigger_source: item.trigger_source.clone(),
        lip_to_listening_ms: item.lip_to_listening_ms,
        utterance_length_ms: item.transcription.audio_duration_ms,
        speech_frames: item.speech_frames,
        speech_duration_ms: (item.speech_frames as u64) * VAD_FRAME_MS,
        peak_vad: item.transcription.peak_vad_prob,
        end_reason: effective_end_reason,
        post_roll_ms: item.post_roll_ms,
        transcript_length: final_text.chars().count(),
        discarded_reason: None,
        outcome: outcome_label.to_string(),
        flag_false_trigger_clicks: 0,
    };

    persist_metrics_and_optional_tuning_wav(
        app_handle,
        cfg,
        &metrics_rec,
        item.tuning_capture.take(),
    );

    let mut final_cleanup = item.cleanup;
    final_cleanup.cleaned_text = final_text;

    let complete_event = UtteranceCompleteEvent {
        utterance_id: item.utterance_id,
        transcription: item.transcription,
        cleanup: final_cleanup,
        target_policy: final_policy,
        injection: injection_outcome,
        injection_error,
        trigger_source: item.trigger_source,
        timestamp_ms: now_ms,
        flagged_false_trigger: false,
    };

    if cfg.history_persistence_enabled {
        if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
            if let Err(err) = append_history_entry(&app_data_dir, &complete_event, now_ms) {
                eprintln!("saywhen: history write warning: {}", err);
            }
        }
    }

    let _ = app_handle.emit("utterance-complete", complete_event);
}

fn run_audio_worker(
    app_handle: AppHandle,
    transcriber: Arc<TranscriberEngine>,
    cmd_rx: mpsc::Receiver<AudioCommand>,
    cmd_tx_self: Sender<AudioCommand>,
    config: Arc<Mutex<DictationConfig>>,
    shared: Arc<Mutex<SharedCaptureBuffer>>,
    settings_path: std::path::PathBuf,
    llm_api_key: Arc<Mutex<String>>,
) {
    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(16000)
        .chunk_size(VAD_CHUNK_SAMPLES)
        .build()
    {
        Ok(v) => Some(v),
        Err(err) => {
            let _ = app_handle.emit(
                "pipeline-error",
                format!("Failed to initialize Silero VAD ONNX engine: {}", err),
            );
            None
        }
    };

    let initial_pref = config.lock().unwrap().preferred_device.clone();
    let mut active_stream: Option<cpal::Stream> = None;
    let mut active_device_name = "Unavailable".to_string();

    match build_cpal_stream(
        initial_pref.as_deref(),
        Arc::clone(&shared),
        app_handle.clone(),
    ) {
        Ok((stream, name)) => {
            active_stream = Some(stream);
            active_device_name = name;
        }
        Err(err) => {
            let _ = app_handle.emit("pipeline-error", err);
        }
    }

    let mut last_telemetry_emit = Instant::now();
    let mut last_health_check = Instant::now();
    let mut last_loop_wall_ms = unix_timestamp_ms();

    loop {
        let cfg = config.lock().unwrap().clone();
        let turn_cfg = cfg.to_turn_taking_config();

        let now_wall_ms = unix_timestamp_ms();
        if let Some(jump_ms) = detect_system_sleep_or_stall_jump(
            last_loop_wall_ms,
            now_wall_ms,
            SYSTEM_SLEEP_JUMP_THRESHOLD_MS,
        ) {
            if let Some(ref mut v) = vad {
                v.reset();
            }
            let (was_listening, was_preview) = {
                let mut guard = shared.lock().unwrap();
                let wl = guard.state == PipelineState::LISTENING;
                let wp = guard.state == PipelineState::PREVIEW;
                guard.cancel_active_or_preview(&turn_cfg);
                guard.state = PipelineState::IDLE;
                guard.arming_hysteresis.reset();
                guard.pre_roll.clear();
                guard.vad_pending.clear();
                guard.last_vision_update = None;
                guard.last_audio_callback_at = None;
                (wl, wp)
            };
            let msg = format!(
                "Canceled active dictation after {:.1}s system sleep or lock",
                jump_ms as f64 / 1000.0
            );
            if was_preview {
                let _ = app_handle.emit("preview-canceled", msg.clone());
                let _ = app_handle.emit("pipeline-error", msg);
            } else if was_listening {
                let _ = app_handle.emit("utterance-canceled", msg.clone());
                let _ = app_handle.emit("pipeline-error", msg);
            }
        }
        last_loop_wall_ms = now_wall_ms;

        if last_health_check.elapsed() >= Duration::from_millis(2000) {
            last_health_check = Instant::now();
            let (stream_faulted, ms_since_cb) = {
                let guard = shared.lock().unwrap();
                (
                    guard.stream_faulted,
                    guard
                        .last_audio_callback_at
                        .map(|t| t.elapsed().as_millis() as u64),
                )
            };
            let available = list_input_devices();
            let action = evaluate_audio_stream_health(
                active_stream.is_some(),
                &active_device_name,
                cfg.preferred_device.as_deref(),
                &available,
                stream_faulted,
                ms_since_cb,
            );
            match action {
                AudioStreamRecoveryAction::Healthy => {}
                AudioStreamRecoveryAction::ReconnectAfterFaultOrStall { reason } => {
                    let prev_device = active_device_name.clone();
                    drop(active_stream.take());
                    {
                        let mut guard = shared.lock().unwrap();
                        guard.stream_faulted = false;
                        guard.last_audio_callback_at = None;
                        if guard.state == PipelineState::LISTENING {
                            guard.cancel_active_or_preview(&turn_cfg);
                            let _ = app_handle.emit("utterance-canceled", reason.clone());
                        }
                    }
                    match build_cpal_stream(
                        cfg.preferred_device.as_deref(),
                        Arc::clone(&shared),
                        app_handle.clone(),
                    ) {
                        Ok((stream, name)) => {
                            active_stream = Some(stream);
                            active_device_name = name.clone();
                            let _ = app_handle.emit("audio-devices-changed", &available);
                            if prev_device != name {
                                let _ = app_handle.emit(
                                    "pipeline-error",
                                    format!(
                                        "Microphone '{}' disconnected; switched to '{}'",
                                        prev_device, name
                                    ),
                                );
                            }
                        }
                        Err(err) => {
                            active_device_name = "Unavailable".to_string();
                            let _ = app_handle.emit("pipeline-error", err);
                        }
                    }
                }
                AudioStreamRecoveryAction::SwitchBackToPreferred { preferred } => {
                    drop(active_stream.take());
                    match build_cpal_stream(
                        Some(preferred.as_str()),
                        Arc::clone(&shared),
                        app_handle.clone(),
                    ) {
                        Ok((stream, name)) => {
                            active_stream = Some(stream);
                            active_device_name = name;
                            let _ = app_handle.emit("audio-devices-changed", &available);
                        }
                        Err(err) => {
                            let _ = app_handle.emit("pipeline-error", err);
                        }
                    }
                }
            }
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                AudioCommand::StartListening(source) => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let mut guard = shared.lock().unwrap();
                    let is_interactive_setup =
                        source == "practice" || source.starts_with("voice_enroll:");
                    if is_interactive_setup && guard.state == PipelineState::PREVIEW {
                        guard.pending_preview.take();
                        guard.cancel_active_or_preview(&turn_cfg);
                    }
                    if guard.state == PipelineState::IDLE || guard.state == PipelineState::ARMED {
                        let seed = if source == "hands_free_multimodal" {
                            guard.consecutive_vad_frames.min(2)
                        } else if source == WAKE_WORD_TRIGGER {
                            guard.wake_consecutive_frames.min(WAKE_START_MIN_VAD_FRAMES)
                        } else {
                            0
                        };
                        guard.begin_listening(&source, seed);
                        if !source.starts_with("voice_enroll:") {
                            if let Some(ref vp) = cfg.voice_profile {
                                guard.peak_speech_rms =
                                    guard.peak_speech_rms.max(vp.typical_rms * 0.70);
                            }
                        }
                    } else if guard.is_unconfirmed_wake_candidate() && source != WAKE_WORD_TRIGGER {
                        guard.upgrade_wake_candidate_to(&source);
                        if !source.starts_with("voice_enroll:") {
                            if let Some(ref vp) = cfg.voice_profile {
                                guard.peak_speech_rms =
                                    guard.peak_speech_rms.max(vp.typical_rms * 0.70);
                            }
                        }
                    } else if is_interactive_setup && guard.state != PipelineState::LISTENING {
                        let _ = app_handle.emit(
                            "pipeline-error",
                            "Finish the current transcription first, then try again.",
                        );
                    }
                }
                AudioCommand::CancelListening => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let mut guard = shared.lock().unwrap();
                    guard.cancel_active_or_preview(&turn_cfg);
                    let _ = app_handle.emit("utterance-canceled", "Canceled");
                }
                AudioCommand::DropWakeCandidate(started_at_ms) => {
                    let mut guard = shared.lock().unwrap();
                    if guard.is_unconfirmed_wake_candidate()
                        && guard.listening_started_at_ms == started_at_ms
                    {
                        if let Some(ref mut v) = vad {
                            v.reset();
                        }
                        let now_ms = guard.now_ms();
                        let candidate_rms = guard.peak_speech_rms.max(guard.latest_rms);
                        guard.cancel_active_or_preview(&turn_cfg);
                        wake_log!("[saywhen wake] no wake word at the start: dropped");
                        guard.wake_needs_quiet = true;
                        guard.wake_dropped_at_ms = Some(now_ms);
                        guard.wake_floor_rms = candidate_rms;
                        guard.wake_check_in_flight = false;
                    }
                }
                AudioCommand::CancelPreview(reason) => {
                    let canceled_item = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state == PipelineState::PREVIEW {
                            let item = guard.pending_preview.take();
                            guard.cancel_active_or_preview(&turn_cfg);
                            item
                        } else {
                            None
                        }
                    };
                    if let Some(mut item) = canceled_item {
                        let metrics_rec = UtteranceMetricsRecord {
                            utterance_id: item.utterance_id,
                            timestamp_ms: unix_timestamp_ms(),
                            trigger_source: item.trigger_source,
                            lip_to_listening_ms: item.lip_to_listening_ms,
                            utterance_length_ms: item.transcription.audio_duration_ms,
                            speech_frames: item.speech_frames,
                            speech_duration_ms: (item.speech_frames as u64) * VAD_FRAME_MS,
                            peak_vad: item.transcription.peak_vad_prob,
                            end_reason: item.end_reason.as_str().to_string(),
                            post_roll_ms: item.post_roll_ms,
                            transcript_length: item.cleanup.cleaned_text.chars().count(),
                            discarded_reason: None,
                            outcome: "canceled".to_string(),
                            flag_false_trigger_clicks: 0,
                        };
                        persist_metrics_and_optional_tuning_wav(
                            &app_handle,
                            &cfg,
                            &metrics_rec,
                            item.tuning_capture.take(),
                        );
                        let _ = app_handle.emit("preview-canceled", reason);
                    }
                }
                AudioCommand::ResumeFromPreviewOnSpeech => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let resumed = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state == PipelineState::PREVIEW {
                            let prev = guard.pending_preview.take();
                            let source = prev
                                .as_ref()
                                .map(|p| p.trigger_source.clone())
                                .unwrap_or_else(|| "hands_free_multimodal".to_string());
                            let prefix = prev.and_then(|p| {
                                let t = p.transcription.text.trim().to_string();
                                if t.is_empty() {
                                    None
                                } else {
                                    Some(t)
                                }
                            });
                            let seed = guard.consecutive_vad_frames.min(2).max(1);
                            guard.resume_listening_with_continuation(
                                &source,
                                prefix,
                                seed,
                                cfg.voice_profile.as_ref(),
                            );
                            true
                        } else {
                            false
                        }
                    };
                    if resumed {
                        let _ = app_handle.emit("preview-resumed", ());
                    }
                }
                AudioCommand::ConfirmPreviewNow(override_text) => {
                    let taken = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state == PipelineState::PREVIEW {
                            let item = guard.pending_preview.take();
                            guard.continuation_pcm.clear();
                            guard.state = guard.resting_state(&turn_cfg);
                            item
                        } else {
                            None
                        }
                    };
                    if let Some(item) = taken {
                        finalize_preview_and_inject(&app_handle, item, override_text, &cfg);
                    }
                }
                AudioCommand::BeginPreview(item) => {
                    let evt = PreviewStartedEvent {
                        transcription: item.transcription.clone(),
                        cleanup: item.cleanup.clone(),
                        target_policy: item.target_policy.clone(),
                        countdown_ms: item.countdown_ms,
                        trigger_source: item.trigger_source.clone(),
                        send_mode: item.send_mode.as_str().to_string(),
                        waits_for_nod: item.waits_for_nod,
                    };
                    let resumed_immediately = {
                        let mut guard = shared.lock().unwrap();
                        let now_ms = guard.now_ms();
                        let spoke_during_tx = cfg.cancel_on_speech
                            && guard.consecutive_vad_frames >= 2
                            && guard
                                .last_vad_voice_at_ms
                                .map(|t| now_ms.saturating_sub(t) <= 160)
                                .unwrap_or(false)
                            && (!guard.is_vision_fresh()
                                || guard.vision.lips_moving
                                || !guard.vision.mouth_closed);
                        if spoke_during_tx {
                            let source = item.trigger_source.clone();
                            let t = item.transcription.text.trim().to_string();
                            let prefix = if t.is_empty() { None } else { Some(t) };
                            let seed = guard.consecutive_vad_frames.min(2).max(1);
                            guard.resume_listening_with_continuation(
                                &source,
                                prefix,
                                seed,
                                cfg.voice_profile.as_ref(),
                            );
                            true
                        } else {
                            guard.state = PipelineState::PREVIEW;
                            // Fresh count so speech that ended the turn is not read as "cancel".
                            guard.consecutive_vad_frames = 0;
                            guard.pending_preview = Some(item);
                            false
                        }
                    };
                    if resumed_immediately {
                        let _ = app_handle.emit("preview-resumed", ());
                    } else {
                        let _ = app_handle.emit("preview-started", evt);
                    }
                }
                AudioCommand::ToggleListening(source) => {
                    let (current, unconfirmed_wake) = {
                        let guard = shared.lock().unwrap();
                        (guard.state, guard.is_unconfirmed_wake_candidate())
                    };
                    match current {
                        PipelineState::IDLE | PipelineState::ARMED => {
                            let _ = cmd_tx_self.send(AudioCommand::StartListening(source));
                        }
                        PipelineState::LISTENING if unconfirmed_wake => {
                            let _ = cmd_tx_self.send(AudioCommand::StartListening(source));
                        }
                        PipelineState::LISTENING => {
                            let _ = cmd_tx_self
                                .send(AudioCommand::StopAndTranscribe(Some(EndReason::Hotkey)));
                        }
                        PipelineState::PREVIEW => {
                            let _ = cmd_tx_self.send(AudioCommand::ConfirmPreviewNow(None));
                        }
                        PipelineState::TRANSCRIBING => {}
                    }
                }
                AudioCommand::UpdateVision(vision_payload) => {
                    let mut guard = shared.lock().unwrap();
                    if !guard.camera_enabled {
                        // Drop frames that were in flight when the camera gate was paused.
                        continue;
                    }
                    let now_ms = guard.now_ms();
                    let gesture = vision_payload.gesture.clone();
                    guard.vision = vision_payload.clone();
                    guard.last_vision_update = Some(Instant::now());

                    if cfg.save_tuning_audio_enabled {
                        guard.vision_trace.push_back((now_ms, vision_payload));
                        let cutoff_ms = now_ms.saturating_sub(65_000);
                        while guard
                            .vision_trace
                            .front()
                            .map(|(t, _)| *t < cutoff_ms)
                            .unwrap_or(false)
                        {
                            guard.vision_trace.pop_front();
                        }
                    } else if !guard.vision_trace.is_empty() {
                        guard.vision_trace.clear();
                    }

                    match guard.state {
                        PipelineState::IDLE | PipelineState::ARMED => {
                            let currently_armed = guard.state == PipelineState::ARMED;
                            let vision_snap = guard.vision.clone();
                            let should_be_armed = evaluate_arming_hysteresis(
                                &mut guard.arming_hysteresis,
                                currently_armed,
                                true,
                                &vision_snap,
                                &turn_cfg,
                                now_ms,
                            );
                            guard.state = if should_be_armed {
                                PipelineState::ARMED
                            } else {
                                PipelineState::IDLE
                            };
                        }
                        PipelineState::LISTENING => {
                            if guard.is_unconfirmed_wake_candidate() {
                                let vision_snap = guard.vision.clone();
                                let currently_armed =
                                    is_within_arm_exit_cone(&vision_snap, &turn_cfg);
                                let _ = evaluate_arming_hysteresis(
                                    &mut guard.arming_hysteresis,
                                    currently_armed,
                                    true,
                                    &vision_snap,
                                    &turn_cfg,
                                    now_ms,
                                );
                            }
                            if guard.vision.lips_moving {
                                guard.last_lip_motion_in_turn_ms = Some(now_ms);
                            }
                            if gesture == "shake" {
                                guard.cancel_active_or_preview(&turn_cfg);
                                let _ = app_handle.emit(
                                    "utterance-canceled",
                                    "Head shake gesture canceled utterance",
                                );
                            } else if gesture == "nod" {
                                if cfg.post_roll_ms > 0 && guard.post_roll_started_at_ms.is_none() {
                                    guard.post_roll_started_at_ms = Some(now_ms);
                                    guard.pending_end_reason = Some(EndReason::Nod);
                                } else if cfg.post_roll_ms == 0 {
                                    let _ = cmd_tx_self.send(AudioCommand::StopAndTranscribe(
                                        Some(EndReason::Nod),
                                    ));
                                }
                            }
                        }
                        PipelineState::PREVIEW => {
                            if gesture == "shake" {
                                let _ = cmd_tx_self.send(AudioCommand::CancelPreview(
                                    "Head shake canceled preview".to_string(),
                                ));
                            } else if gesture == "nod" {
                                let _ = cmd_tx_self.send(AudioCommand::ConfirmPreviewNow(None));
                            } else if cfg.cancel_on_look_away
                                && !is_within_arm_exit_cone(&guard.vision, &turn_cfg)
                            {
                                let _ = cmd_tx_self.send(AudioCommand::CancelPreview(
                                    "Looked away during preview window".to_string(),
                                ));
                            }
                        }
                        PipelineState::TRANSCRIBING => {}
                    }
                }
                AudioCommand::SetCameraGateEnabled(enabled) => {
                    let mut guard = shared.lock().unwrap();
                    guard.camera_enabled = enabled;
                    if !enabled {
                        guard.vision = VisionTelemetryPayload::default();
                        guard.last_vision_update = None;
                        guard.arming_hysteresis.reset();
                        if guard.state == PipelineState::ARMED {
                            guard.state = PipelineState::IDLE;
                        }
                    }
                }
                AudioCommand::SwitchDevice(pref) => {
                    drop(active_stream.take());
                    match build_cpal_stream(
                        pref.as_deref(),
                        Arc::clone(&shared),
                        app_handle.clone(),
                    ) {
                        Ok((stream, name)) => {
                            active_stream = Some(stream);
                            active_device_name = name;
                        }
                        Err(err) => {
                            active_device_name = "Error".to_string();
                            let _ = app_handle.emit("pipeline-error", err);
                        }
                    }
                }
                AudioCommand::StopAndTranscribe(explicit_reason) => {
                    let captured = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state != PipelineState::LISTENING {
                            continue;
                        }
                        let reason = explicit_reason
                            .or(guard.pending_end_reason)
                            .unwrap_or(EndReason::ManualStop);
                        let post_roll_ms = match reason {
                            EndReason::Silence | EndReason::Nod => cfg.post_roll_ms,
                            _ => 0,
                        };
                        guard.take_utterance_for_transcription(
                            reason,
                            post_roll_ms,
                            cfg.save_tuning_audio_enabled,
                        )
                    };

                    if let Some(ref mut v) = vad {
                        v.reset();
                    }

                    // Task 3: Minimum speech evidence check before invoking Whisper.
                    // Skip discarding when `carried_transcript_prefix` is present so a brief
                    // sound that resumed `PREVIEW` restores the previous transcript rather than
                    // throwing it away.
                    if captured.carried_transcript_prefix.is_none() {
                        if let Err(discard_msg) = has_minimum_speech_evidence(
                            captured.speech_frames,
                            &captured.trigger_source,
                            &turn_cfg,
                        ) {
                            {
                                let mut guard = shared.lock().unwrap();
                                guard.state = guard.resting_state(&turn_cfg);
                            }
                            let was_wake_candidate = captured.trigger_source == WAKE_WORD_TRIGGER;
                            let audio_duration_ms =
                                ((captured.samples.len() as f64 / 16000.0) * 1000.0) as u64;
                            let metrics_rec = UtteranceMetricsRecord {
                                utterance_id: captured.utterance_id,
                                timestamp_ms: unix_timestamp_ms(),
                                trigger_source: captured.trigger_source,
                                lip_to_listening_ms: captured.lip_to_listening_ms,
                                utterance_length_ms: audio_duration_ms,
                                speech_frames: captured.speech_frames,
                                speech_duration_ms: (captured.speech_frames as u64) * VAD_FRAME_MS,
                                peak_vad: captured.peak_vad,
                                end_reason: captured.end_reason.as_str().to_string(),
                                post_roll_ms: captured.post_roll_ms,
                                transcript_length: 0,
                                discarded_reason: Some("insufficient_speech_evidence".to_string()),
                                outcome: "discarded".to_string(),
                                flag_false_trigger_clicks: 0,
                            };
                            let tuning_capture = if cfg.save_tuning_audio_enabled {
                                Some((captured.samples, captured.vision_trace))
                            } else {
                                None
                            };
                            persist_metrics_and_optional_tuning_wav(
                                &app_handle,
                                &cfg,
                                &metrics_rec,
                                tuning_capture,
                            );
                            // A wake word candidate that was too short is room noise, not a mistake.
                            if was_wake_candidate {
                                wake_log!(
                                    "[saywhen wake] too little clear speech to check: dropped"
                                );
                            } else {
                                let _ = app_handle.emit("utterance-discarded", discard_msg);
                            }
                            continue;
                        }
                    }

                    let _ = app_handle.emit(
                        "audio-telemetry",
                        AudioTelemetryEvent {
                            state: "TRANSCRIBING".to_string(),
                            rms: 0.0,
                            vad_prob: 0.0,
                            is_speech: false,
                            pre_roll_ms: 500,
                            listening_duration_ms: 0,
                            preview_remaining_ms: 0,
                            silence_remaining_ms: 0,
                            silence_total_ms: 0,
                            post_roll_active: false,
                            device_name: active_device_name.clone(),
                            speech_detected_in_utterance: true,
                            vision_active: false,
                            vision_armed: false,
                            vision_lips_moving: false,
                            camera_enabled: shared.lock().unwrap().camera_enabled,
                            hud_enabled: cfg.hud_enabled,
                            save_tuning_audio_enabled: cfg.save_tuning_audio_enabled,
                            session_metrics_enabled: cfg.session_metrics_enabled,
                            partial_transcript: String::new(),
                            partial_ends_with_connective: false,
                            send_mode: cfg.send_mode.as_str().to_string(),
                            wake_pending: false,
                            wake_word_active: cfg.wake_word_enabled,
                            ui_palette: cfg.ui_palette.clone(),
                            mood_colors: cfg.mood_colors,
                        },
                    );

                    let transcriber_ref = Arc::clone(&transcriber);
                    let shared_after = Arc::clone(&shared);
                    let config_for_tx = Arc::clone(&config);
                    let settings_path_for_tx = settings_path.clone();
                    let cfg_snapshot = config.lock().unwrap().clone();
                    let llm_api_key_for_tx = llm_api_key.lock().unwrap().clone();
                    let turn_cfg_snapshot = cfg_snapshot.to_turn_taking_config();
                    let app_for_tx = app_handle.clone();
                    let tx_for_preview = cmd_tx_self.clone();

                    thread::spawn(move || {
                        let language_override = {
                            let front = detect_frontmost_app();
                            match_app_profile(
                                &cfg_snapshot.app_profiles,
                                &front.name,
                                front.bundle_id.as_deref(),
                                front.exe.as_deref(),
                            )
                            .and_then(|profile| profile.language.clone())
                        };
                        transcriber_ref.set_language(
                            language_override
                                .as_deref()
                                .unwrap_or(cfg_snapshot.transcription_language.as_str()),
                        );
                        if let Some(step_str) =
                            captured.trigger_source.strip_prefix("voice_enroll:")
                        {
                            let step: usize = step_str.parse().unwrap_or(0);
                            {
                                let mut guard = shared_after.lock().unwrap();
                                guard.state = guard.resting_state(&turn_cfg_snapshot);
                            }
                            let heard_text = if transcriber_ref.is_model_available() {
                                transcriber_ref
                                    .transcribe_pcm(&captured.samples, captured.peak_vad)
                                    .map(|t| t.text)
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            let wake_word = effective_wake_word(&cfg_snapshot.wake_word);
                            let learned_alias =
                                VOICE_ENROLL_COMMAND_SUFFIXES.get(step).and_then(|suffix| {
                                    extract_leading_wake_alias(&heard_text, wake_word, suffix)
                                });
                            let existing = if step == 0 {
                                None
                            } else {
                                cfg_snapshot.voice_profile.as_ref()
                            };
                            match merge_into_voice_profile(
                                existing,
                                &captured.samples,
                                learned_alias.as_deref(),
                            ) {
                                Ok(new_profile) => {
                                    let updated_cfg = {
                                        let mut cfg_guard = config_for_tx.lock().unwrap();
                                        cfg_guard.voice_profile = Some(new_profile.clone());
                                        cfg_guard.clone()
                                    };
                                    if let Err(err) =
                                        updated_cfg.save_to_path(&settings_path_for_tx)
                                    {
                                        eprintln!("saywhen: voice profile save warning: {err}");
                                    }
                                    let _ =
                                        app_for_tx.emit("dictation-config-updated", &updated_cfg);
                                    let _ = app_for_tx.emit(
                                        "voice-enroll-step",
                                        VoiceEnrollStepEvent {
                                            step,
                                            total_steps: VOICE_ENROLL_COMMAND_SUFFIXES.len(),
                                            heard_text,
                                            learned_alias,
                                            profile: new_profile,
                                        },
                                    );
                                }
                                Err(err) => {
                                    let _ = app_for_tx.emit("utterance-discarded", err);
                                }
                            }
                            return;
                        }

                        let tx_res =
                            transcriber_ref.transcribe_pcm(&captured.samples, captured.peak_vad);
                        match tx_res {
                            Ok(mut transcription) => {
                                if captured.trigger_source == WAKE_WORD_TRIGGER
                                    && captured.carried_transcript_prefix.is_none()
                                {
                                    let wake_word = effective_wake_word(&cfg_snapshot.wake_word);
                                    match strip_wake_word_with_profile(
                                        &transcription.text,
                                        wake_word,
                                        cfg_snapshot.voice_profile.as_ref(),
                                    ) {
                                        Some(rest) if !rest.trim().is_empty() => {
                                            wake_log!("[saywhen wake] wake word found: sending the rest to the check step");
                                            transcription.text = rest;
                                        }
                                        stripped => {
                                            {
                                                let mut guard = shared_after.lock().unwrap();
                                                let now_ms = guard.now_ms();
                                                guard.state =
                                                    guard.resting_state(&turn_cfg_snapshot);
                                                if stripped.is_none() {
                                                    guard.wake_needs_quiet = true;
                                                    guard.wake_dropped_at_ms = Some(now_ms);
                                                }
                                            }
                                            wake_log!(
                                                "[saywhen wake] full transcript: {}{}",
                                                if stripped.is_some() {
                                                    "only the wake word, nothing after it"
                                                } else {
                                                    "no wake word at the start, dropped"
                                                },
                                                dev_first_word_note(&transcription.text)
                                            );
                                            // Only the wake word on its own is worth telling
                                            // the user about; anything else was not for us.
                                            if stripped.is_some() {
                                                let _ = app_for_tx.emit(
                                                    "utterance-discarded",
                                                    "Heard the wake word but nothing after it",
                                                );
                                            }
                                            return;
                                        }
                                    }
                                }

                                if let Some(ref prefix) = captured.carried_transcript_prefix {
                                    let cont = transcription.text.trim().to_string();
                                    if is_preview_spoken_cancel_command(&cont) {
                                        transcription.text = "cancel".to_string();
                                        transcription.discarded_reason = None;
                                    } else if cont.is_empty() {
                                        transcription.text = prefix.clone();
                                        transcription.discarded_reason = None;
                                    } else {
                                        transcription.text =
                                            stitch_continued_transcript(prefix, &cont);
                                        transcription.discarded_reason = None;
                                    }
                                }

                                let delivery_app = detect_frontmost_app();
                                let profile = match_app_profile(
                                    &cfg_snapshot.app_profiles,
                                    &delivery_app.name,
                                    delivery_app.bundle_id.as_deref(),
                                    delivery_app.exe.as_deref(),
                                );
                                let vocab = merge_vocab(
                                    &cfg_snapshot.custom_vocab,
                                    profile.map(|item| item.extra_vocab.as_str()).unwrap_or(""),
                                );
                                let resolved_send_mode = profile
                                    .and_then(|item| item.send_mode)
                                    .unwrap_or(cfg_snapshot.send_mode);
                                let mut cleanup = run_local_developer_cleanup_with_vocab(
                                    &transcription.text,
                                    cfg_snapshot.cleanup_enabled,
                                    &vocab,
                                );

                                if !cleanup.canceled_by_voice {
                                    if let Some(command) =
                                        parse_voice_command(&cleanup.cleaned_text)
                                    {
                                        let updated = {
                                            let mut guard = config_for_tx.lock().unwrap();
                                            match command {
                                                VoiceCommand::PauseListening => {
                                                    guard.hands_free_enabled = false;
                                                }
                                                VoiceCommand::ResumeListening => {
                                                    guard.hands_free_enabled = true;
                                                }
                                                VoiceCommand::SetSendMode(mode) => {
                                                    guard.send_mode = mode;
                                                }
                                                VoiceCommand::SetVisionPower(mode) => {
                                                    guard.vision_power_mode = mode;
                                                    guard.adaptive_fps_enabled =
                                                        mode != VisionPowerMode::Full;
                                                }
                                            }
                                            let updated = guard.clone();
                                            let _ = updated.save_to_path(&settings_path_for_tx);
                                            updated
                                        };
                                        {
                                            let mut guard = shared_after.lock().unwrap();
                                            guard.state = guard.resting_state(&turn_cfg_snapshot);
                                        }
                                        let _ = app_for_tx.emit(
                                            "voice-command",
                                            VoiceCommandEvent {
                                                message: describe_voice_command(command),
                                            },
                                        );
                                        let _ =
                                            app_for_tx.emit("dictation-config-updated", &updated);
                                        return;
                                    }
                                }

                                if cleanup.canceled_by_voice {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state = guard.resting_state(&turn_cfg_snapshot);
                                    }
                                    let metrics_rec = UtteranceMetricsRecord {
                                        utterance_id: captured.utterance_id,
                                        timestamp_ms: unix_timestamp_ms(),
                                        trigger_source: captured.trigger_source,
                                        lip_to_listening_ms: captured.lip_to_listening_ms,
                                        utterance_length_ms: transcription.audio_duration_ms,
                                        speech_frames: captured.speech_frames,
                                        speech_duration_ms: (captured.speech_frames as u64)
                                            * VAD_FRAME_MS,
                                        peak_vad: transcription.peak_vad_prob,
                                        end_reason: EndReason::Canceled.as_str().to_string(),
                                        post_roll_ms: captured.post_roll_ms,
                                        transcript_length: 0,
                                        discarded_reason: None,
                                        outcome: "canceled".to_string(),
                                        flag_false_trigger_clicks: 0,
                                    };
                                    let tuning_capture = if cfg_snapshot.save_tuning_audio_enabled {
                                        Some((captured.samples, captured.vision_trace))
                                    } else {
                                        None
                                    };
                                    persist_metrics_and_optional_tuning_wav(
                                        &app_for_tx,
                                        &cfg_snapshot,
                                        &metrics_rec,
                                        tuning_capture,
                                    );
                                    let _ = app_for_tx.emit(
                                        "preview-canceled",
                                        "Spoken cancel command aborted utterance",
                                    );
                                    return;
                                }

                                if cleanup.cleaned_text.is_empty() {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state = guard.resting_state(&turn_cfg_snapshot);
                                    }
                                    let reason =
                                        transcription.discarded_reason.clone().unwrap_or_else(
                                            || "No speech recognized: nothing was sent".to_string(),
                                        );
                                    let metrics_rec = UtteranceMetricsRecord {
                                        utterance_id: captured.utterance_id,
                                        timestamp_ms: unix_timestamp_ms(),
                                        trigger_source: captured.trigger_source,
                                        lip_to_listening_ms: captured.lip_to_listening_ms,
                                        utterance_length_ms: transcription.audio_duration_ms,
                                        speech_frames: captured.speech_frames,
                                        speech_duration_ms: (captured.speech_frames as u64)
                                            * VAD_FRAME_MS,
                                        peak_vad: transcription.peak_vad_prob,
                                        end_reason: captured.end_reason.as_str().to_string(),
                                        post_roll_ms: captured.post_roll_ms,
                                        transcript_length: 0,
                                        discarded_reason: Some(
                                            sanitize_discarded_reason_for_metrics(&reason),
                                        ),
                                        outcome: "discarded".to_string(),
                                        flag_false_trigger_clicks: 0,
                                    };
                                    let tuning_capture = if cfg_snapshot.save_tuning_audio_enabled {
                                        Some((captured.samples, captured.vision_trace))
                                    } else {
                                        None
                                    };
                                    persist_metrics_and_optional_tuning_wav(
                                        &app_for_tx,
                                        &cfg_snapshot,
                                        &metrics_rec,
                                        tuning_capture,
                                    );
                                    let _ = app_for_tx.emit("utterance-discarded", reason);
                                    return;
                                }

                                // Onboarding practice speaks for the review only. It never
                                // inspects a target app, previews, injects or writes history.
                                if captured.trigger_source == "practice" {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state = guard.resting_state(&turn_cfg_snapshot);
                                    }
                                    let _ = app_for_tx.emit(
                                        "practice-complete",
                                        PracticeCompleteEvent {
                                            transcription,
                                            cleanup,
                                        },
                                    );
                                    return;
                                }

                                let expanded =
                                    apply_snippets(&cleanup.cleaned_text, &cfg_snapshot.snippets);
                                if expanded != cleanup.cleaned_text {
                                    cleanup.cleaned_text = expanded;
                                    cleanup.modified = true;
                                    cleanup.is_destructive_command =
                                        check_destructive_command(&cleanup.cleaned_text);
                                }

                                if cfg_snapshot.llm_cleanup_enabled {
                                    let llm_cfg = LlmCleanupConfig {
                                        enabled: true,
                                        endpoint_url: cfg_snapshot.llm_endpoint_url.clone(),
                                        model: cfg_snapshot.llm_model.clone(),
                                        api_key: llm_api_key_for_tx.clone(),
                                        timeout_ms: 1200,
                                    };
                                    match run_opt_in_llm_cleanup(
                                        &cleanup.cleaned_text,
                                        &vocab,
                                        &llm_cfg,
                                    ) {
                                        Ok(llm_out) => {
                                            cleanup.modified = llm_out != cleanup.raw_text;
                                            cleanup.cleaned_text = llm_out;
                                            cleanup.is_destructive_command =
                                                check_destructive_command(&cleanup.cleaned_text);
                                            cleanup.mode_used = match classify_llm_endpoint_privacy(
                                                &cfg_snapshot.llm_endpoint_url,
                                            ) {
                                                "local_loopback" => "local_ollama".to_string(),
                                                _ => "byok_llm".to_string(),
                                            };
                                        }
                                        Err(err) => {
                                            let _ = app_for_tx.emit("pipeline-error", err);
                                        }
                                    }
                                }

                                let front_app = detect_frontmost_app();
                                let wants_enter = resolved_send_mode.wants_enter(
                                    cfg_snapshot.auto_press_enter,
                                    transcription.end_phrase_triggered,
                                ) && !cleanup.force_paste_only;
                                let mut target_policy = evaluate_app_injection_policy(
                                    &front_app,
                                    wants_enter,
                                    cfg_snapshot.per_app_rules_enabled,
                                    cleanup.is_destructive_command,
                                    &cfg_snapshot.auto_send_allowlist,
                                );
                                if cleanup.force_paste_only {
                                    target_policy.policy = "paste_only".to_string();
                                    target_policy.will_press_enter = false;
                                    target_policy.reason = format!(
                                        "Voice override ('paste only'): pasting into '{}' without pressing Enter",
                                        target_policy.app_name
                                    );
                                } else if target_policy.will_press_enter
                                    && captured.vision_fresh
                                    && !cfg_snapshot.side_camera_mode
                                {
                                    // Head yaw only maps to a display when the camera faces the user.
                                    let adjusted_yaw = captured.latest_yaw_deg
                                        - cfg_snapshot.calibrated_yaw_offset_deg;
                                    if let Some(gaze_reason) =
                                        evaluate_gaze_window_alignment(&front_app, adjusted_yaw)
                                    {
                                        target_policy.policy = "paste_only".to_string();
                                        target_policy.will_press_enter = false;
                                        target_policy.reason = gaze_reason;
                                    }
                                }

                                let tuning_capture = if cfg_snapshot.save_tuning_audio_enabled {
                                    Some((captured.samples, captured.vision_trace))
                                } else {
                                    None
                                };

                                let waits_for_nod =
                                    resolved_send_mode.preview_waits_for_nod(captured.end_reason);
                                let trailing_connective =
                                    ends_with_connective(&cleanup.cleaned_text);
                                let item = PendingPreviewItem {
                                    utterance_id: captured.utterance_id,
                                    started_at: Instant::now(),
                                    countdown_ms: if waits_for_nod {
                                        NOD_WAIT_TIMEOUT_MS
                                    } else if trailing_connective {
                                        cfg_snapshot.preview_countdown_ms.max(3000)
                                    } else {
                                        cfg_snapshot.preview_countdown_ms
                                    },
                                    send_mode: resolved_send_mode,
                                    waits_for_nod,
                                    transcription,
                                    cleanup,
                                    target_policy,
                                    trigger_source: captured.trigger_source,
                                    lip_to_listening_ms: captured.lip_to_listening_ms,
                                    speech_frames: captured.speech_frames,
                                    end_reason: captured.end_reason,
                                    post_roll_ms: captured.post_roll_ms,
                                    tuning_capture,
                                };

                                // Nod mode always needs the preview: it is where the nod is read.
                                if (cfg_snapshot.preview_enabled || waits_for_nod)
                                    && cfg_snapshot.auto_inject
                                {
                                    let _ = tx_for_preview.send(AudioCommand::BeginPreview(item));
                                } else {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state = guard.resting_state(&turn_cfg_snapshot);
                                    }
                                    finalize_preview_and_inject(
                                        &app_for_tx,
                                        item,
                                        None,
                                        &cfg_snapshot,
                                    );
                                }
                            }
                            Err(err) => {
                                {
                                    let mut guard = shared_after.lock().unwrap();
                                    guard.state = guard.resting_state(&turn_cfg_snapshot);
                                }
                                let _ = app_for_tx.emit("pipeline-error", err);
                            }
                        }
                    });
                }
            }
        }

        // Check if active PREVIEW countdown has completed
        let preview_expired = {
            let guard = shared.lock().unwrap();
            if guard.state == PipelineState::PREVIEW {
                guard.pending_preview.as_ref().and_then(|item| {
                    preview_deadline(
                        item.waits_for_nod,
                        item.started_at.elapsed().as_millis() as u64,
                        item.countdown_ms,
                    )
                })
            } else {
                None
            }
        };

        match preview_expired {
            Some(PreviewDeadline::Send) => {
                let _ = cmd_tx_self.send(AudioCommand::ConfirmPreviewNow(None));
            }
            Some(PreviewDeadline::GiveUp) => {
                let _ = cmd_tx_self.send(AudioCommand::CancelPreview(format!(
                    "No nod within {} seconds, so nothing was sent",
                    NOD_WAIT_TIMEOUT_MS / 1000
                )));
            }
            None => {}
        }
        let mut cancel_preview_for_speech = false;

        // Process pending 512-sample chunks through Silero VAD
        let mut should_auto_stop: Option<EndReason> = None;
        let mut should_auto_start_listening = false;
        let mut should_wake_start_listening = false;
        // Checked once per tick: the model loads lazily, so `is_loaded` is false after launch.
        let wake_model_available = cfg.wake_word_enabled && transcriber.is_model_available();

        loop {
            let chunk: Option<[f32; VAD_CHUNK_SAMPLES]> = {
                let mut guard = shared.lock().unwrap();
                if guard.vad_pending.len() >= VAD_CHUNK_SAMPLES {
                    let mut arr = [0.0f32; VAD_CHUNK_SAMPLES];
                    for slot in &mut arr {
                        *slot = guard.vad_pending.pop_front().unwrap_or(0.0);
                    }
                    Some(arr)
                } else {
                    None
                }
            };

            let Some(chunk_samples) = chunk else {
                break;
            };

            let prob = if let Some(ref mut v) = vad {
                v.predict(chunk_samples.iter().copied())
            } else {
                0.0
            };
            let chunk_rms = (chunk_samples.iter().map(|s| s * s).sum::<f32>()
                / VAD_CHUNK_SAMPLES as f32)
                .sqrt();

            let mut guard = shared.lock().unwrap();
            let now_ms = guard.now_ms();
            guard.latest_vad_prob = prob;

            let prev_voice_ms = guard.last_vad_voice_at_ms;
            if prob >= cfg.vad_threshold {
                guard.consecutive_vad_frames += 1;
                guard.last_vad_voice_at_ms = Some(now_ms);
            } else {
                guard.consecutive_vad_frames = 0;
            }
            let wake_thr = wake_vad_threshold(cfg.vad_threshold);
            if prob >= wake_thr {
                guard.wake_consecutive_frames += 1;
            } else {
                guard.wake_consecutive_frames = 0;
            }

            if guard.state == PipelineState::PREVIEW && !cancel_preview_for_speech {
                let elapsed = guard
                    .pending_preview
                    .as_ref()
                    .map(|p| p.started_at.elapsed().as_millis() as u64)
                    .unwrap_or(0);
                let user_resumed_speaking = cfg.cancel_on_speech
                    && elapsed >= 120
                    && guard.consecutive_vad_frames >= 3
                    && (!guard.is_vision_fresh()
                        || guard.vision.lips_moving
                        || !guard.vision.mouth_closed);
                if user_resumed_speaking
                    || should_cancel_preview_on_speech(
                        cfg.cancel_on_speech,
                        elapsed,
                        guard.consecutive_vad_frames,
                        guard.is_vision_fresh(),
                        guard.vision.lips_moving,
                    )
                {
                    cancel_preview_for_speech = true;
                }
            }

            if guard.state == PipelineState::ARMED
                && (!cfg.hands_free_enabled || !guard.is_vision_fresh())
            {
                guard.arming_hysteresis.reset();
                guard.state = PipelineState::IDLE;
            }

            // Stage 3 Multimodal Auto-Start via pure decision function.
            // An unconfirmed wake candidate started by background audio (such as a song on YouTube)
            // never blocks the camera gate: if the user faces the screen and moves their lips,
            // auto-start upgrades the turn to `hands_free_multimodal` with a clean 500 ms pre-roll.
            let armed_for_gate = guard.state == PipelineState::ARMED
                || (guard.is_unconfirmed_wake_candidate()
                    && guard.is_vision_fresh()
                    && is_within_arm_exit_cone(&guard.vision, &turn_cfg));
            let gate_inputs = GateInputs {
                is_armed: armed_for_gate,
                hands_free_enabled: cfg.hands_free_enabled,
                vision_fresh: guard.is_vision_fresh(),
                face_present: guard.vision.face_present,
                within_exit_cone: is_within_arm_exit_cone(&guard.vision, &turn_cfg),
                lips_moving: guard.vision.lips_moving,
                vad_prob: prob,
                recent_voice_ms: prev_voice_ms.map(|t| now_ms.saturating_sub(t)),
                consecutive_vad_frames: guard.consecutive_vad_frames,
            };
            if should_auto_start(&gate_inputs, &turn_cfg) {
                should_auto_start_listening = true;
            }

            // Wake word fallback: speech while not armed becomes a candidate that is checked
            // for the wake word and dropped silently when it does not start with it.
            let quiet_gap_met = prob < cfg.vad_threshold
                && guard
                    .last_vad_voice_at_ms
                    .map(|t| now_ms.saturating_sub(t) >= WAKE_REARM_QUIET_MS)
                    .unwrap_or(true);
            if quiet_gap_met {
                guard.wake_floor_rms = 0.0;
                guard.wake_dropped_at_ms = None;
            }
            if guard.wake_needs_quiet
                && should_clear_wake_quiet_wait(
                    now_ms,
                    guard.last_vad_voice_at_ms,
                    guard.wake_dropped_at_ms,
                    prob,
                    cfg.vad_threshold,
                    chunk_rms,
                    guard.wake_floor_rms,
                )
            {
                guard.wake_needs_quiet = false;
            }
            // ARMED counts too: from a side angle the camera often says "facing" while lip
            // motion is not detected, and then the normal camera path never starts.
            let wake_can_start_here =
                matches!(guard.state, PipelineState::IDLE | PipelineState::ARMED);
            let wake_watching =
                cfg.wake_word_enabled && !should_auto_start_listening && wake_can_start_here;
            let above_recent_drop_floor = guard
                .wake_dropped_at_ms
                .map(|t| now_ms.saturating_sub(t) >= 10_000)
                .unwrap_or(true)
                || guard.wake_floor_rms <= 0.0
                || chunk_rms >= (guard.wake_floor_rms * 1.25).max(0.012)
                || prob >= 0.68;
            if wake_watching
                && !guard.wake_needs_quiet
                && wake_model_available
                && prob >= wake_thr
                && above_recent_drop_floor
                && guard.wake_consecutive_frames >= WAKE_START_MIN_VAD_FRAMES
            {
                should_wake_start_listening = true;
                guard.wake_burst_frames = 0;
                guard.wake_burst_peak = 0.0;
            } else if wake_watching
                && !wake_model_available
                && guard.wake_consecutive_frames == WAKE_START_MIN_VAD_FRAMES
            {
                wake_log!(
                    "[saywhen wake] voice heard but not started: the speech model is not downloaded"
                );
            }

            // Diagnostics: a burst of sound that never looked enough like a voice to start.
            if wake_watching && !should_wake_start_listening {
                if prob >= WAKE_DIAG_SOUND_LEVEL {
                    guard.wake_burst_frames += 1;
                    guard.wake_burst_peak = guard.wake_burst_peak.max(prob);
                } else if guard.wake_burst_frames > 0 {
                    if guard.wake_burst_frames >= 5 && guard.wake_burst_peak < wake_thr {
                        wake_log!(
                            "[saywhen wake] sound heard for {} ms but the voice detector was unsure (peak {:.2}, needs {:.2}): too quiet or too far from the microphone",
                            guard.wake_burst_frames as u64 * VAD_FRAME_MS,
                            guard.wake_burst_peak,
                            wake_thr
                        );
                    }
                    guard.wake_burst_frames = 0;
                    guard.wake_burst_peak = 0.0;
                }
            }

            if guard.state == PipelineState::LISTENING {
                if prob > guard.peak_vad_prob {
                    guard.peak_vad_prob = prob;
                }

                // Early wake word check on the first ~2 s so room chatter is dropped quickly.
                let paused = now_ms.saturating_sub(guard.last_speech_at_ms) >= 240
                    && guard.utterance_pcm.len() >= WAKE_CHECK_MIN_SAMPLES_ON_PAUSE;
                if guard.is_unconfirmed_wake_candidate()
                    && !guard.wake_check_in_flight
                    && !guard.wake_check_done
                    && (guard.utterance_pcm.len() >= WAKE_CHECK_MIN_SAMPLES || paused)
                {
                    guard.wake_check_in_flight = true;
                    guard.wake_check_done = true;
                    let head_len = guard.utterance_pcm.len().min(WAKE_CHECK_MAX_SAMPLES);
                    let head = guard.utterance_pcm[..head_len].to_vec();
                    let token = guard.listening_started_at_ms;
                    let peak_vad = guard.peak_vad_prob;
                    let wake_word = effective_wake_word(&cfg.wake_word).to_string();
                    let voice_profile_snap = cfg.voice_profile.clone();
                    let tx_clone = Arc::clone(&transcriber);
                    let shared_wake = Arc::clone(&shared);
                    let cmd_wake = cmd_tx_self.clone();
                    let app_wake = app_handle.clone();
                    thread::spawn(move || {
                        let heard = tx_clone
                            .transcribe_pcm_fast(&head, peak_vad)
                            .map(|t| t.text)
                            .unwrap_or_default();
                        let verdict = early_wake_verdict_with_profile(
                            &heard,
                            &wake_word,
                            voice_profile_snap.as_ref(),
                        );
                        let voice_matches = voice_profile_snap
                            .as_ref()
                            .map(|vp| matches_voice_profile(&head, vp))
                            .unwrap_or(true);
                        let words = word_count(&heard);
                        // Word count only: this may be someone else talking, so never log text.
                        wake_log!(
                            "[saywhen wake] early check on the first {:.1} s: {:?} ({} words heard, voice_match={}{})",
                            head.len() as f32 / TARGET_SAMPLE_RATE as f32,
                            verdict,
                            words,
                            voice_matches,
                            dev_first_word_note(&heard)
                        );
                        let mut g = shared_wake.lock().unwrap();
                        if g.listening_started_at_ms != token || !g.is_unconfirmed_wake_candidate()
                        {
                            return;
                        }
                        g.wake_check_in_flight = false;
                        match verdict {
                            EarlyWakeVerdict::Confirmed => {
                                g.wake_confirmed = true;
                                drop(g);
                                let _ = app_wake.emit("wake-word-heard", false);
                            }
                            EarlyWakeVerdict::AwaitingCommand => {
                                g.wake_confirmed = true;
                                g.wake_awaiting_command = true;
                                g.wake_confirmed_at_ms = g.now_ms();
                                drop(g);
                                // true: only the wake word so far, waiting for the command.
                                let _ = app_wake.emit("wake-word-heard", true);
                            }
                            // Pure instrumental music, noise over 1.5-1.8 s producing 0 words,
                            // or an unclear snippet whose voiceprint does not match the enrolled
                            // user is dropped right away instead of holding the turn for 4 s.
                            EarlyWakeVerdict::Unclear if words == 0 || !voice_matches => {
                                drop(g);
                                let _ = cmd_wake.send(AudioCommand::DropWakeCandidate(token));
                            }
                            // Let the full transcript decide when 1-2 words were heard.
                            EarlyWakeVerdict::Unclear => {}
                            EarlyWakeVerdict::NotForUs => {
                                drop(g);
                                let _ = cmd_wake.send(AudioCommand::DropWakeCandidate(token));
                            }
                        }
                    });
                }
                if guard.vision.lips_moving && guard.is_vision_fresh() {
                    guard.last_lip_motion_in_turn_ms = Some(now_ms);
                }

                // Wake word candidates count speech with the lower wake threshold, so a voice
                // aimed away from the microphone is not cut off or discarded as too short.
                let speech_thr = if guard.trigger_source == WAKE_WORD_TRIGGER {
                    wake_thr
                } else {
                    cfg.vad_threshold
                };
                let count_as_user_speech = prob >= speech_thr
                    && should_count_vad_frame_in_turn(
                        now_ms,
                        guard.is_vision_fresh(),
                        cfg.hands_free_enabled,
                        guard.vision.face_present,
                        guard.vision.lips_moving,
                        guard.vision.mouth_closed,
                        guard.last_lip_motion_in_turn_ms,
                        guard.speech_frames_in_utterance,
                        chunk_rms,
                        guard.peak_speech_rms,
                    );
                if count_as_user_speech {
                    guard.speech_frames_in_utterance += 1;
                    guard.last_speech_at_ms = now_ms;
                    guard.peak_speech_rms = guard.peak_speech_rms.max(chunk_rms);
                    guard.partial_ends_with_connective = false;
                    // If the user resumed speaking during a silence-triggered post-roll, abort the stop.
                    if guard.pending_end_reason == Some(EndReason::Silence) {
                        guard.post_roll_started_at_ms = None;
                        guard.pending_end_reason = None;
                    }
                } else if now_ms.saturating_sub(guard.last_speech_at_ms) >= 240
                    && guard.speech_frames_in_utterance >= 8
                    && !guard.partial_check_in_flight
                    && guard.partial_checked_speech_frames != guard.speech_frames_in_utterance
                    && transcriber.is_loaded()
                {
                    let tail_len = guard
                        .utterance_pcm
                        .len()
                        .min((TARGET_SAMPLE_RATE as usize) * 2);
                    if tail_len >= (TARGET_SAMPLE_RATE as usize) / 2 {
                        let start_idx = guard.utterance_pcm.len() - tail_len;
                        let tail_samples = guard.utterance_pcm[start_idx..].to_vec();
                        let peak_vad = guard.peak_vad_prob;
                        let checked_frames = guard.speech_frames_in_utterance;
                        guard.partial_check_in_flight = true;
                        guard.partial_checked_speech_frames = checked_frames;

                        let tx_clone = Arc::clone(&transcriber);
                        let shared_partial = Arc::clone(&shared);
                        thread::spawn(move || {
                            let res = tx_clone.transcribe_pcm_fast(&tail_samples, peak_vad);
                            if let Ok(mut g) = shared_partial.lock() {
                                g.partial_check_in_flight = false;
                                if g.state == PipelineState::LISTENING
                                    && g.speech_frames_in_utterance == checked_frames
                                {
                                    if let Ok(tx) = res {
                                        let text = tx.text.trim().to_string();
                                        if !text.is_empty() {
                                            g.partial_ends_with_connective =
                                                ends_with_connective(&text);
                                            g.partial_transcript = text;
                                        }
                                    }
                                }
                            }
                        });
                    }
                }

                if guard.wake_awaiting_command
                    && prob >= wake_thr
                    && now_ms > guard.wake_confirmed_at_ms
                {
                    guard.wake_awaiting_command = false;
                }
                let wake_hold = guard.trigger_source == WAKE_WORD_TRIGGER
                    && wake_holds_turn(
                        guard.wake_confirmed,
                        guard.wake_awaiting_command,
                        now_ms.saturating_sub(guard.listening_started_at_ms),
                        now_ms.saturating_sub(guard.last_speech_at_ms),
                    );

                if guard.post_roll_started_at_ms.is_none() && !wake_hold {
                    let turn_state = guard.build_turn_state(cfg.hands_free_enabled);
                    if let Some(reason) = end_of_turn(&turn_state, &turn_cfg, now_ms) {
                        if cfg.post_roll_ms > 0 && reason != EndReason::MaxLength {
                            guard.post_roll_started_at_ms = Some(now_ms);
                            guard.pending_end_reason = Some(reason);
                        } else {
                            should_auto_stop = Some(reason);
                        }
                    }
                }

                if let Some(pr_start) = guard.post_roll_started_at_ms {
                    if now_ms.saturating_sub(pr_start) >= cfg.post_roll_ms {
                        let reason = guard
                            .pending_end_reason
                            .take()
                            .unwrap_or(EndReason::Silence);
                        guard.post_roll_started_at_ms = None;
                        should_auto_stop = Some(reason);
                    }
                }
            }
        }

        if cancel_preview_for_speech {
            let _ = cmd_tx_self.send(AudioCommand::ResumeFromPreviewOnSpeech);
        }

        if should_auto_start_listening {
            let _ = cmd_tx_self.send(AudioCommand::StartListening(
                "hands_free_multimodal".to_string(),
            ));
        } else if should_wake_start_listening {
            let _ = cmd_tx_self.send(AudioCommand::StartListening(WAKE_WORD_TRIGGER.to_string()));
        }

        if let Some(reason) = should_auto_stop {
            let _ = cmd_tx_self.send(AudioCommand::StopAndTranscribe(Some(reason)));
        }

        if last_telemetry_emit.elapsed() >= Duration::from_millis(50) {
            last_telemetry_emit = Instant::now();
            let payload = {
                let guard = shared.lock().unwrap();
                let now_ms = guard.now_ms();
                let pre_roll_ms = ((guard.pre_roll.len() as f64 / TARGET_SAMPLE_RATE as f64)
                    * 1000.0)
                    .round() as u64;
                let listening_duration_ms = guard
                    .listening_started_at
                    .map(|t| t.elapsed().as_millis() as u64)
                    .unwrap_or(0);
                let preview_remaining_ms = guard
                    .pending_preview
                    .as_ref()
                    .map(|p| {
                        p.countdown_ms
                            .saturating_sub(p.started_at.elapsed().as_millis() as u64)
                    })
                    .unwrap_or(0);
                let (silence_remaining_ms, silence_total_ms) =
                    if guard.state == PipelineState::LISTENING {
                        let ts = guard.build_turn_state(cfg.hands_free_enabled);
                        silence_countdown_ms(&ts, &turn_cfg, now_ms)
                    } else {
                        (0, 0)
                    };
                let vision_fresh = guard.is_vision_fresh();
                let min_frames = (cfg.min_speech_ms / VAD_FRAME_MS).max(1) as usize;
                AudioTelemetryEvent {
                    state: guard.state.as_str().to_string(),
                    rms: guard.latest_rms,
                    vad_prob: guard.latest_vad_prob,
                    is_speech: guard.latest_vad_prob >= cfg.vad_threshold,
                    pre_roll_ms,
                    listening_duration_ms,
                    preview_remaining_ms,
                    silence_remaining_ms,
                    silence_total_ms,
                    post_roll_active: guard.post_roll_started_at_ms.is_some(),
                    device_name: active_device_name.clone(),
                    speech_detected_in_utterance: guard.speech_frames_in_utterance >= min_frames,
                    vision_active: vision_fresh,
                    vision_armed: vision_fresh && is_within_arm_exit_cone(&guard.vision, &turn_cfg),
                    vision_lips_moving: vision_fresh && guard.vision.lips_moving,
                    camera_enabled: guard.camera_enabled,
                    hud_enabled: cfg.hud_enabled,
                    save_tuning_audio_enabled: cfg.save_tuning_audio_enabled,
                    session_metrics_enabled: cfg.session_metrics_enabled,
                    partial_transcript: guard.partial_transcript.clone(),
                    partial_ends_with_connective: guard.partial_ends_with_connective,
                    send_mode: cfg.send_mode.as_str().to_string(),
                    wake_pending: guard.is_unconfirmed_wake_candidate(),
                    wake_word_active: cfg.wake_word_enabled,
                    ui_palette: cfg.ui_palette.clone(),
                    mood_colors: cfg.mood_colors,
                }
            };
            let _ = app_handle.emit("audio-telemetry", payload);
        }

        thread::sleep(Duration::from_millis(12));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_without_hud_field_still_load() {
        // A settings.local.json written before the HUD, injection reliability, and turn-taking
        // fields existed must still load, and `saveTuningAudioEnabled` must reset on startup.
        let mut legacy = serde_json::to_value(DictationConfig::default()).unwrap();
        let obj = legacy.as_object_mut().unwrap();
        obj.remove("hudEnabled");
        obj.remove("onTargetChanged");
        obj.remove("enterDelayMs");
        obj.remove("perAppEnterDelayMs");
        obj.remove("clipboardRestoreDelayMs");
        obj.remove("maxSilenceTimeoutMs");
        obj.remove("armYawEnterDeg");
        obj.remove("armYawExitDeg");
        obj.remove("armPitchEnterDeg");
        obj.remove("armPitchExitDeg");
        obj.remove("armDwellMs");
        obj.remove("disarmDwellMs");
        obj.remove("minSpeechMs");
        obj.remove("minSpeechHotkeyMs");
        obj.remove("postRollMs");
        obj.remove("sessionMetricsEnabled");
        obj.remove("saveTuningAudioEnabled");
        obj.remove("calibratedYawOffsetDeg");
        obj.remove("calibratedPitchOffsetDeg");
        obj.remove("calibratedRestingMar");
        obj.remove("adaptiveFpsEnabled");
        obj.remove("customVocab");
        obj.remove("historyPersistenceEnabled");
        legacy["vadThreshold"] = serde_json::json!(0.7);
        let mut parsed: DictationConfig = serde_json::from_value(legacy).unwrap();
        assert!(parsed.hud_enabled);
        assert!(parsed.custom_vocab.is_empty());
        assert!(parsed.history_persistence_enabled);
        assert_eq!(parsed.on_target_changed, TargetChangePolicy::Abort);
        assert_eq!(parsed.enter_delay_ms, DEFAULT_ENTER_DELAY_MS);
        assert!(parsed.per_app_enter_delay_ms.is_empty());
        assert_eq!(
            parsed.clipboard_restore_delay_ms,
            DEFAULT_CLIPBOARD_RESTORE_DELAY_MS
        );
        assert_eq!(
            parsed.max_silence_timeout_ms,
            DEFAULT_MAX_SILENCE_TIMEOUT_MS
        );
        assert_eq!(parsed.arm_yaw_enter_deg, DEFAULT_ARM_YAW_ENTER_DEG);
        assert_eq!(parsed.arm_yaw_exit_deg, DEFAULT_ARM_YAW_EXIT_DEG);
        assert_eq!(parsed.arm_dwell_ms, DEFAULT_ARM_DWELL_MS);
        assert_eq!(parsed.disarm_dwell_ms, DEFAULT_DISARM_DWELL_MS);
        assert_eq!(parsed.min_speech_ms, DEFAULT_MIN_SPEECH_MS);
        assert_eq!(parsed.min_speech_hotkey_ms, DEFAULT_MIN_SPEECH_HOTKEY_MS);
        assert_eq!(parsed.post_roll_ms, DEFAULT_POST_ROLL_MS);
        assert!(!parsed.session_metrics_enabled);
        assert!(!parsed.save_tuning_audio_enabled);
        assert_eq!(parsed.calibrated_yaw_offset_deg, 0.0);
        assert_eq!(parsed.calibrated_pitch_offset_deg, 0.0);
        assert!((parsed.calibrated_resting_mar - DEFAULT_CALIBRATED_RESTING_MAR).abs() < 1e-6);
        assert!(parsed.adaptive_fps_enabled);
        assert!((parsed.vad_threshold - 0.7).abs() < f32::EPSILON);

        // Even if someone manually edited settings.local.json with saveTuningAudioEnabled = true,
        // sanitize_after_parse forces it back to false on startup.
        parsed.save_tuning_audio_enabled = true;
        parsed.sanitize_after_parse();
        assert!(!parsed.save_tuning_audio_enabled);
    }

    #[test]
    fn settings_save_in_app_data_dir_and_prefer_it_over_legacy() {
        let root =
            std::env::temp_dir().join(format!("saywhen-settings-test-{}", std::process::id()));
        let app_data = root.join("appdata");
        let legacy = root.join("legacy").join("settings.local.json");
        let _ = std::fs::remove_dir_all(&root);

        let mut cfg = DictationConfig::default();
        cfg.vad_threshold = 0.42;
        cfg.hud_enabled = false;
        cfg.save_to_path(&legacy).unwrap();

        let loaded = DictationConfig::load_with_legacy(&app_data, &[legacy.clone()]);
        assert!((loaded.vad_threshold - 0.42).abs() < f32::EPSILON);
        assert!(!loaded.hud_enabled);
        let migrated = DictationConfig::settings_file_path(&app_data);
        assert!(migrated.is_file());

        let mut updated = loaded;
        updated.vad_threshold = 0.55;
        updated.save_to_path(&migrated).unwrap();

        let mut stale = DictationConfig::default();
        stale.vad_threshold = 0.1;
        stale.save_to_path(&legacy).unwrap();

        let again = DictationConfig::load_with_legacy(&app_data, &[legacy]);
        assert!((again.vad_threshold - 0.55).abs() < f32::EPSILON);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wake_upgrade_and_continuation_preserve_utterance_audio() {
        let mut buf = SharedCaptureBuffer::new();
        buf.begin_listening(WAKE_WORD_TRIGGER, 3);
        let first_part = vec![0.25f32; 16_000 * 6];
        buf.utterance_pcm.extend_from_slice(&first_part);
        buf.speech_frames_in_utterance = 40;

        // Upgrading wake candidate to hands_free_multimodal keeps existing speech audio intact.
        buf.upgrade_wake_candidate_to("hands_free_multimodal");
        assert_eq!(buf.trigger_source, "hands_free_multimodal");
        assert_eq!(buf.utterance_pcm.len(), first_part.len());
        assert_eq!(buf.speech_frames_in_utterance, 40);

        // Samples buffered during TRANSCRIBING / PREVIEW are carried into the resumed turn.
        let _captured = buf.take_utterance_for_transcription(EndReason::Silence, 200, false);
        assert_eq!(buf.state, PipelineState::TRANSCRIBING);
        let during_tx = vec![0.4f32; 16_000 * 2];
        buf.continuation_pcm.extend_from_slice(&during_tx);

        buf.resume_listening_with_continuation(
            "hands_free_multimodal",
            Some("first part".to_string()),
            2,
            None,
        );
        assert_eq!(buf.state, PipelineState::LISTENING);
        assert_eq!(buf.utterance_pcm, during_tx);
        assert_eq!(buf.carried_transcript_prefix.as_deref(), Some("first part"));
        assert!(buf.continuation_pcm.is_empty());
    }
}
