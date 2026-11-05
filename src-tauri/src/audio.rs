use crate::cleanup::{run_local_developer_cleanup, CleanupResult};
use crate::inject::{
    default_auto_send_allowlist, detect_frontmost_app, evaluate_app_injection_policy,
    inject_text_into_focused_app, AppTargetPolicy, InjectionOutcome, InjectionTimingConfig,
    TargetChangePolicy, DEFAULT_CLIPBOARD_RESTORE_DELAY_MS, DEFAULT_ENTER_DELAY_MS,
};
use crate::transcribe::{TranscriberEngine, TranscriptionResult};
use crate::turn_taking::{
    append_session_metrics_line, end_of_turn, evaluate_arming_hysteresis,
    has_minimum_speech_evidence, is_within_arm_exit_cone, resolve_recordings_dir,
     sanitize_discarded_reason_for_metrics, save_tuning_utterance_wav_and_sidecar,
    should_auto_start, silence_countdown_ms, ArmingHysteresisState, EndReason, GateInputs,
    RecordedVisionSample, TuningRecordingSidecar, TurnState, TurnTakingConfig,
    UtteranceMetricsRecord, DEFAULT_ARM_DWELL_MS, DEFAULT_ARM_PITCH_ENTER_DEG,
    DEFAULT_ARM_PITCH_EXIT_DEG, DEFAULT_ARM_YAW_ENTER_DEG, DEFAULT_ARM_YAW_EXIT_DEG,
    DEFAULT_DISARM_DWELL_MS, DEFAULT_MAX_SILENCE_TIMEOUT_MS, DEFAULT_MIN_SPEECH_HOTKEY_MS,
    DEFAULT_MIN_SPEECH_MS, DEFAULT_POST_ROLL_MS, VAD_FRAME_MS,
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
            cancel_on_look_away: false,
            per_app_rules_enabled: true,
            auto_send_allowlist: default_auto_send_allowlist(),
            auto_inject: true,
            auto_press_enter: true,
            use_clipboard_paste: true,
            preferred_device: None,
            hud_enabled: true,
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
        }
    }

    pub fn sanitize_after_parse(&mut self) {
        self.auto_send_allowlist
            .retain(|entry| !entry.trim().eq_ignore_ascii_case("saywhen"));
        // Saving utterance audio for tuning must always turn itself off when the app restarts.
        self.save_tuning_audio_enabled = false;
    }

    pub fn config_file_path() -> std::path::PathBuf {
        let candidates = [
            std::path::PathBuf::from("../settings.local.json"),
            std::path::PathBuf::from("settings.local.json"),
        ];
        for c in &candidates {
            if c.exists() {
                return c.clone();
            }
        }
        std::path::PathBuf::from("../settings.local.json")
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_file_path();
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(mut parsed) = serde_json::from_str::<DictationConfig>(&raw) {
                parsed.sanitize_after_parse();
                return parsed;
            }
        }
        Self::default()
    }

    pub fn save_to_disk(&self) {
        let path = Self::config_file_path();
        let mut persisted = self.clone();
        // Never persist `save_tuning_audio_enabled: true` across restarts.
        persisted.save_tuning_audio_enabled = false;
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(path, json);
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStartedEvent {
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
    pub target_policy: AppTargetPolicy,
    pub countdown_ms: u64,
    pub trigger_source: String,
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
}

#[derive(Debug, Clone)]
pub struct PendingPreviewItem {
    pub utterance_id: u64,
    pub started_at: Instant,
    pub countdown_ms: u64,
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
}

struct SharedCaptureBuffer {
    epoch: Instant,
    next_utterance_id: u64,
    pre_roll: VecDeque<f32>,
    vad_pending: VecDeque<f32>,
    utterance_pcm: Vec<f32>,
    state: PipelineState,
    listening_started_at: Option<Instant>,
    listening_started_at_ms: u64,
    pending_preview: Option<PendingPreviewItem>,
    speech_frames_in_utterance: usize,
    consecutive_vad_frames: usize,
    last_speech_at_ms: u64,
    last_vad_voice_at_ms: Option<u64>,
    last_lip_motion_in_turn_ms: Option<u64>,
    lip_to_listening_ms: Option<u64>,
    post_roll_started_at_ms: Option<u64>,
    pending_end_reason: Option<EndReason>,
    peak_vad_prob: f32,
    latest_rms: f32,
    latest_vad_prob: f32,
    trigger_source: String,
    vision: VisionTelemetryPayload,
    last_vision_update: Option<Instant>,
    arming_hysteresis: ArmingHysteresisState,
    vision_trace: VecDeque<(u64, VisionTelemetryPayload)>,
    camera_enabled: bool,
}

impl SharedCaptureBuffer {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            next_utterance_id: 1,
            pre_roll: VecDeque::with_capacity(PREROLL_SAMPLES + 1024),
            vad_pending: VecDeque::with_capacity(VAD_CHUNK_SAMPLES * 4),
            utterance_pcm: Vec::with_capacity(16_000 * 10),
            state: PipelineState::IDLE,
            listening_started_at: None,
            listening_started_at_ms: 0,
            pending_preview: None,
            speech_frames_in_utterance: 0,
            consecutive_vad_frames: 0,
            last_speech_at_ms: 0,
            last_vad_voice_at_ms: None,
            last_lip_motion_in_turn_ms: None,
            lip_to_listening_ms: None,
            post_roll_started_at_ms: None,
            pending_end_reason: None,
            peak_vad_prob: 0.0,
            latest_rms: 0.0,
            latest_vad_prob: 0.0,
            trigger_source: "manual".to_string(),
            vision: VisionTelemetryPayload::default(),
            last_vision_update: None,
            arming_hysteresis: ArmingHysteresisState::default(),
            vision_trace: VecDeque::with_capacity(256),
            camera_enabled: false,
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
        self.trigger_source = source.to_string();
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
            ends_with_connective: false,
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
        let start_with_preroll_ms = self.listening_started_at_ms.saturating_sub(500);
        let samples = std::mem::take(&mut self.utterance_pcm);
        let peak_vad = self.peak_vad_prob;
        let trigger_source = self.trigger_source.clone();
        let lip_to_listening_ms = self.lip_to_listening_ms.take();
        let speech_frames = self.speech_frames_in_utterance;

        self.peak_vad_prob = 0.0;
        self.speech_frames_in_utterance = 0;
        self.last_lip_motion_in_turn_ms = None;
        self.post_roll_started_at_ms = None;
        self.pending_end_reason = None;

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
        }
    }

    fn cancel_active_or_preview(&mut self, turn_cfg: &TurnTakingConfig) {
        self.state = self.resting_state(turn_cfg);
        self.utterance_pcm.clear();
        self.listening_started_at = None;
        self.pending_preview = None;
        self.speech_frames_in_utterance = 0;
        self.last_lip_motion_in_turn_ms = None;
        self.lip_to_listening_ms = None;
        self.post_roll_started_at_ms = None;
        self.pending_end_reason = None;
        self.peak_vad_prob = 0.0;
    }
}

