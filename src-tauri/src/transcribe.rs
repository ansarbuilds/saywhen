use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub filename: String,
    pub label: String,
    pub size_mb: u64,
    pub installed: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub audio_duration_ms: u64,
    pub inference_ms: u64,
    pub rtf: f32,
    pub model_used: String,
    pub peak_vad_prob: f32,
    pub end_phrase_triggered: bool,
    /// Set when the transcript was dropped as a likely Whisper hallucination on non-speech audio.
    #[serde(default)]
    pub discarded_reason: Option<String>,
}

pub struct TranscriberEngine {
    loaded_model: Mutex<Option<(String, Arc<WhisperContext>)>>,
    cached_state: Mutex<Option<(String, WhisperState)>>,
    fast_loaded_model: Mutex<Option<(String, Arc<WhisperContext>)>>,
    fast_cached_state: Mutex<Option<(String, WhisperState)>>,
    selected_model_id: Mutex<String>,
    app_models_dir: Option<PathBuf>,
    download_lock: Mutex<()>,
}

fn model_filename(model_id: &str) -> Option<&'static str> {
    match model_id {
        "tiny.en" => Some("ggml-tiny.en.bin"),
        "base.en" => Some("ggml-base.en.bin"),
        "small.en" => Some("ggml-small.en.bin"),
        "large-v3-turbo" => Some("ggml-large-v3-turbo-q5_0.bin"),
        _ => None,
    }
}

impl TranscriberEngine {
    pub fn new() -> Self {
        Self::with_app_data_dir(None)
    }

    pub fn with_app_data_dir(app_data_dir: Option<PathBuf>) -> Self {
        let engine = Self {
            download_lock: Mutex::new(()),
            loaded_model: Mutex::new(None),
            cached_state: Mutex::new(None),
            fast_loaded_model: Mutex::new(None),
            fast_cached_state: Mutex::new(None),
            selected_model_id: Mutex::new("tiny.en".to_string()),
            app_models_dir: app_data_dir.map(|d| d.join("models")),
        };
        let models_dir = engine.resolve_models_dir();
        for (id, file) in [
            ("large-v3-turbo", "ggml-large-v3-turbo-q5_0.bin"),
            ("small.en", "ggml-small.en.bin"),
            ("base.en", "ggml-base.en.bin"),
            ("tiny.en", "ggml-tiny.en.bin"),
        ] {
            if validate_downloaded_model(&models_dir.join(file)).is_ok() {
                if let Ok(mut selected) = engine.selected_model_id.lock() {
                    *selected = id.to_string();
                }
                break;
            }
        }
        engine
    }

