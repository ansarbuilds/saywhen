use ndarray::{Array1, Array2, Array3, ArrayD, Ix3};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::tensor::TensorElementType;
use ort::value::{Tensor, ValueType};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const PARAKEET_MODEL_ID: &str = "parakeet-tdt-0.6b-v2";
pub const PARAKEET_DIR_NAME: &str = "parakeet-tdt-0.6b-v2";
pub const PARAKEET_REPO_REVISION: &str = "0bbb45a3365852604aef28b538a8f066f4ccaa85";
pub const PARAKEET_APPROX_MB: u64 = 631;
const PARAKEET_MAX_TOKENS_PER_STEP: usize = 10;

pub const PARAKEET_FILES: [(&str, &str, u64); 4] = [
    (
        "nemo128.onnx",
        "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
        139_764,
    ),
    (
        "encoder-model.int8.onnx",
        "3e0581fda6ab843888b51e56d7ee78b6d5bc3237ec113af1f732d1d5286aa155",
        652_184_014,
    ),
    (
        "decoder_joint-model.int8.onnx",
        "a449f49acd68979d418651dd2dcb737cc0f1bf0225e009e29ee326354edbf7d3",
        8_998_286,
    ),
    (
        "vocab.txt",
        "ec182b70dd42113aff6c5372c75cac58c952443eb22322f57bbd7f53977d497d",
        9_384,
    ),
];

pub fn parakeet_model_dir(models_dir: &Path) -> PathBuf {
    models_dir.join(PARAKEET_DIR_NAME)
}

pub fn is_parakeet_installed(models_dir: &Path) -> bool {
    let dir = parakeet_model_dir(models_dir);
    PARAKEET_FILES.iter().all(|(name, _, expected_min_bytes)| {
        let p = dir.join(name);
        fs::metadata(&p)
            .map(|m| m.is_file() && m.len() >= *expected_min_bytes / 2)
            .unwrap_or(false)
    })
}

pub fn parakeet_installed_bytes(models_dir: &Path) -> u64 {
    let dir = parakeet_model_dir(models_dir);
    PARAKEET_FILES
        .iter()
        .map(|(name, _, _)| {
            let final_len = fs::metadata(dir.join(name)).map(|m| m.len()).unwrap_or(0);
            let part_len = fs::metadata(dir.join(format!("{name}.part")))
                .map(|m| m.len())
                .unwrap_or(0);
            final_len.max(part_len)
        })
        .sum()
}

fn verify_sha256(path: &Path, expected_hex: &str) -> Result<(), String> {
    let mut file =
        File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Failed reading {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    if actual != expected_hex {
        return Err(format!(
            "Checksum mismatch for {}: expected {}, got {}",
            path.display(),
            expected_hex,
            actual
        ));
    }
    Ok(())
}

pub fn download_parakeet_model(models_dir: &Path) -> Result<(), String> {
    let dir = parakeet_model_dir(models_dir);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create Parakeet model directory: {}", e))?;

    for (filename, expected_sha256, _) in PARAKEET_FILES {
        let dest = dir.join(filename);
        if dest.exists() && verify_sha256(&dest, expected_sha256).is_ok() {
            continue;
        }
        let temp = dir.join(format!("{filename}.part"));
        let _ = fs::remove_file(&temp);
        let url = format!(
            "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/{}/{}",
            PARAKEET_REPO_REVISION, filename
        );
        let res = (|| -> Result<(), String> {
            let status = Command::new("curl")
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
                .arg(&temp)
                .status()
                .map_err(|e| format!("Failed to run curl for {}: {}", filename, e))?;
            if !status.success() {
                return Err(format!(
                    "Download failed for {}. Check your internet connection and retry.",
                    filename
                ));
            }
            verify_sha256(&temp, expected_sha256)?;
            if dest.exists() {
                let _ = fs::remove_file(&dest);
            }
            fs::rename(&temp, &dest)
                .map_err(|e| format!("Failed to install {}: {}", filename, e))?;
            Ok(())
        })();
        let _ = fs::remove_file(&temp);
        res?;
    }
    Ok(())
}

pub fn parse_vocab_txt(raw: &str) -> Result<(Vec<String>, usize), String> {
    let mut entries: Vec<(usize, String)> = Vec::new();
    let mut blank_idx: Option<usize> = None;

    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((token, id_str)) = line.rsplit_once(' ') else {
            continue;
        };
        let id: usize = id_str
            .parse()
            .map_err(|e| format!("Invalid vocab id '{}' in line '{}': {}", id_str, line, e))?;
        if token == "<blk>" {
            blank_idx = Some(id);
        }
        entries.push((id, token.replace('\u{2581}', " ")));
    }

    if entries.is_empty() {
        return Err("Parakeet vocab.txt is empty".to_string());
    }
    let max_id = entries.iter().map(|(id, _)| *id).max().unwrap_or(0);
    let mut vocab = vec![String::new(); max_id + 1];
    for (id, tok) in entries {
        vocab[id] = tok;
    }
    let blank = blank_idx.ok_or_else(|| "Parakeet vocab.txt is missing <blk> token".to_string())?;
    Ok((vocab, blank))
}

