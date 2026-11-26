//! Saved dictation settings. The audio worker reads this. It does not own the microphone.

use crate::inject::{
    default_auto_send_allowlist, InjectionTimingConfig, TargetChangePolicy,
    DEFAULT_CLIPBOARD_RESTORE_DELAY_MS, DEFAULT_ENTER_DELAY_MS,
};
use crate::license::LicenseEnforcement;
use crate::snippets::{sanitize_snippets, Snippet};
use crate::turn_taking::{
    SendMode, TurnTakingConfig, VisionPowerMode, DEFAULT_ARM_DWELL_MS, DEFAULT_ARM_PITCH_ENTER_DEG,
    DEFAULT_ARM_PITCH_EXIT_DEG, DEFAULT_ARM_YAW_ENTER_DEG, DEFAULT_ARM_YAW_EXIT_DEG,
    DEFAULT_CALIBRATED_RESTING_MAR, DEFAULT_DISARM_DWELL_MS, DEFAULT_MAX_SILENCE_TIMEOUT_MS,
    DEFAULT_MIN_SPEECH_HOTKEY_MS, DEFAULT_MIN_SPEECH_MS, DEFAULT_POST_ROLL_MS,
    MAX_CALIBRATED_RESTING_MAR, MAX_CALIBRATED_YAW_OFFSET_DEG,
};
use crate::wake_word::{VoiceProfile, DEFAULT_WAKE_WORD};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// `off` keeps every feature available. `soft` only warns. `required` gates Pro features.
    #[serde(default = "default_license_enforcement")]
    pub license_enforcement: String,
    /// HTTPS license server. Empty means the public development checker, which must not be used to charge money.
    #[serde(default)]
    pub license_endpoint: String,
    /// Do not auto-start during this local-time window. The window can cross midnight.
    #[serde(default)]
    pub quiet_hours_enabled: bool,
    #[serde(default)]
    pub quiet_hours_start_minute: u16,
    #[serde(default = "default_quiet_end_minute")]
    pub quiet_hours_end_minute: u16,
    /// Turn spoken "period" and "comma" into characters.
    #[serde(default)]
    pub spoken_punctuation: bool,
    /// Capitalize the first letter after a sentence.
    #[serde(default)]
    pub capitalize_sentences: bool,
    /// Where the floating pill sits: bottom-center, bottom-left, bottom-right, top-center.
    #[serde(default = "default_hud_anchor")]
    pub hud_anchor: String,
    /// stable or beta. The app does not phone home for this. It is remembered for a future updater.
    #[serde(default = "default_update_channel")]
    pub update_channel: String,
    /// Keep a short local list of error notes the user can copy. Off by default. Never uploaded.
    #[serde(default)]
    pub diagnostics_opt_in: bool,
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

fn default_license_enforcement() -> String {
    "off".to_string()
}

fn default_quiet_end_minute() -> u16 {
    7 * 60
}

fn default_hud_anchor() -> String {
    "bottom-center".to_string()
}

fn default_update_channel() -> String {
    "stable".to_string()
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
            license_enforcement: default_license_enforcement(),
            license_endpoint: String::new(),
            quiet_hours_enabled: false,
            quiet_hours_start_minute: 22 * 60,
            quiet_hours_end_minute: default_quiet_end_minute(),
            spoken_punctuation: false,
            capitalize_sentences: false,
            hud_anchor: default_hud_anchor(),
            update_channel: default_update_channel(),
            diagnostics_opt_in: false,
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
        self.license_enforcement = LicenseEnforcement::parse(&self.license_enforcement)
            .as_str()
            .to_string();
        self.license_endpoint = self.license_endpoint.trim().to_string();
        self.quiet_hours_start_minute = crate::desk::clamp_minute(self.quiet_hours_start_minute);
        self.quiet_hours_end_minute = crate::desk::clamp_minute(self.quiet_hours_end_minute);
        self.hud_anchor = crate::desk::HudAnchor::parse(&self.hud_anchor)
            .as_str()
            .to_string();
        self.update_channel = crate::desk::UpdateChannel::parse(&self.update_channel)
            .as_str()
            .to_string();
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