    pub fn resolve_models_dir(&self) -> PathBuf {
        // Packaged apps always use their writable data directory, independent of cwd.
        if !cfg!(debug_assertions) {
            if let Some(ref app_dir) = self.app_models_dir {
                return app_dir.clone();
            }
        }
        let mut candidates = vec![PathBuf::from("models"), PathBuf::from("../models")];
        if let Some(ref app_dir) = self.app_models_dir {
            candidates.push(app_dir.clone());
        }

        // Prefer a candidate directory that already has a downloaded Whisper .bin model
        for candidate in &candidates {
            if candidate.join("ggml-tiny.en.bin").exists()
                || candidate.join("ggml-base.en.bin").exists()
                || candidate.join("ggml-small.en.bin").exists()
                || candidate.join("ggml-large-v3-turbo-q5_0.bin").exists()
            {
                return candidate.clone();
            }
        }
        for candidate in &candidates {
            if candidate.exists() && candidate.is_dir() {
                return candidate.clone();
            }
        }
        let fallback = self
            .app_models_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("../models"));
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }

    /// Path the downloader writes to, so the UI can follow its progress.
    pub fn model_download_path(&self, model_id: &str) -> Option<PathBuf> {
        let filename = model_filename(model_id)?;
        Some(self.resolve_models_dir().join(filename).with_extension("bin.part"))
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        let dir = self.resolve_models_dir();
        let specs = [
            (
                "tiny.en",
                "ggml-tiny.en.bin",
                "Whisper Tiny.en (Fastest, ~75 MB)",
                75u64,
            ),
            (
                "base.en",
                "ggml-base.en.bin",
                "Whisper Base.en (Balanced, ~142 MB)",
                142u64,
            ),
            (
                "small.en",
                "ggml-small.en.bin",
                "Whisper Small.en (High Accuracy, ~465 MB)",
                465u64,
            ),
            (
                "large-v3-turbo",
                "ggml-large-v3-turbo-q5_0.bin",
                "Whisper Large v3 Turbo (Best Accuracy, ~547 MB)",
                547u64,
            ),
        ];

        specs
            .iter()
            .map(|(id, filename, label, default_mb)| {
                let full_path = dir.join(filename);
                let installed = validate_downloaded_model(&full_path).is_ok();
                let size_mb = if installed {
                    std::fs::metadata(&full_path)
                        .map(|m| m.len() / (1024 * 1024))
                        .unwrap_or(*default_mb)
                } else {
                    *default_mb
                };
                ModelInfo {
                    id: id.to_string(),
                    filename: filename.to_string(),
                    label: label.to_string(),
                    size_mb,
                    installed,
                    path: full_path.to_string_lossy().to_string(),
                }
            })
            .collect()
    }

    pub fn set_selected_model(&self, model_id: &str) -> Result<String, String> {
        let models = self.list_models();
        let target = models
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| format!("Unknown model id: {}", model_id))?;

        if !target.installed {
            return Err(format!(
                "Model {} is not downloaded yet. Click Download in the Model Manager first.",
                target.filename
            ));
        }

        let mut selected = self.selected_model_id.lock().unwrap();
        *selected = model_id.to_string();
        Ok(target.filename)
    }

    pub fn get_selected_model_id(&self) -> String {
        self.selected_model_id.lock().unwrap().clone()
    }

    fn ensure_fast_context(&self, model_path: &Path) -> Result<Arc<WhisperContext>, String> {
        let path_key = model_path.to_string_lossy().to_string();
        let mut guard = self.fast_loaded_model.lock().unwrap();
        if let Some((ref loaded_key, ref ctx)) = *guard {
            if loaded_key == &path_key {
                return Ok(Arc::clone(ctx));
            }
        }

        if !model_path.exists() {
            return self.ensure_context(model_path);
        }

        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(true);
        ctx_params.flash_attn(true);

        let ctx = WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| format!("Failed to initialize fast WhisperContext: {:?}", e))?;
        let arc_ctx = Arc::new(ctx);
        *guard = Some((path_key, Arc::clone(&arc_ctx)));
        Ok(arc_ctx)
    }

    /// Low-latency transcription pass used for mid-utterance partial checks and early wake-word
    /// verification so they complete in ~60-110 ms and never block final utterance decoding.
    pub fn transcribe_pcm_fast(
        &self,
        samples_16k: &[f32],
        peak_vad_prob: f32,
    ) -> Result<TranscriptionResult, String> {
        let selected_id = self.get_selected_model_id();
        if selected_id == "tiny.en" || selected_id == "base.en" {
            return self.transcribe_pcm(samples_16k, peak_vad_prob);
        }
        let models_dir = self.resolve_models_dir();
        let (fast_id, fast_file) = if validate_downloaded_model(&models_dir.join("ggml-base.en.bin")).is_ok() {
            ("base.en", "ggml-base.en.bin")
        } else if validate_downloaded_model(&models_dir.join("ggml-tiny.en.bin")).is_ok() {
            ("tiny.en", "ggml-tiny.en.bin")
        } else {
            return self.transcribe_pcm(samples_16k, peak_vad_prob);
        };
        if samples_16k.len() < 1600 {
            return Err("Audio segment too short (< 100 ms)".to_string());
        }
        let model_path = models_dir.join(fast_file);
        let path_key = model_path.to_string_lossy().to_string();
        let ctx = self.ensure_fast_context(&model_path)?;
        let start = Instant::now();
        let mut state = {
            let taken = self
                .fast_cached_state
                .lock()
                .ok()
                .and_then(|mut g| g.take())
                .filter(|(k, _)| k == &path_key)
                .map(|(_, s)| s);
            match taken {
                Some(s) => s,
                None => ctx
                    .create_state()
                    .map_err(|e| format!("Failed to create fast WhisperState: {:?}", e))?,
            }
        };
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 8) as i32)
            .unwrap_or(4);
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(n_threads);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_temperature(0.0);
        params.set_temperature_inc(0.0);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(false);

        let pcm_input = normalize_and_pad_speech_pcm(samples_16k);
        let infer_res = state
            .full(params, &pcm_input)
            .map_err(|e| format!("Whisper fast inference failed: {:?}", e));
        if infer_res.is_err() {
            if let Ok(mut g) = self.fast_cached_state.lock() {
                *g = Some((path_key, state));
            }
            infer_res?;
            unreachable!();
        }
        let mut raw_text = String::new();
        for segment in state.as_iter() {
            if let Ok(seg_str) = segment.to_str_lossy() {
                raw_text.push_str(&seg_str);
            }
        }
        if let Ok(mut g) = self.fast_cached_state.lock() {
            *g = Some((path_key, state));
        }
        let cleaned_raw = clean_whisper_hallucinations(&raw_text);
        let (mut cleaned, end_phrase_triggered) = strip_end_phrase(&cleaned_raw);
        let mut discarded_reason = None;
        if is_likely_silence_hallucination(&cleaned, peak_vad_prob) {
            discarded_reason = Some(format!(
                "Discarded \"{}\" as a likely Whisper hallucination (peak VAD {:.2})",
                cleaned.trim(),
                peak_vad_prob
            ));
            cleaned.clear();
        }
        let inference_ms = start.elapsed().as_millis() as u64;
        let audio_duration_ms = ((samples_16k.len() as f64 / 16000.0) * 1000.0) as u64;
        let rtf = if audio_duration_ms > 0 {
            (inference_ms as f32) / (audio_duration_ms as f32)
        } else {
            0.0
        };
        Ok(TranscriptionResult {
            text: cleaned,
            audio_duration_ms,
            inference_ms,
            rtf,
            model_used: fast_id.to_string(),
            peak_vad_prob,
            end_phrase_triggered,
            discarded_reason,
        })
    }
    /// first transcription, so `is_loaded` is false right after launch.
    pub fn is_model_available(&self) -> bool {
        let selected_id = self.get_selected_model_id();
        let filename = model_filename(&selected_id).unwrap_or("ggml-tiny.en.bin");
        validate_downloaded_model(&self.resolve_models_dir().join(filename)).is_ok()
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded_model
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn unload(&self) {
        if let Ok(mut state_guard) = self.cached_state.lock() {
            *state_guard = None;
        }
        if let Ok(mut guard) = self.loaded_model.lock() {
            *guard = None;
        }
        if let Ok(mut state_guard) = self.fast_cached_state.lock() {
            *state_guard = None;
        }
        if let Ok(mut guard) = self.fast_loaded_model.lock() {
            *guard = None;
        }
    }

    pub fn download_model(&self, model_id: &str) -> Result<ModelInfo, String> {
        let _download = self
            .download_lock
            .lock()
            .map_err(|_| "Model download lock failed")?;
        let filename = model_filename(model_id)
            .ok_or_else(|| format!("Unsupported model id: {}", model_id))?;
        let expected_sha256 = pinned_model_sha256(filename)
            .ok_or_else(|| format!("No pinned checksum for model: {}", model_id))?;

        let dir = self.resolve_models_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create models dir: {}", e))?;
        let dest = dir.join(filename);
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/{}/{}",
            MODEL_REPO_REVISION, filename
        );

        // Never expose a partial download as an installed model or overwrite a working one.
        let temporary = dest.with_extension("bin.part");
        let result = (|| -> Result<(), String> {
            let status = std::process::Command::new("curl")
                .args([
                    "--proto",
                    "=https",
                    "--proto-redir",
                    "=https",
                    "-L",
                    "-f",
                    "-sS",
                    "--connect-timeout",
                    "30",
                    "--max-time",
                    "1800",
                    "--retry",
                    "2",
                    &url,
                    "-o",
                ])
                .arg(&temporary)
                .status()
                .map_err(|e| format!("Failed to run curl to download model: {}", e))?;
            if !status.success() {
                return Err("Model download failed. Check your connection and retry.".to_string());
            }
            validate_downloaded_model(&temporary)?;
            verify_model_sha256(&temporary, expected_sha256)?;
            // An existing model is kept intact. Downloads are serialized by download_lock.
            if validate_downloaded_model(&dest).is_err() {
                if dest.exists() {
                    std::fs::remove_file(&dest)
                        .map_err(|e| format!("Failed to replace invalid model: {}", e))?;
                }
                std::fs::rename(&temporary, &dest)
                    .map_err(|e| format!("Failed to install downloaded model: {}", e))?;
            }
            Ok(())
        })();
        let _ = std::fs::remove_file(&temporary);
        result?;

        self.list_models()
            .into_iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| "Model metadata missing after download".to_string())
    }

    fn ensure_context(&self, model_path: &Path) -> Result<Arc<WhisperContext>, String> {
        let path_key = model_path.to_string_lossy().to_string();
        let mut guard = self.loaded_model.lock().unwrap();
        if let Some((ref loaded_key, ref ctx)) = *guard {
            if loaded_key == &path_key {
                return Ok(Arc::clone(ctx));
            }
        }

        if !model_path.exists() {
            return Err(format!(
                "Whisper model file not found at {}. Please download the model first.",
                path_key
            ));
        }

        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(true);
        ctx_params.flash_attn(true);

        let ctx = WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| format!("Failed to initialize WhisperContext: {:?}", e))?;
        let arc_ctx = Arc::new(ctx);
        *guard = Some((path_key, Arc::clone(&arc_ctx)));
        Ok(arc_ctx)
    }

    pub fn transcribe_pcm(
        &self,
        samples_16k: &[f32],
        peak_vad_prob: f32,
    ) -> Result<TranscriptionResult, String> {
        if samples_16k.len() < 1600 {
            return Err("Audio segment too short (< 100 ms)".to_string());
        }

        let selected_id = self.get_selected_model_id();
        let filename = model_filename(&selected_id).unwrap_or("ggml-tiny.en.bin");
        let model_path = self.resolve_models_dir().join(filename);
        let path_key = model_path.to_string_lossy().to_string();
        let ctx = self.ensure_context(&model_path)?;

        let start = Instant::now();
        let mut state = {
            let taken = self
                .cached_state
                .lock()
                .ok()
                .and_then(|mut g| g.take())
                .filter(|(k, _)| k == &path_key)
                .map(|(_, s)| s);
            match taken {
                Some(s) => s,
                None => ctx
                    .create_state()
                    .map_err(|e| format!("Failed to create WhisperState: {:?}", e))?,
            }
        };

        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 8) as i32)
            .unwrap_or(4);

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(n_threads);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_temperature(0.0);
        params.set_temperature_inc(0.0);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(false);

        let pcm_input = normalize_and_pad_speech_pcm(samples_16k);

        let infer_res = state
            .full(params, &pcm_input)
            .map_err(|e| format!("Whisper inference failed: {:?}", e));

        if infer_res.is_err() {
            if let Ok(mut g) = self.cached_state.lock() {
                *g = Some((path_key, state));
            }
            infer_res?;
            unreachable!();
        }

        let mut raw_text = String::new();
        for segment in state.as_iter() {
            if let Ok(seg_str) = segment.to_str_lossy() {
                raw_text.push_str(&seg_str);
            }
        }
        if let Ok(mut g) = self.cached_state.lock() {
            *g = Some((path_key, state));
        }

        let cleaned_raw = clean_whisper_hallucinations(&raw_text);
        let (mut cleaned, end_phrase_triggered) = strip_end_phrase(&cleaned_raw);
        let mut discarded_reason = None;
        if is_likely_silence_hallucination(&cleaned, peak_vad_prob) {
            discarded_reason = Some(format!(
                "Discarded \"{}\" as a likely Whisper hallucination (peak VAD {:.2})",
                cleaned.trim(),
                peak_vad_prob
            ));
            cleaned.clear();
        }
        let inference_ms = start.elapsed().as_millis() as u64;
        let audio_duration_ms = ((samples_16k.len() as f64 / 16000.0) * 1000.0) as u64;
        let rtf = if audio_duration_ms > 0 {
            (inference_ms as f32) / (audio_duration_ms as f32)
        } else {
            0.0
        };

        Ok(TranscriptionResult {
            text: cleaned,
            audio_duration_ms,
            inference_ms,
            rtf,
            model_used: selected_id,
            peak_vad_prob,
            end_phrase_triggered,
            discarded_reason,
        })
    }
}

