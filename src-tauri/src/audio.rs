use crate::cleanup::{run_local_developer_cleanup, CleanupResult};
use crate::inject::{
    default_auto_send_allowlist, detect_frontmost_app, evaluate_app_injection_policy,
    inject_text_into_focused_app, AppTargetPolicy, InjectionOutcome, InjectionTimingConfig,
    TargetChangePolicy, DEFAULT_CLIPBOARD_RESTORE_DELAY_MS, DEFAULT_ENTER_DELAY_MS,
};
use crate::transcribe::{TranscriberEngine, TranscriptionResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
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

impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            hands_free_enabled: true,
            vad_threshold: 0.45,
            silence_timeout_ms: 650,
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
                parsed
                    .auto_send_allowlist
                    .retain(|entry| !entry.trim().eq_ignore_ascii_case("saywhen"));
                return parsed;
            }
        }
        Self::default()
    }

    pub fn save_to_disk(&self) {
        let path = Self::config_file_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
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
    pub device_name: String,
    pub speech_detected_in_utterance: bool,
    pub vision_active: bool,
    pub vision_armed: bool,
    pub vision_lips_moving: bool,
    /// The user turned the camera gate on in the UI. Combined with `vision_active == false`
    /// this means vision frames stopped arriving (for example a throttled hidden webview).
    pub camera_enabled: bool,
    pub hud_enabled: bool,
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
    pub started_at: Instant,
    pub countdown_ms: u64,
    pub transcription: TranscriptionResult,
    pub cleanup: CleanupResult,
    pub target_policy: AppTargetPolicy,
    pub trigger_source: String,
}

struct SharedCaptureBuffer {
    pre_roll: VecDeque<f32>,
    vad_pending: VecDeque<f32>,
    utterance_pcm: Vec<f32>,
    state: PipelineState,
    listening_started_at: Option<Instant>,
    pending_preview: Option<PendingPreviewItem>,
    speech_frames_in_utterance: usize,
    last_speech_instant: Option<Instant>,
    last_vad_voice_instant: Option<Instant>,
    peak_vad_prob: f32,
    latest_rms: f32,
    latest_vad_prob: f32,
    trigger_source: String,
    vision: VisionTelemetryPayload,
    last_vision_update: Option<Instant>,
    camera_enabled: bool,
}

impl SharedCaptureBuffer {
    fn new() -> Self {
        Self {
            pre_roll: VecDeque::with_capacity(PREROLL_SAMPLES + 1024),
            vad_pending: VecDeque::with_capacity(VAD_CHUNK_SAMPLES * 4),
            utterance_pcm: Vec::with_capacity(16_000 * 10),
            state: PipelineState::IDLE,
            listening_started_at: None,
            pending_preview: None,
            speech_frames_in_utterance: 0,
            last_speech_instant: None,
            last_vad_voice_instant: None,
            peak_vad_prob: 0.0,
            latest_rms: 0.0,
            latest_vad_prob: 0.0,
            trigger_source: "manual".to_string(),
            vision: VisionTelemetryPayload::default(),
            last_vision_update: None,
            camera_enabled: false,
        }
    }

    fn is_vision_fresh(&self) -> bool {
        self.last_vision_update
            .map(|t| t.elapsed() < Duration::from_millis(650))
            .unwrap_or(false)
    }

    fn resting_state(&self, hands_free_enabled: bool) -> PipelineState {
        if hands_free_enabled
            && self.is_vision_fresh()
            && self.vision.face_present
            && self.vision.facing_screen
        {
            PipelineState::ARMED
        } else {
            PipelineState::IDLE
        }
    }

    fn begin_listening(&mut self, source: &str) {
        self.utterance_pcm.clear();
        // Prepend the 500ms circular pre-roll ring buffer so the first syllable is never clipped
        self.utterance_pcm.extend(self.pre_roll.iter().copied());
        self.state = PipelineState::LISTENING;
        self.listening_started_at = Some(Instant::now());
        self.pending_preview = None;
        self.speech_frames_in_utterance = 0;
        self.last_speech_instant = Some(Instant::now());
        self.peak_vad_prob = self.latest_vad_prob;
        self.trigger_source = source.to_string();
    }

    fn take_utterance_for_transcription(&mut self) -> (Vec<f32>, f32, String) {
        self.state = PipelineState::TRANSCRIBING;
        self.listening_started_at = None;
        let samples = std::mem::take(&mut self.utterance_pcm);
        let peak = self.peak_vad_prob;
        let source = self.trigger_source.clone();
        self.peak_vad_prob = 0.0;
        self.speech_frames_in_utterance = 0;
        self.last_speech_instant = None;
        (samples, peak, source)
    }

