use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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
    selected_model_id: Mutex<String>,
    app_models_dir: Option<PathBuf>,
    download_lock: Mutex<()>,
}

impl TranscriberEngine {
    pub fn new() -> Self {
        Self::with_app_data_dir(None)
    }

    pub fn with_app_data_dir(app_data_dir: Option<PathBuf>) -> Self {
        Self {
            download_lock: Mutex::new(()),
            loaded_model: Mutex::new(None),
            selected_model_id: Mutex::new("tiny.en".to_string()),
            app_models_dir: app_data_dir.map(|d| d.join("models")),
        }
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

    /// True when the selected model file is on disk. The model itself loads lazily on the
    /// first transcription, so `is_loaded` is false right after launch.
    pub fn is_model_available(&self) -> bool {
        let filename = match self.get_selected_model_id().as_str() {
            "base.en" => "ggml-base.en.bin",
            _ => "ggml-tiny.en.bin",
        };
        validate_downloaded_model(&self.resolve_models_dir().join(filename)).is_ok()
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded_model
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn unload(&self) {
        if let Ok(mut guard) = self.loaded_model.lock() {
            *guard = None;
        }
    }

    pub fn download_model(&self, model_id: &str) -> Result<ModelInfo, String> {
        let _download = self
            .download_lock
            .lock()
            .map_err(|_| "Model download lock failed")?;
        let filename = match model_id {
            "tiny.en" => "ggml-tiny.en.bin",
            "base.en" => "ggml-base.en.bin",
            _ => return Err(format!("Unsupported model id: {}", model_id)),
        };
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
        let filename = match selected_id.as_str() {
            "base.en" => "ggml-base.en.bin",
            _ => "ggml-tiny.en.bin",
        };
        let model_path = self.resolve_models_dir().join(filename);
        let ctx = self.ensure_context(&model_path)?;

        let start = Instant::now();
        let mut state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create WhisperState: {:?}", e))?;

        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 8) as i32)
            .unwrap_or(4);

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(n_threads);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(false);

        // Pad short utterances under 1.05s with trailing silence so Whisper mel window (>=1000ms) succeeds
        let min_samples = 16_800usize;
        let pcm_input: std::borrow::Cow<'_, [f32]> = if samples_16k.len() < min_samples {
            let mut padded = Vec::with_capacity(min_samples);
            padded.extend_from_slice(samples_16k);
            padded.resize(min_samples, 0.0);
            std::borrow::Cow::Owned(padded)
        } else {
            std::borrow::Cow::Borrowed(samples_16k)
        };

        state
            .full(params, &pcm_input)
            .map_err(|e| format!("Whisper inference failed: {:?}", e))?;

        let mut raw_text = String::new();
        for segment in state.as_iter() {
            if let Ok(seg_str) = segment.to_str_lossy() {
                raw_text.push_str(&seg_str);
            }
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
    result.trim().to_string()
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
            let res = engine
                .transcribe_pcm(&samples, 0.85)
                .expect("Whisper Metal inference should succeed");
            println!(
                "Verified Whisper Metal latency on 1500ms audio: {} ms (RTF {:.3}x)",
                res.inference_ms, res.rtf
            );
            assert_eq!(res.audio_duration_ms, 1500);
        }
    }
}