pub fn decode_sentencepiece_tokens(tokens: &[usize], vocab: &[String]) -> String {
    let mut raw = String::new();
    for &id in tokens {
        if let Some(tok) = vocab.get(id) {
            if tok == "<unk>" || tok == "<blk>" {
                continue;
            }
            raw.push_str(tok);
        }
    }
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_session(path: &Path, intra_threads: usize) -> Result<Session, String> {
    Session::builder()
        .map_err(|e| format!("Failed to create ORT session builder: {}", e))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| format!("Failed to set ORT optimization level: {}", e))?
        .with_intra_threads(intra_threads)
        .map_err(|e| format!("Failed to set ORT intra threads: {}", e))?
        .with_inter_threads(1)
        .map_err(|e| format!("Failed to set ORT inter threads: {}", e))?
        .commit_from_file(path)
        .map_err(|e| format!("Failed to load ONNX model {}: {}", path.display(), e))
}

pub struct ParakeetModel {
    preprocessor: Session,
    encoder: Session,
    decoder_joint: Session,
    vocab: Vec<String>,
    blank_idx: usize,
    state_shape_0: usize,
    state_shape_2: usize,
    targets_is_i32: bool,
    target_len_is_i32: bool,
}

impl ParakeetModel {
    pub fn load(models_dir: &Path) -> Result<Self, String> {
        let dir = parakeet_model_dir(models_dir);
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 6))
            .unwrap_or(4);

        let vocab_raw = fs::read_to_string(dir.join("vocab.txt"))
            .map_err(|e| format!("Failed to read Parakeet vocab.txt: {}", e))?;
        let (vocab, blank_idx) = parse_vocab_txt(&vocab_raw)?;

        let preprocessor = build_session(&dir.join("nemo128.onnx"), 1)?;
        let encoder = build_session(&dir.join("encoder-model.int8.onnx"), n_threads)?;
        let decoder_joint = build_session(&dir.join("decoder_joint-model.int8.onnx"), 1)?;

        let mut state_shape_0 = 2usize;
        let mut state_shape_2 = 640usize;
        let mut targets_is_i32 = true;
        let mut target_len_is_i32 = true;

        for input in &decoder_joint.inputs {
            if let ValueType::Tensor { ty, shape, .. } = &input.input_type {
                if input.name == "input_states_1" && shape.len() == 3 {
                    if shape[0] > 0 {
                        state_shape_0 = shape[0] as usize;
                    }
                    if shape[2] > 0 {
                        state_shape_2 = shape[2] as usize;
                    }
                } else if input.name == "targets" {
                    targets_is_i32 = matches!(ty, TensorElementType::Int32);
                } else if input.name == "target_length" {
                    target_len_is_i32 = matches!(ty, TensorElementType::Int32);
                }
            }
        }

        Ok(Self {
            preprocessor,
            encoder,
            decoder_joint,
            vocab,
            blank_idx,
            state_shape_0,
            state_shape_2,
            targets_is_i32,
            target_len_is_i32,
        })
    }

    fn run_decoder_step(
        &mut self,
        enc_frame: Array3<f32>,
        last_tok: usize,
        state1: &ArrayD<f32>,
        state2: &ArrayD<f32>,
    ) -> Result<(Vec<f32>, ArrayD<f32>, ArrayD<f32>), String> {
        let targets_val: ort::session::SessionInputValue<'_> = if self.targets_is_i32 {
            Tensor::from_array(Array2::from_elem((1, 1), last_tok as i32))
                .map_err(|e| e.to_string())?
                .into()
        } else {
            Tensor::from_array(Array2::from_elem((1, 1), last_tok as i64))
                .map_err(|e| e.to_string())?
                .into()
        };
        let target_len_val: ort::session::SessionInputValue<'_> = if self.target_len_is_i32 {
            Tensor::from_array(Array1::from_vec(vec![1i32]))
                .map_err(|e| e.to_string())?
                .into()
        } else {
            Tensor::from_array(Array1::from_vec(vec![1i64]))
                .map_err(|e| e.to_string())?
                .into()
        };

        let dec_inputs = ort::inputs![
            "encoder_outputs" => Tensor::from_array(enc_frame).map_err(|e| e.to_string())?,
            "targets" => targets_val,
            "target_length" => target_len_val,
            "input_states_1" => Tensor::from_array(state1.clone()).map_err(|e| e.to_string())?,
            "input_states_2" => Tensor::from_array(state2.clone()).map_err(|e| e.to_string())?,
        ];
        let dec_out = self
            .decoder_joint
            .run(dec_inputs)
            .map_err(|e| format!("Parakeet decoder failed: {}", e))?;
        let logits: Vec<f32> = dec_out
            .get("outputs")
            .ok_or_else(|| "Parakeet decoder missing 'outputs'".to_string())?
            .try_extract_array::<f32>()
            .map_err(|e| format!("Failed extracting decoder logits: {}", e))?
            .iter()
            .copied()
            .collect();
        let s1 = dec_out
            .get("output_states_1")
            .ok_or_else(|| "Parakeet decoder missing 'output_states_1'".to_string())?
            .try_extract_array::<f32>()
            .map_err(|e| format!("Failed extracting output_states_1: {}", e))?
            .to_owned();
        let s2 = dec_out
            .get("output_states_2")
            .ok_or_else(|| "Parakeet decoder missing 'output_states_2'".to_string())?
            .try_extract_array::<f32>()
            .map_err(|e| format!("Failed extracting output_states_2: {}", e))?
            .to_owned();
        Ok((logits, s1, s2))
    }

    pub fn transcribe(&mut self, samples_16k: &[f32]) -> Result<String, String> {
        if samples_16k.is_empty() {
            return Ok(String::new());
        }

        let n_samples = samples_16k.len();
        let waveforms = Array2::from_shape_vec((1, n_samples), samples_16k.to_vec())
            .map_err(|e| format!("Failed to pack waveform tensor: {}", e))?;
        let waveforms_lens = Array1::from_vec(vec![n_samples as i64]);

        let (features_owned, features_lens_owned) = {
            let prep_inputs = ort::inputs![
                "waveforms" => Tensor::from_array(waveforms).map_err(|e| e.to_string())?,
                "waveforms_lens" => Tensor::from_array(waveforms_lens).map_err(|e| e.to_string())?,
            ];
            let prep_out = self
                .preprocessor
                .run(prep_inputs)
                .map_err(|e| format!("Parakeet preprocessor failed: {}", e))?;
            let features = prep_out
                .get("features")
                .ok_or_else(|| "Parakeet preprocessor missing 'features'".to_string())?
                .try_extract_array::<f32>()
                .map_err(|e| format!("Failed extracting features: {}", e))?
                .to_owned();
            let features_lens = prep_out
                .get("features_lens")
                .ok_or_else(|| "Parakeet preprocessor missing 'features_lens'".to_string())?
                .try_extract_array::<i64>()
                .map_err(|e| format!("Failed extracting features_lens: {}", e))?
                .to_owned();
            (features, features_lens)
        };

        let (encoder_time_major, encoded_len) = {
            let enc_inputs = ort::inputs![
                "audio_signal" => Tensor::from_array(features_owned).map_err(|e| e.to_string())?,
                "length" => Tensor::from_array(features_lens_owned).map_err(|e| e.to_string())?,
            ];
            let enc_out = self
                .encoder
                .run(enc_inputs)
                .map_err(|e| format!("Parakeet encoder failed: {}", e))?;
            let outputs = enc_out
                .get("outputs")
                .ok_or_else(|| "Parakeet encoder missing 'outputs'".to_string())?
                .try_extract_array::<f32>()
                .map_err(|e| format!("Failed extracting encoder outputs: {}", e))?
                .into_dimensionality::<Ix3>()
                .map_err(|e| format!("Unexpected encoder output rank: {}", e))?;
            let encoded_lengths = enc_out
                .get("encoded_lengths")
                .ok_or_else(|| "Parakeet encoder missing 'encoded_lengths'".to_string())?
                .try_extract_array::<i64>()
                .map_err(|e| format!("Failed extracting encoded_lengths: {}", e))?;

            let enc_len = (encoded_lengths[[0]].max(0) as usize).min(outputs.shape()[2]);
            let transposed = outputs.permuted_axes([0, 2, 1]).to_owned();
            (transposed, enc_len)
        };

        if encoded_len == 0 {
            return Ok(String::new());
        }

        let enc_dim = encoder_time_major.shape()[2];
        let mut state1: ArrayD<f32> =
            Array3::<f32>::zeros((self.state_shape_0, 1, self.state_shape_2)).into_dyn();
        let mut state2: ArrayD<f32> =
            Array3::<f32>::zeros((self.state_shape_0, 1, self.state_shape_2)).into_dyn();

        let mut tokens: Vec<usize> = Vec::new();
        let mut t = 0usize;
        let mut emitted_tokens = 0usize;
        let vocab_size = self.vocab.len();

        while t < encoded_len {
            let mut enc_frame = Array3::<f32>::zeros((1, enc_dim, 1));
            for d in 0..enc_dim {
                enc_frame[[0, d, 0]] = encoder_time_major[[0, t, d]];
            }
            let last_tok = tokens.last().copied().unwrap_or(self.blank_idx);

            let (logits_vec, next_s1, next_s2) =
                self.run_decoder_step(enc_frame, last_tok, &state1, &state2)?;

            if logits_vec.len() <= vocab_size {
                return Err(format!(
                    "Parakeet decoder output length {} <= vocab size {}",
                    logits_vec.len(),
                    vocab_size
                ));
            }

            let token = argmax(&logits_vec[..vocab_size]);
            let step = argmax(&logits_vec[vocab_size..]);

            if token != self.blank_idx {
                state1 = next_s1;
                state2 = next_s2;
                tokens.push(token);
                emitted_tokens += 1;
            }

            if step > 0 {
                t += step;
                emitted_tokens = 0;
            } else if token == self.blank_idx || emitted_tokens >= PARAKEET_MAX_TOKENS_PER_STEP {
                t += 1;
                emitted_tokens = 0;
            }
        }

        Ok(decode_sentencepiece_tokens(&tokens, &self.vocab))
    }
}