    fn cancel_active_or_preview(&mut self, hands_free_enabled: bool) {
        self.state = self.resting_state(hands_free_enabled);
        self.utterance_pcm.clear();
        self.listening_started_at = None;
        self.pending_preview = None;
        self.speech_frames_in_utterance = 0;
        self.last_speech_instant = None;
        self.peak_vad_prob = 0.0;
    }
}

pub enum AudioCommand {
    StartListening(String),
    StopAndTranscribe,
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

fn finalize_preview_and_inject(
    app_handle: &AppHandle,
    item: PendingPreviewItem,
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

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let mut final_cleanup = item.cleanup;
    final_cleanup.cleaned_text = final_text;

    let _ = app_handle.emit(
        "utterance-complete",
        UtteranceCompleteEvent {
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

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                AudioCommand::StartListening(source) => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let mut guard = shared.lock().unwrap();
                    if guard.state == PipelineState::IDLE || guard.state == PipelineState::ARMED {
                        guard.begin_listening(&source);
                    }
                }
                AudioCommand::CancelListening => {
                    if let Some(ref mut v) = vad {
                        v.reset();
                    }
                    let mut guard = shared.lock().unwrap();
                    guard.cancel_active_or_preview(cfg.hands_free_enabled);
                    let _ = app_handle.emit("utterance-canceled", "Canceled");
                }
                AudioCommand::CancelPreview(reason) => {
                    let mut guard = shared.lock().unwrap();
                    if guard.state == PipelineState::PREVIEW {
                        guard.cancel_active_or_preview(cfg.hands_free_enabled);
                        let _ = app_handle.emit("preview-canceled", reason);
                    }
                }
                AudioCommand::ConfirmPreviewNow(override_text) => {
                    let taken = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state == PipelineState::PREVIEW {
                            let item = guard.pending_preview.take();
                            guard.state = guard.resting_state(cfg.hands_free_enabled);
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
                            let _ = cmd_tx_self.send(AudioCommand::StopAndTranscribe);
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
                    let gesture = vision_payload.gesture.clone();
                    guard.vision = vision_payload;
                    guard.last_vision_update = Some(Instant::now());

                    match guard.state {
                        PipelineState::IDLE => {
                            if cfg.hands_free_enabled
                                && guard.vision.face_present
                                && guard.vision.facing_screen
                            {
                                guard.state = PipelineState::ARMED;
                            }
                        }
                        PipelineState::ARMED => {
                            if !cfg.hands_free_enabled
                                || !guard.vision.face_present
                                || !guard.vision.facing_screen
                            {
                                guard.state = PipelineState::IDLE;
                            }
                        }
                        PipelineState::LISTENING => {
                            if gesture == "shake" {
                                guard.cancel_active_or_preview(cfg.hands_free_enabled);
                                let _ = app_handle.emit(
                                    "utterance-canceled",
                                    "Head shake gesture canceled utterance",
                                );
                            } else if gesture == "nod" {
                                let _ = cmd_tx_self.send(AudioCommand::StopAndTranscribe);
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
                                && (!guard.vision.face_present || !guard.vision.facing_screen)
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
                AudioCommand::StopAndTranscribe => {
                    let (samples, peak_vad, trigger_source) = {
                        let mut guard = shared.lock().unwrap();
                        if guard.state != PipelineState::LISTENING {
                            continue;
                        }
                        guard.take_utterance_for_transcription()
                    };

                    if let Some(ref mut v) = vad {
                        v.reset();
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
                            device_name: active_device_name.clone(),
                            speech_detected_in_utterance: true,
                            vision_active: false,
                            vision_armed: false,
                            vision_lips_moving: false,
                            camera_enabled: shared.lock().unwrap().camera_enabled,
                            hud_enabled: cfg.hud_enabled,
                        },
                    );

                    let transcriber_ref = Arc::clone(&transcriber);
                    let shared_after = Arc::clone(&shared);
                    let cfg_snapshot = config.lock().unwrap().clone();
                    let app_for_tx = app_handle.clone();
                    let tx_for_preview = cmd_tx_self.clone();

                    thread::spawn(move || {
                        let tx_res = transcriber_ref.transcribe_pcm(&samples, peak_vad);
                        match tx_res {
                            Ok(transcription) => {
                                let trimmed_lower = transcription.text.trim().to_lowercase();
                                if trimmed_lower == "cancel" || trimmed_lower == "cancel." {
                                    let mut guard = shared_after.lock().unwrap();
                                    guard.state =
                                        guard.resting_state(cfg_snapshot.hands_free_enabled);
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
                                    let mut guard = shared_after.lock().unwrap();
                                    guard.state =
                                        guard.resting_state(cfg_snapshot.hands_free_enabled);
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

                                let item = PendingPreviewItem {
                                    started_at: Instant::now(),
                                    countdown_ms: cfg_snapshot.preview_countdown_ms,
                                    transcription,
                                    cleanup,
                                    target_policy,
                                    trigger_source,
                                };

                                if cfg_snapshot.preview_enabled && cfg_snapshot.auto_inject {
                                    let _ = tx_for_preview.send(AudioCommand::BeginPreview(item));
                                } else {
                                    {
                                        let mut guard = shared_after.lock().unwrap();
                                        guard.state =
                                            guard.resting_state(cfg_snapshot.hands_free_enabled);
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
                                    guard.state =
                                        guard.resting_state(cfg_snapshot.hands_free_enabled);
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
        let mut should_auto_stop = false;
        let mut should_auto_start = false;

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
            guard.latest_vad_prob = prob;

            if prob >= cfg.vad_threshold {
                guard.last_vad_voice_instant = Some(Instant::now());
            }

            if guard.state == PipelineState::ARMED
                && (!cfg.hands_free_enabled || !guard.is_vision_fresh())
            {
                guard.state = PipelineState::IDLE;
            }

            // Stage 3 Multimodal Auto-Start: ARMED + Lips Moving + Silero VAD Speech -> LISTENING
            if guard.state == PipelineState::ARMED
                && cfg.hands_free_enabled
                && guard.is_vision_fresh()
                && guard.vision.face_present
                && guard.vision.facing_screen
                && guard.vision.lips_moving
            {
                let recent_voice = guard
                    .last_vad_voice_instant
                    .map(|t| t.elapsed() <= Duration::from_millis(320))
                    .unwrap_or(false);
                if prob >= cfg.vad_threshold || recent_voice {
                    should_auto_start = true;
                }
            }

            if guard.state == PipelineState::LISTENING {
                if prob > guard.peak_vad_prob {
                    guard.peak_vad_prob = prob;
                }
                if prob >= cfg.vad_threshold {
                    guard.speech_frames_in_utterance += 1;
                    guard.last_speech_instant = Some(Instant::now());
                } else if cfg.auto_stop_on_silence && guard.speech_frames_in_utterance >= 3 {
                    let mouth_quiet = if guard.is_vision_fresh() && cfg.hands_free_enabled {
                        guard.vision.mouth_closed || !guard.vision.lips_moving
                    } else {
                        true
                    };

                    if mouth_quiet {
                        if let Some(last_speech) = guard.last_speech_instant {
                            if last_speech.elapsed().as_millis() as u64 >= cfg.silence_timeout_ms {
                                should_auto_stop = true;
                            }
                        }
                    }
                }
            }
        }

        if should_auto_start {
            let _ = cmd_tx_self.send(AudioCommand::StartListening(
                "hands_free_multimodal".to_string(),
            ));
        }

        if should_auto_stop {
            let _ = cmd_tx_self.send(AudioCommand::StopAndTranscribe);
        }

        if last_telemetry_emit.elapsed() >= Duration::from_millis(50) {
            last_telemetry_emit = Instant::now();
            let payload = {
                let guard = shared.lock().unwrap();
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
                let vision_fresh = guard.is_vision_fresh();
                AudioTelemetryEvent {
                    state: guard.state.as_str().to_string(),
                    rms: guard.latest_rms,
                    vad_prob: guard.latest_vad_prob,
                    is_speech: guard.latest_vad_prob >= cfg.vad_threshold,
                    pre_roll_ms,
                    listening_duration_ms,
                    preview_remaining_ms,
                    device_name: active_device_name.clone(),
                    speech_detected_in_utterance: guard.speech_frames_in_utterance >= 3,
                    vision_active: vision_fresh,
                    vision_armed: vision_fresh
                        && guard.vision.face_present
                        && guard.vision.facing_screen,
                    vision_lips_moving: vision_fresh && guard.vision.lips_moving,
                    camera_enabled: guard.camera_enabled,
                    hud_enabled: cfg.hud_enabled,
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
        // A settings.local.json written before the HUD and injection reliability fields existed must still load.
        let mut legacy = serde_json::to_value(DictationConfig::default()).unwrap();
        let obj = legacy.as_object_mut().unwrap();
        obj.remove("hudEnabled");
        obj.remove("onTargetChanged");
        obj.remove("enterDelayMs");
        obj.remove("perAppEnterDelayMs");
        obj.remove("clipboardRestoreDelayMs");
        legacy["vadThreshold"] = serde_json::json!(0.7);
        let parsed: DictationConfig = serde_json::from_value(legacy).unwrap();
        assert!(parsed.hud_enabled);
        assert_eq!(parsed.on_target_changed, TargetChangePolicy::Abort);
        assert_eq!(parsed.enter_delay_ms, DEFAULT_ENTER_DELAY_MS);
        assert!(parsed.per_app_enter_delay_ms.is_empty());
        assert_eq!(
            parsed.clipboard_restore_delay_ms,
            DEFAULT_CLIPBOARD_RESTORE_DELAY_MS
        );
        assert!((parsed.vad_threshold - 0.7).abs() < f32::EPSILON);
    }
}