fn normalize_and_pad_speech_pcm(samples_16k: &[f32]) -> Vec<f32> {
    let min_samples = 16_800usize;
    let peak = samples_16k
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    let gain = if (0.012..0.65).contains(&peak) {
        (0.75 / peak).min(5.0)
    } else {
        1.0
    };
    let mut out = Vec::with_capacity(samples_16k.len().max(min_samples));
    for &s in samples_16k {
        out.push((s * gain).clamp(-0.98, 0.98));
    }
    if out.len() < min_samples {
        out.resize(min_samples, 0.0);
    }
    out
}

fn strip_end_phrase(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    let stripped_punct = trimmed
        .trim_end_matches(|c: char| ".!,?;:".contains(c))
        .trim_end();
    let lower = stripped_punct.to_lowercase();

    for phrase in ["send it", "submit it"] {
        if lower == phrase {
            return (String::new(), true);
        }
        if lower.ends_with(phrase) {
            let prefix_len = stripped_punct.len() - phrase.len();
            let before = stripped_punct[..prefix_len]
                .trim_end()
                .trim_end_matches(|c: char| ",;:-".contains(c))
                .trim_end();
            return (before.to_string(), true);
        }
    }
    (trimmed.to_string(), false)
}

/// Peak Silero VAD probability at or above which a stock phrase is trusted as real speech.
/// Tuned from the 56-utterance macOS turn-taking corpus (`cargo run --bin replay`):
/// real target speech (including short replies "yes", "no", "okay", "continue", "thank you")
/// measured `peak_vad >= 0.957`, whereas non-speech breath/noise bursts peaked at `<= 0.89`.
const HALLUCINATION_TRUST_VAD: f32 = 0.90;

