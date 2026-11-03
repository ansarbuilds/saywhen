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
}

pub struct TranscriberEngine {
    loaded_model: Mutex<Option<(String, Arc<WhisperContext>)>>,
    selected_model_id: Mutex<String>,
}

impl TranscriberEngine {
    pub fn new() -> Self {
        Self {
            loaded_model: Mutex::new(None),
            selected_model_id: Mutex::new("tiny.en".to_string()),
        }
    }

    pub fn resolve_models_dir() -> PathBuf {
        let candidates = [
            PathBuf::from("models"),
            PathBuf::from("../models"),
            PathBuf::from("/Users/mohamedansar/Projects/saywhen/models"),
        ];
        for candidate in &candidates {
            if candidate.exists() && candidate.is_dir() {
                return candidate.clone();
            }
        }
        let fallback = PathBuf::from("../models");
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        let dir = Self::resolve_models_dir();
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
                let installed = full_path.exists();
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

    pub fn unload(&self) {
        if let Ok(mut guard) = self.loaded_model.lock() {
            *guard = None;
        }
    }

    pub fn download_model(&self, model_id: &str) -> Result<ModelInfo, String> {
        let filename = match model_id {
            "tiny.en" => "ggml-tiny.en.bin",
            "base.en" => "ggml-base.en.bin",
            _ => return Err(format!("Unsupported model id: {}", model_id)),
        };

        let dir = Self::resolve_models_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create models dir: {}", e))?;
        let dest = dir.join(filename);
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            filename
        );

        let status = std::process::Command::new("curl")
            .args(["-L", "-f", "-sS", &url, "-o"])
            .arg(&dest)
            .status()
            .map_err(|e| format!("Failed to run curl to download model: {}", e))?;

        if !status.success() {
            return Err(format!("Download failed for {}", url));
        }

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
        let model_path = Self::resolve_models_dir().join(filename);
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
        let (cleaned, end_phrase_triggered) = strip_end_phrase(&cleaned_raw);
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
        })
    }
}

fn strip_end_phrase(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    let stripped_punct = trimmed.trim_end_matches(|c: char| ".!,?;:".contains(c)).trim_end();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hallucination_cleaning() {
        assert_eq!(clean_whisper_hallucinations(" [BLANK_AUDIO] "), "");
        assert_eq!(
            clean_whisper_hallucinations("[BLANK_AUDIO] cargo test --release"),
            "cargo test --release"
        );
    }

    #[test]
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