pub enum AudioCommand {
    StartListening(String),
    StopAndTranscribe(Option<EndReason>),
    ToggleListening(String),
    CancelListening,
    ConfirmPreviewNow(Option<String>),
    CancelPreview(String),
    BeginPreview(PendingPreviewItem),
    SwitchDevice(Option<String>),
    UpdateVision(VisionTelemetryPayload),
    SetCameraGateEnabled(bool),
}

pub struct AudioPipelineController {
    cmd_tx: Sender<AudioCommand>,
    config: Arc<Mutex<DictationConfig>>,
    shared: Arc<Mutex<SharedCaptureBuffer>>,
}

impl AudioPipelineController {
    pub fn spawn(app_handle: AppHandle, transcriber: Arc<TranscriberEngine>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();
        let config = Arc::new(Mutex::new(DictationConfig::load_or_default()));
        let shared = Arc::new(Mutex::new(SharedCaptureBuffer::new()));

        let config_clone = Arc::clone(&config);
        let shared_clone = Arc::clone(&shared);
        let cmd_tx_for_worker = cmd_tx.clone();

        thread::spawn(move || {
            run_audio_worker(
                app_handle,
                transcriber,
                cmd_rx,
                cmd_tx_for_worker,
                config_clone,
                shared_clone,
            );
        });

        Self {
            cmd_tx,
            config,
            shared,
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
        new_cfg.save_to_disk();
        if device_changed {
            let _ = self.send_command(AudioCommand::SwitchDevice(preferred_device));
        }
        Ok(new_cfg)
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
        updated.save_to_disk();
        updated
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
    let supported_config = device
        .default_input_config()
        .map_err(|e| format!("Failed to read microphone config for {}: {}", device_name, e))?;

    let in_sample_rate = supported_config.sample_rate().0;
    let in_channels = supported_config.channels() as usize;
    let sample_format = supported_config.sample_format();
    let stream_config: cpal::StreamConfig = supported_config.into();

    let err_handle = app_handle.clone();
    let err_fn = move |err: cpal::StreamError| {
        let _ = err_handle.emit("pipeline-error", format!("Microphone stream error: {}", err));
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
                                let sum: f32 =
                                    frame.iter().map(|&s| s as f32 / 32768.0).sum();
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
                                let sum: f32 = frame
                                    .iter()
                                    .map(|&s| (s as f32 - 32768.0) / 32768.0)
                                    .sum();
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

fn unix_timestamp_ms() -> u64 {
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

    let _ = app_handle.emit(
        "utterance-complete",
        UtteranceCompleteEvent {
            utterance_id: item.utterance_id,
            transcription: item.transcription,
            cleanup: final_cleanup,
            target_policy: final_policy,
            injection: injection_outcome,
            injection_error,
            trigger_source: item.trigger_source,
            timestamp_ms: now_ms,
        },
    );
}

fn run_audio_worker(
    app_handle: AppHandle,
    transcriber: Arc<TranscriberEngine>,
    cmd_rx: mpsc::Receiver<AudioCommand>,
    cmd_tx_self: Sender<AudioCommand>,
    config: Arc<Mutex<DictationConfig>>,
    shared: Arc<Mutex<SharedCaptureBuffer>>,
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

    loop {
        let cfg = config.lock().unwrap().clone();
        let turn_cfg = cfg.to_turn_taking_config();

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                AudioCommand::StartListening(source) => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let mut guard = shared.lock().unwrap();
                    if guard.state == PipelineState::IDLE || guard.state == PipelineState::ARMED {
                        let seed = if source == "hands_free_multimodal" {
                            guard.consecutive_vad_frames.min(2)
                        } else {
                            0
                        };
                        guard.begin_listening(&source, seed);
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
                AudioCommand::ConfirmPreviewNow(override_text) => {
                    let taken = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state == PipelineState::PREVIEW {
                            let item = guard.pending_preview.take();
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
                    };
                    {
                        let mut guard = shared.lock().unwrap();
                        guard.state = PipelineState::PREVIEW;
                        guard.pending_preview = Some(item);
                    }
                    let _ = app_handle.emit("preview-started", evt);
                }
                AudioCommand::ToggleListening(source) => {
                    let current = shared.lock().unwrap().state;
                    match current {
                        PipelineState::IDLE | PipelineState::ARMED => {
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
                                    let _ = cmd_tx_self
                                        .send(AudioCommand::StopAndTranscribe(Some(EndReason::Nod)));
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
                    if let Err(discard_msg) = has_minimum_speech_evidence(
                        captured.speech_frames,
                        &captured.trigger_source,
                        &turn_cfg,
                    ) {
                        {
                            let mut guard = shared.lock().unwrap();
                            guard.state = guard.resting_state(&turn_cfg);
                        }
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
                        let _ = app_handle.emit("utterance-discarded", discard_msg);
                        continue;
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
                        },
                    );

                    let transcriber_ref = Arc::clone(&transcriber);
                    let shared_after = Arc::clone(&shared);
                    let cfg_snapshot = config.lock().unwrap().clone();
                    let turn_cfg_snapshot = cfg_snapshot.to_turn_taking_config();
                    let app_for_tx = app_handle.clone();
                    let tx_for_preview = cmd_tx_self.clone();

                    thread::spawn(move || {
                        let tx_res =
                            transcriber_ref.transcribe_pcm(&captured.samples, captured.peak_vad);
                        match tx_res {
                            Ok(transcription) => {
                                let trimmed_lower = transcription.text.trim().to_lowercase();
                                if trimmed_lower == "cancel" || trimmed_lower == "cancel." {
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
                                        "Spoken 'cancel' command aborted utterance",
                                    );
                                    return;
                                }

                                let cleanup = run_local_developer_cleanup(
                                    &transcription.text,
                                    cfg_snapshot.cleanup_enabled,
                                );

                                if cleanup.cleaned_text.is_empty() {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state = guard.resting_state(&turn_cfg_snapshot);
                                    }
                                    let reason = transcription
                                        .discarded_reason
                                        .clone()
                                        .unwrap_or_else(|| {
                                            "No speech recognized: nothing was sent".to_string()
                                        });
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

                                let front_app = detect_frontmost_app();
                                let wants_enter = cfg_snapshot.auto_press_enter
                                    || transcription.end_phrase_triggered;
                                let target_policy = evaluate_app_injection_policy(
                                    &front_app,
                                    wants_enter,
                                    cfg_snapshot.per_app_rules_enabled,
                                    cleanup.is_destructive_command,
                                    &cfg_snapshot.auto_send_allowlist,
                                );

                                let tuning_capture = if cfg_snapshot.save_tuning_audio_enabled {
                                    Some((captured.samples, captured.vision_trace))
                                } else {
                                    None
                                };

                                let item = PendingPreviewItem {
                                    utterance_id: captured.utterance_id,
                                    started_at: Instant::now(),
                                    countdown_ms: cfg_snapshot.preview_countdown_ms,
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

                                if cfg_snapshot.preview_enabled && cfg_snapshot.auto_inject {
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
                if let Some(ref item) = guard.pending_preview {
                    item.started_at.elapsed().as_millis() as u64 >= item.countdown_ms
                } else {
                    false
                }
            } else {
                false
            }
        };

        if preview_expired {
            let _ = cmd_tx_self.send(AudioCommand::ConfirmPreviewNow(None));
        }

        // Process pending 512-sample chunks through Silero VAD
        let mut should_auto_stop: Option<EndReason> = None;
        let mut should_auto_start_listening = false;

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

            if guard.state == PipelineState::ARMED
                && (!cfg.hands_free_enabled || !guard.is_vision_fresh())
            {
                guard.arming_hysteresis.reset();
                guard.state = PipelineState::IDLE;
            }

            // Stage 3 Multimodal Auto-Start via pure decision function
            let gate_inputs = GateInputs {
                is_armed: guard.state == PipelineState::ARMED,
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

            if guard.state == PipelineState::LISTENING {
                if prob > guard.peak_vad_prob {
                    guard.peak_vad_prob = prob;
                }
                if guard.vision.lips_moving && guard.is_vision_fresh() {
                    guard.last_lip_motion_in_turn_ms = Some(now_ms);
                }

                if prob >= cfg.vad_threshold {
                    guard.speech_frames_in_utterance += 1;
                    guard.last_speech_at_ms = now_ms;
                    // If the user resumed speaking during a silence-triggered post-roll, abort the stop.
                    if guard.pending_end_reason == Some(EndReason::Silence) {
                        guard.post_roll_started_at_ms = None;
                        guard.pending_end_reason = None;
                    }
                }

                if guard.post_roll_started_at_ms.is_none() {
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

        if should_auto_start_listening {
            let _ = cmd_tx_self.send(AudioCommand::StartListening(
                "hands_free_multimodal".to_string(),
            ));
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
        legacy["vadThreshold"] = serde_json::json!(0.7);
        let mut parsed: DictationConfig = serde_json::from_value(legacy).unwrap();
        assert!(parsed.hud_enabled);
        assert_eq!(parsed.on_target_changed, TargetChangePolicy::Abort);
        assert_eq!(parsed.enter_delay_ms, DEFAULT_ENTER_DELAY_MS);
        assert!(parsed.per_app_enter_delay_ms.is_empty());
        assert_eq!(
            parsed.clipboard_restore_delay_ms,
            DEFAULT_CLIPBOARD_RESTORE_DELAY_MS
        );
        assert_eq!(parsed.max_silence_timeout_ms, DEFAULT_MAX_SILENCE_TIMEOUT_MS);
        assert_eq!(parsed.arm_yaw_enter_deg, DEFAULT_ARM_YAW_ENTER_DEG);
        assert_eq!(parsed.arm_yaw_exit_deg, DEFAULT_ARM_YAW_EXIT_DEG);
        assert_eq!(parsed.arm_dwell_ms, DEFAULT_ARM_DWELL_MS);
        assert_eq!(parsed.disarm_dwell_ms, DEFAULT_DISARM_DWELL_MS);
        assert_eq!(parsed.min_speech_ms, DEFAULT_MIN_SPEECH_MS);
        assert_eq!(parsed.min_speech_hotkey_ms, DEFAULT_MIN_SPEECH_HOTKEY_MS);
        assert_eq!(parsed.post_roll_ms, DEFAULT_POST_ROLL_MS);
        assert!(!parsed.session_metrics_enabled);
        assert!(!parsed.save_tuning_audio_enabled);
        assert!((parsed.vad_threshold - 0.7).abs() < f32::EPSILON);

        // Even if someone manually edited settings.local.json with saveTuningAudioEnabled = true,
        // sanitize_after_parse forces it back to false on startup.
        parsed.save_tuning_audio_enabled = true;
        parsed.sanitize_after_parse();
        assert!(!parsed.save_tuning_audio_enabled);
    }
}