/// Phrases Whisper commonly invents on silence, noise, coughs or keyboard sounds.
/// Matched against the whole transcript only, never as a substring.
const SILENCE_HALLUCINATION_PHRASES: &[&str] = &[
    "thank you",
    "thank you very much",
    "thanks",
    "thanks for watching",
    "thank you for watching",
    "thank you so much for watching",
    "please subscribe",
    "subscribe",
    "bye",
    "bye bye",
    "you",
    "uh",
    "um",
    "hmm",
];

fn normalize_for_phrase_match(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True when the whole transcript is a stock hallucination phrase and the audio did not
/// contain confident speech. A lone "you" is always dropped: it is the most common
/// hallucination and is not a useful command on its own.
fn is_likely_silence_hallucination(text: &str, peak_vad_prob: f32) -> bool {
    let norm = normalize_for_phrase_match(text);
    if norm.is_empty() || !SILENCE_HALLUCINATION_PHRASES.contains(&norm.as_str()) {
        return false;
    }
    norm == "you" || peak_vad_prob < HALLUCINATION_TRUST_VAD
}

fn is_non_speech_annotation(inner: &str) -> bool {
    let lower = inner.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return true;
    }
    const CUES: &[&str] = &[
        "music",
        "singing",
        "song",
        "instrumental",
        "upbeat",
        "melody",
        "guitar",
        "piano",
        "drums",
        "beat",
        "blank_audio",
        "blank audio",
        "silence",
        "inaudible",
        "applause",
        "laughter",
        "laughing",
        "cheering",
        "noise",
        "static",
        "sigh",
        "cough",
        "throat",
        "panting",
        "groan",
        "beep",
        "buzz",
        "hum",
        "click",
        "typing",
        "foreign",
        "speaking",
        "audio",
    ];
    CUES.iter().any(|cue| lower.contains(cue))
}