fn argmax(slice: &[f32]) -> usize {
    let mut best_idx = 0usize;
    let mut best_val = f32::NEG_INFINITY;
    for (idx, &val) in slice.iter().enumerate() {
        if val > best_val {
            best_val = val;
            best_idx = idx;
        }
    }
    best_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parakeet_vocab_parse_and_sentencepiece_decode() {
        let sample = "<unk> 0\n\u{2581}Hello 1\n, 2\n\u{2581}world 3\n! 4\n<blk> 5\n";
        let (vocab, blank) = parse_vocab_txt(sample).expect("vocab parses");
        assert_eq!(blank, 5);
        assert_eq!(vocab.len(), 6);
        let text = decode_sentencepiece_tokens(&[1, 2, 3, 4], &vocab);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn test_parakeet_pinned_sha256_entries() {
        assert_eq!(PARAKEET_FILES.len(), 4);
        for (name, sha, min_bytes) in PARAKEET_FILES {
            assert!(!name.is_empty());
            assert_eq!(sha.len(), 64);
            assert!(min_bytes > 0);
        }
    }

    #[test]
    #[ignore = "downloads/uses local Parakeet TDT 0.6B v2 ONNX model and runs real 15s speech benchmark"]
    fn test_parakeet_tdt_15s_speech_benchmark() {
        let models_dir = Path::new("../models");
        if !is_parakeet_installed(models_dir) {
            download_parakeet_model(models_dir).expect("Parakeet TDT model download should succeed");
        }
        assert!(is_parakeet_installed(models_dir));

        // Synthesize a >15-second developer monologue with macOS `say` to verify long-form accuracy & speed.
        let tmp_aiff = std::env::temp_dir().join("saywhen-parakeet-15s.aiff");
        let tmp_wav = std::env::temp_dir().join("saywhen-parakeet-15s.wav");
        let prompt = "Let's check the git status and run cargo test with release mode enabled. \
            If the speech pipeline goes past fifteen seconds, it should never cut off or lose the beginning of my sentence, \
            and Parakeet TDT should transcribe every word cleanly with proper punctuation and low latency.";
        let say_ok = Command::new("say")
            .args(["-o"])
            .arg(&tmp_aiff)
            .arg(prompt)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(say_ok, "macOS `say` should synthesize benchmark audio");
        let conv_ok = Command::new("afconvert")
            .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1"])
            .arg(&tmp_aiff)
            .arg(&tmp_wav)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(conv_ok, "afconvert should produce 16 kHz mono WAV");

        let (samples, rate) = crate::turn_taking::read_wav_mono_f32(&tmp_wav)
            .expect("WAV reader should decode benchmark clip");
        assert_eq!(rate, 16_000);
        let duration_ms = ((samples.len() as f64 / 16_000.0) * 1000.0) as u64;
        assert!(duration_ms >= 14_000, "expected >=14s audio, got {}ms", duration_ms);

        let t_load = std::time::Instant::now();
        let mut model = ParakeetModel::load(models_dir).expect("ParakeetModel::load should succeed");
        let load_ms = t_load.elapsed().as_millis();

        let t1 = std::time::Instant::now();
        let out1 = model.transcribe(&samples).expect("Cold Parakeet transcribe should succeed");
        let ms1 = t1.elapsed().as_millis();

        let t2 = std::time::Instant::now();
        let out2 = model.transcribe(&samples).expect("Warm Parakeet transcribe should succeed");
        let ms2 = t2.elapsed().as_millis();

        // Silence sanity check: should return empty or near-empty, never "Thank you."
        let silence = vec![0.0f32; 24_000];
        let sil_out = model.transcribe(&silence).expect("Silence transcribe should succeed");

        let _ = std::fs::remove_file(&tmp_aiff);
        let _ = std::fs::remove_file(&tmp_wav);

        println!(
            "Parakeet TDT 0.6B v2 ({} ms audio): load={} ms, run1={} ms (RTF {:.3}x), run2={} ms (RTF {:.3}x)\nTranscript: {}\nSilence output: {:?}",
            duration_ms,
            load_ms,
            ms1,
            ms1 as f64 / duration_ms as f64,
            ms2,
            ms2 as f64 / duration_ms as f64,
            out2,
            sil_out,
        );
        assert_eq!(out1, out2);
        let lower = out2.to_ascii_lowercase();
        assert!(lower.contains("git status"), "missing 'git status' in: {}", out2);
        assert!(lower.contains("cargo test"), "missing 'cargo test' in: {}", out2);
        assert!(lower.contains("fifteen seconds") || lower.contains("15 seconds"), "missing beginning/middle phrase in: {}", out2);
        assert!(lower.contains("low latency"), "missing ending phrase in: {}", out2);
    }
}