fn strip_delimiters_if_cue(input: &str, open: char, close: char) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + open.len_utf8()..];
        if let Some(end) = after_open.find(close) {
            let inner = &after_open[..end];
            if !is_non_speech_annotation(inner) {
                out.push(open);
                out.push_str(inner);
                out.push(close);
            }
            rest = &after_open[end + close.len_utf8()..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

fn strip_musical_note_spans(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_note_span = false;
    for ch in input.chars() {
        if ch == '♪' || ch == '♫' {
            in_note_span = !in_note_span;
            continue;
        }
        if !in_note_span {
            out.push(ch);
        }
    }
    // If an odd number of ♪ appeared (for example a leading "♪ You're my heart"), drop the
    // trailing sung lyric if the whole line started with a musical note.
    if in_note_span && input.trim_start().starts_with(['♪', '♫']) {
        return String::new();
    }
    out
}

fn clean_whisper_hallucinations(input: &str) -> String {
    let trimmed = input.trim();
    let blocked_tokens = [
        "[BLANK_AUDIO]",
        "(blank audio)",
        "[ Silence ]",
        "[Silence]",
        "[silence]",
        "(silence)",
        "[INAUDIBLE]",
        "[inaudible]",
        "(inaudible)",
        "[MUSIC]",
        "[Music]",
    ];

    let mut result = trimmed.to_string();
    for token in &blocked_tokens {
        result = result.replace(token, "");
    }
    result = strip_musical_note_spans(&result);
    result = strip_delimiters_if_cue(&result, '[', ']');
    result = strip_delimiters_if_cue(&result, '(', ')');
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Speech models are downloaded from one fixed revision of the whisper.cpp model repository
// and must match these SHA-256 digests (taken from that repository's file metadata).
const MODEL_REPO_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

fn pinned_model_sha256(filename: &str) -> Option<&'static str> {
    match filename {
        "ggml-tiny.en.bin" => {
            Some("921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f")
        }
        "ggml-base.en.bin" => {
            Some("a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002")
        }
        "ggml-small.en.bin" => {
            Some("c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d")
        }
        "ggml-large-v3-turbo-q5_0.bin" => {
            Some("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2")
        }
        _ => None,
    }
}

fn file_sha256_hex(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect())
}

// Fail loudly: a download that is not the expected file is never installed.
fn verify_model_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let actual = file_sha256_hex(path)?;
    if actual != expected {
        return Err(
            "The downloaded speech model does not match its expected checksum. Nothing was installed. Please retry."
                .to_string(),
        );
    }
    Ok(())
}

// Reject error pages and obviously truncated responses before marking a model installed.
// This is a format sanity check. The checksum above is the authenticity check.
fn validate_downloaded_model(path: &Path) -> Result<(), String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let mut magic = [0; 4];
    file.read_exact(&mut magic)
        .map_err(|e| format!("Incomplete model download: {}", e))?;
    if size < 1_000_000 || magic != *b"lmgg" {
        return Err("Invalid or incomplete Whisper model download. Please retry.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_validation_rejects_partial_and_error_responses() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("saywhen-model-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.bin");
        std::fs::write(&path, b"lmgg").unwrap();
        assert!(validate_downloaded_model(&path).is_err());
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"html").unwrap();
        file.set_len(1_000_000).unwrap();
        assert!(validate_downloaded_model(&path).is_err());
        drop(file);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"lmgg").unwrap();
        file.set_len(1_000_000).unwrap();
        assert!(validate_downloaded_model(&path).is_ok());
        drop(file);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn model_checksum_accepts_only_the_expected_file() {
        let dir =
            std::env::temp_dir().join(format!("saywhen-checksum-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.bin");
        std::fs::write(&path, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(file_sha256_hex(&path).unwrap(), abc);
        assert!(verify_model_sha256(&path, abc).is_ok());
        std::fs::write(&path, b"abd").unwrap();
        assert!(verify_model_sha256(&path, abc).is_err());
        std::fs::remove_dir_all(dir).unwrap();
        for name in ["ggml-tiny.en.bin", "ggml-base.en.bin"] {
            assert_eq!(pinned_model_sha256(name).unwrap().len(), 64);
        }
        assert!(pinned_model_sha256("ggml-large.bin").is_none());
    }

    #[test]
    fn test_hallucination_cleaning() {
        assert_eq!(clean_whisper_hallucinations(" [BLANK_AUDIO] "), "");
        assert_eq!(
            clean_whisper_hallucinations("[BLANK_AUDIO] cargo test --release"),
            "cargo test --release"
        );
        assert_eq!(clean_whisper_hallucinations("(upbeat music)"), "");
        assert_eq!(
            clean_whisper_hallucinations("(upbeat music) Eevee, run the tests"),
            "Eevee, run the tests"
        );
        assert_eq!(
            clean_whisper_hallucinations("♪ You're on your own ♪ Eevee, git status"),
            "Eevee, git status"
        );
    }

    #[test]
    fn test_silence_hallucination_filter() {
        // Stock phrases on weak speech evidence are dropped.
        assert!(is_likely_silence_hallucination("Thank you.", 0.55));
        assert!(is_likely_silence_hallucination("Thank you.", 0.88));
        assert!(is_likely_silence_hallucination(
            " Thanks for watching! ",
            0.7
        ));
        assert!(is_likely_silence_hallucination("uh", 0.72));
        assert!(is_likely_silence_hallucination("hmm...", 0.65));
        assert!(is_likely_silence_hallucination("you", 0.99));
        // A confidently spoken "thank you" (measured >= 0.957 in corpus) is kept.
        assert!(!is_likely_silence_hallucination("Thank you.", 0.96));
        // Short real replies to coding agents are never dropped.
        assert!(!is_likely_silence_hallucination("yes", 0.5));
        assert!(!is_likely_silence_hallucination("no", 0.5));
        assert!(!is_likely_silence_hallucination("okay", 0.5));
        assert!(!is_likely_silence_hallucination("continue", 0.5));
        // Phrases inside a longer command are never touched.
        assert!(!is_likely_silence_hallucination(
            "thank you, now run cargo test",
            0.5
        ));
        assert!(!is_likely_silence_hallucination("", 0.1));
    }

    #[test]
    #[ignore = "requires a local Whisper model and working GPU; run explicitly on a desktop"]
    fn test_metal_whisper_pipeline_benchmark() {
        let engine = TranscriberEngine::new();
        let models = engine.list_models();
        let tiny = models.iter().find(|m| m.id == "tiny.en").unwrap();
        if tiny.installed {
            // 1.5 seconds of 16 kHz audio
            let samples = vec![0.0f32; 24_000];
            let cold = engine
                .transcribe_pcm(&samples, 0.85)
                .expect("Whisper Metal cold inference should succeed");
            let warm = engine
                .transcribe_pcm(&samples, 0.85)
                .expect("Whisper Metal warm inference should succeed");
            println!(
                "Verified Whisper Metal ({}) latency on 1500ms audio: cold {} ms (RTF {:.3}x), warm {} ms (RTF {:.3}x)",
                warm.model_used, cold.inference_ms, cold.rtf, warm.inference_ms, warm.rtf
            );
            assert_eq!(warm.audio_duration_ms, 1500);
        }
    }
}
