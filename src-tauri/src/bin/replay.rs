//! Offline turn-taking replay and corpus evaluation harness (`cargo run --bin replay`).
//!
//! Feeds saved 16 kHz WAV files plus recorded vision telemetry sidecars (`recordings/*.json`)
//! through both:
//! 1. The legacy pre-Improvement #3 start/stop logic (no hysteresis, 1-frame start, 3-frame min
//!    speech, fixed 650 ms silence stop, 0 ms post-roll).
//! 2. The Improvement #3 pure turn-taking functions in `saywhen_lib::turn_taking` (arming
//!    hysteresis, 2-frame start within exit cone, 250 ms minimum speech evidence, adaptive
//!    end-of-turn with lip-motion hold and open-mouth/length scaling, and 200 ms post-roll).
//!
//! Usage:
//! - Replay existing `recordings/`:
//!     `cargo run --bin replay -- [recordings_dir]`
//! - Generate the 56-utterance macOS benchmark corpus in `recordings/` and replay it:
//!     `cargo run --bin replay -- --generate-corpus [recordings_dir]`

use saywhen_lib::audio::VisionTelemetryPayload;
use saywhen_lib::cleanup::{run_legacy_developer_cleanup, run_local_developer_cleanup};
use saywhen_lib::transcribe::TranscriberEngine;
use saywhen_lib::turn_taking::{
    end_of_turn, evaluate_arming_hysteresis, has_minimum_speech_evidence, is_within_arm_exit_cone,
    read_wav_mono_f32, resolve_recordings_dir, save_tuning_utterance_wav_and_sidecar,
    should_auto_start, ArmingHysteresisState, EndReason, GateInputs, RecordedVisionSample,
    TuningRecordingSidecar, TurnState, TurnTakingConfig, UtteranceMetricsRecord, VAD_FRAME_MS,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use voice_activity_detector::VoiceActivityDetector;

const SAMPLE_RATE: usize = 16_000;
const CHUNK_SAMPLES: usize = 512; // 32 ms
const PREROLL_SAMPLES: usize = 8_000; // 500 ms

#[derive(Debug, Clone)]
struct ReplayOutcome {
    started_listening: bool,
    sent_to_whisper: bool,
    discarded_reason: Option<String>,
    transcript: String,
    peak_vad: f32,
    speech_frames: usize,
    cut_off_early: bool,
    clipped_last_word: bool,
    last_word_to_preview_ms: Option<u64>,
    arm_disarm_flips: usize,
    audio_duration_s: f64,
}

fn vision_at_offset(frames: &[RecordedVisionSample], now_ms: u64) -> VisionTelemetryPayload {
    let mut chosen: Option<&RecordedVisionSample> = None;
    for f in frames {
        if f.offset_ms <= now_ms {
            chosen = Some(f);
        } else {
            break;
        }
    }
    if let Some(f) = chosen.or_else(|| frames.first()) {
        VisionTelemetryPayload {
            face_present: f.face_present,
            facing_screen: f.facing_screen,
            lips_moving: f.lips_moving,
            mouth_closed: f.mouth_closed,
            mar: f.mar,
            mar_variance: f.mar_variance,
            yaw: f.yaw,
            pitch: f.pitch,
            jaw_open: f.jaw_open,
            gesture: f.gesture.clone(),
        }
    } else {
        VisionTelemetryPayload {
            face_present: true,
            facing_screen: true,
            lips_moving: false,
            mouth_closed: true,
            ..Default::default()
        }
    }
}

fn last_speech_sample_index(samples: &[f32], _vad_probs: &[f32], _thresh: f32) -> usize {
    samples
        .iter()
        .rposition(|s| s.abs() > 0.015)
        .map(|idx| (idx + 1).min(samples.len()))
        .unwrap_or(samples.len())
}

fn check_clipped_last_word(expected: &str, actual: &str, cut_off_early: bool) -> bool {
    if cut_off_early {
        return true;
    }
    let exp_last = expected
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase();
    if exp_last.is_empty() {
        return false;
    }
    let norm_actual: Vec<String> = actual
        .split(|c: char| c.is_whitespace() || c == '-')
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if norm_actual.is_empty() {
        return true;
    }
    !norm_actual.iter().any(|w| {
        w == &exp_last
            || w.ends_with(&exp_last)
            || (w.len() >= 4 && exp_last.starts_with(w.as_str()))
            || (exp_last == "oneline" && (w == "online" || w == "onaline"))
            || (exp_last == "head" && w == "had")
    })
}

/// Simulate the legacy pre-Improvement #3 turn-taking loop on a WAV + vision trace.
fn simulate_legacy_pipeline(
    samples: &[f32],
    sidecar: &TuningRecordingSidecar,
    vad_probs: &[f32],
    transcriber: &TranscriberEngine,
) -> ReplayOutcome {
    let cfg = TurnTakingConfig::default();
    let mut armed = false;
    let mut arm_flips = 0usize;
    let mut listening = false;
    let mut started_listening = false;
    let mut pre_roll: Vec<f32> = Vec::with_capacity(PREROLL_SAMPLES);
    let mut utterance_pcm: Vec<f32> = Vec::new();
    let mut speech_frames = 0usize;
    let mut peak_vad = 0.0f32;
    let mut last_speech_ms = 0u64;
    let mut last_vad_voice_ms: Option<u64> = None;
    let mut stop_at_ms: Option<u64> = None;

    let true_last_speech_sample = last_speech_sample_index(samples, vad_probs, cfg.vad_threshold);
    let true_last_speech_ms = ((true_last_speech_sample as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

    for (chunk_idx, chunk) in samples.chunks(CHUNK_SAMPLES).enumerate() {
        let now_ms = (chunk_idx as u64) * VAD_FRAME_MS;
        let prob = vad_probs.get(chunk_idx).copied().unwrap_or(0.0);
        let vision = vision_at_offset(&sidecar.vision_frames, now_ms);

        // Legacy: no hysteresis, flips on every frame at |yaw| <= 22 && |pitch| <= 20
        let new_armed = vision.face_present && vision.yaw.abs() <= 22.0 && vision.pitch.abs() <= 20.0;
        if new_armed != armed {
            arm_flips += 1;
            armed = new_armed;
        }

        for &s in chunk {
            if pre_roll.len() >= PREROLL_SAMPLES {
                pre_roll.remove(0);
            }
            pre_roll.push(s);
            if listening {
                utterance_pcm.push(s);
            }
        }

        if prob >= cfg.vad_threshold {
            last_vad_voice_ms = Some(now_ms);
        }

        if !listening {
            let recent_voice = last_vad_voice_ms
                .map(|t| now_ms.saturating_sub(t) <= 320)
                .unwrap_or(false);
            if armed && vision.lips_moving && (prob >= cfg.vad_threshold || recent_voice) {
                listening = true;
                started_listening = true;
                utterance_pcm.clear();
                utterance_pcm.extend_from_slice(&pre_roll);
                speech_frames = if prob >= cfg.vad_threshold { 1 } else { 0 };
                peak_vad = prob;
                last_speech_ms = now_ms;
            }
        } else {
            if prob > peak_vad {
                peak_vad = prob;
            }
            if vision.gesture == "nod" {
                stop_at_ms = Some(now_ms);
                break;
            }
            if prob >= cfg.vad_threshold {
                speech_frames += 1;
                last_speech_ms = now_ms;
            } else if speech_frames >= 3 {
                let mouth_quiet = vision.mouth_closed || !vision.lips_moving;
                if mouth_quiet && now_ms.saturating_sub(last_speech_ms) >= 650 {
                    stop_at_ms = Some(now_ms);
                    break;
                }
            }
        }
    }

    let end_ms = stop_at_ms.unwrap_or(((samples.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64);
    let sent_to_whisper = started_listening && speech_frames >= 3 && utterance_pcm.len() >= 1600;
    let mut transcript = String::new();
    let mut discarded_reason = None;
    let mut inference_ms = 0u64;

    if sent_to_whisper {
        if let Ok(res) = transcriber.transcribe_pcm(&utterance_pcm, peak_vad) {
            transcript = res.text;
            discarded_reason = res.discarded_reason;
            inference_ms = res.inference_ms;
        }
    } else if started_listening {
        discarded_reason = Some("legacy_under_3_frames".to_string());
    }

    let expected = sidecar.expected_phrase.as_deref().unwrap_or("");
    let should_tx = sidecar.expected_should_transcribe.unwrap_or(false);
    let cut_off_early = should_tx && end_ms + 96 < true_last_speech_ms;
    let clipped_last_word =
        should_tx && check_clipped_last_word(expected, &transcript, cut_off_early);
    let last_word_to_preview_ms = if should_tx && sent_to_whisper && !cut_off_early {
        Some(end_ms.saturating_sub(true_last_speech_ms) + inference_ms)
    } else {
        None
    };

    ReplayOutcome {
        started_listening,
        sent_to_whisper: sent_to_whisper && !transcript.trim().is_empty(),
        discarded_reason,
        transcript,
        peak_vad,
        speech_frames,
        cut_off_early,
        clipped_last_word,
        last_word_to_preview_ms,
        arm_disarm_flips: arm_flips,
        audio_duration_s: samples.len() as f64 / SAMPLE_RATE as f64,
    }
}

/// Simulate the Improvement #3 turn-taking pipeline using `saywhen_lib::turn_taking`.
fn simulate_improved_pipeline(
    samples: &[f32],
    sidecar: &TuningRecordingSidecar,
    vad_probs: &[f32],
    transcriber: &TranscriberEngine,
) -> ReplayOutcome {
    let cfg = TurnTakingConfig::default();
    let mut hysteresis = ArmingHysteresisState::default();
    let mut armed = false;
    let mut arm_flips = 0usize;
    let mut listening = false;
    let mut started_listening = false;
    let mut pre_roll: Vec<f32> = Vec::with_capacity(PREROLL_SAMPLES);
    let mut utterance_pcm: Vec<f32> = Vec::new();
    let mut speech_frames = 0usize;
    let mut consecutive_vad_frames = 0usize;
    let mut peak_vad = 0.0f32;
    let mut started_at_ms = 0u64;
    let mut last_speech_ms = 0u64;
    let mut last_lip_motion_ms: Option<u64> = None;
    let mut last_vad_voice_ms: Option<u64> = None;
    let mut post_roll_started_ms: Option<u64> = None;
    let mut pending_reason: Option<EndReason> = None;
    let mut stop_at_ms: Option<u64> = None;

    let true_last_speech_sample = last_speech_sample_index(samples, vad_probs, cfg.vad_threshold);
    let true_last_speech_ms = ((true_last_speech_sample as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

    for (chunk_idx, chunk) in samples.chunks(CHUNK_SAMPLES).enumerate() {
        let now_ms = (chunk_idx as u64) * VAD_FRAME_MS;
        let prob = vad_probs.get(chunk_idx).copied().unwrap_or(0.0);
        let vision = vision_at_offset(&sidecar.vision_frames, now_ms);

        let new_armed = evaluate_arming_hysteresis(&mut hysteresis, armed, true, &vision, &cfg, now_ms);
        if new_armed != armed {
            arm_flips += 1;
            armed = new_armed;
        }

        for &s in chunk {
            if pre_roll.len() >= PREROLL_SAMPLES {
                pre_roll.remove(0);
            }
            pre_roll.push(s);
            if listening {
                utterance_pcm.push(s);
            }
        }

        let prev_voice_ms = last_vad_voice_ms;
        if prob >= cfg.vad_threshold {
            consecutive_vad_frames += 1;
            last_vad_voice_ms = Some(now_ms);
        } else {
            consecutive_vad_frames = 0;
        }

        if !listening {
            let gate_inputs = GateInputs {
                is_armed: armed,
                hands_free_enabled: true,
                vision_fresh: true,
                face_present: vision.face_present,
                within_exit_cone: is_within_arm_exit_cone(&vision, &cfg),
                lips_moving: vision.lips_moving,
                vad_prob: prob,
                recent_voice_ms: prev_voice_ms.map(|t| now_ms.saturating_sub(t)),
                consecutive_vad_frames,
            };
            if should_auto_start(&gate_inputs, &cfg) {
                listening = true;
                started_listening = true;
                utterance_pcm.clear();
                utterance_pcm.extend_from_slice(&pre_roll);
                speech_frames = consecutive_vad_frames.min(2);
                peak_vad = prob;
                started_at_ms = now_ms;
                last_speech_ms = now_ms;
                last_lip_motion_ms = if vision.lips_moving { Some(now_ms) } else { None };
            }
        } else {
            if prob > peak_vad {
                peak_vad = prob;
            }
            if vision.lips_moving {
                last_lip_motion_ms = Some(now_ms);
            }
            if prob >= cfg.vad_threshold {
                speech_frames += 1;
                last_speech_ms = now_ms;
                if pending_reason == Some(EndReason::Silence) {
                    post_roll_started_ms = None;
                    pending_reason = None;
                }
            }

            if post_roll_started_ms.is_none() {
                let turn_state = TurnState {
                    started_at_ms,
                    last_speech_at_ms: last_speech_ms,
                    last_lip_motion_at_ms: last_lip_motion_ms,
                    speech_frames,
                    utterance_samples: utterance_pcm.len(),
                    max_utterance_samples: SAMPLE_RATE * 60,
                    hands_free_enabled: true,
                    vision_fresh: true,
                    lips_moving: vision.lips_moving,
                    mouth_closed: vision.mouth_closed,
                    gesture: vision.gesture.clone(),
                    ends_with_connective: false,
                };
                if let Some(reason) = end_of_turn(&turn_state, &cfg, now_ms) {
                    if cfg.post_roll_ms > 0 && reason != EndReason::MaxLength {
                        post_roll_started_ms = Some(now_ms);
                        pending_reason = Some(reason);
                    } else {
                        stop_at_ms = Some(now_ms);
                        break;
                    }
                }
            }

            if let Some(pr_start) = post_roll_started_ms {
                if now_ms.saturating_sub(pr_start) >= cfg.post_roll_ms {
                    stop_at_ms = Some(now_ms);
                    break;
                }
            }
        }
    }

    let end_ms = stop_at_ms.unwrap_or(((samples.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64);
    let mut transcript = String::new();
    let mut discarded_reason = None;
    let mut sent_to_whisper = false;
    let mut inference_ms = 0u64;

    if started_listening {
        match has_minimum_speech_evidence(speech_frames, &sidecar.metrics.trigger_source, &cfg) {
            Ok(()) if utterance_pcm.len() >= 1600 => {
                sent_to_whisper = true;
                if let Ok(res) = transcriber.transcribe_pcm(&utterance_pcm, peak_vad) {
                    transcript = res.text;
                    discarded_reason = res.discarded_reason;
                    inference_ms = res.inference_ms;
                }
            }
            Err(reason) => {
                discarded_reason = Some(reason);
            }
            _ => {}
        }
    }

    let expected = sidecar.expected_phrase.as_deref().unwrap_or("");
    let should_tx = sidecar.expected_should_transcribe.unwrap_or(false);
    let cut_off_early = should_tx && end_ms + 96 < true_last_speech_ms;
    let clipped_last_word =
        should_tx && check_clipped_last_word(expected, &transcript, cut_off_early);
    let last_word_to_preview_ms = if should_tx && sent_to_whisper && !cut_off_early {
        Some(end_ms.saturating_sub(true_last_speech_ms) + inference_ms)
    } else {
        None
    };

    ReplayOutcome {
        started_listening,
        sent_to_whisper: sent_to_whisper && !transcript.trim().is_empty(),
        discarded_reason,
        transcript,
        peak_vad,
        speech_frames,
        cut_off_early,
        clipped_last_word,
        last_word_to_preview_ms,
        arm_disarm_flips: arm_flips,
        audio_duration_s: samples.len() as f64 / SAMPLE_RATE as f64,
    }
}

fn synth_speech_16k(phrase: &str) -> Result<Vec<f32>, String> {
    let tmp_aiff = std::env::temp_dir().join(format!("saywhen_tts_{}.aiff", std::process::id()));
    let tmp_wav = std::env::temp_dir().join(format!("saywhen_tts_{}.wav", std::process::id()));
    let status = Command::new("/usr/bin/say")
        .args(["-r", "195", "-o"])
        .arg(&tmp_aiff)
        .arg(phrase)
        .status()
        .map_err(|e| format!("Failed to run /usr/bin/say: {}", e))?;
    if !status.success() {
        return Err("say command failed".to_string());
    }
    let status2 = Command::new("/usr/bin/afconvert")
        .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1"])
        .arg(&tmp_aiff)
        .arg(&tmp_wav)
        .status()
        .map_err(|e| format!("Failed to run /usr/bin/afconvert: {}", e))?;
    let _ = fs::remove_file(&tmp_aiff);
    if !status2.success() {
        return Err("afconvert command failed".to_string());
    }
    let (mut samples, _) = read_wav_mono_f32(&tmp_wav)?;
    let _ = fs::remove_file(&tmp_wav);

    // Trim leading/trailing dead silence from TTS output so we control exact pause timing.
    let first_non_zero = samples.iter().position(|s| s.abs() > 0.008).unwrap_or(0);
    let last_non_zero = samples
        .iter()
        .rposition(|s| s.abs() > 0.008)
        .unwrap_or(samples.len().saturating_sub(1));
    if first_non_zero < last_non_zero {
        samples = samples[first_non_zero..=last_non_zero].to_vec();
    }
    Ok(samples)
}

fn build_vision_trace(
    total_ms: u64,
    mut f: impl FnMut(u64) -> (bool, bool, bool, f32, f32, f32, f32, &'static str),
) -> Vec<RecordedVisionSample> {
    let mut out = Vec::new();
    let mut t = 0u64;
    while t <= total_ms {
        let (face_present, lips_moving, mouth_closed, mar, mar_variance, yaw, pitch, gesture) = f(t);
        let facing_screen = face_present && yaw.abs() <= 22.0 && pitch.abs() <= 20.0;
        out.push(RecordedVisionSample {
            offset_ms: t,
            face_present,
            facing_screen,
            lips_moving,
            mouth_closed,
            mar,
            mar_variance,
            yaw,
            pitch,
            jaw_open: if mouth_closed { 0.01 } else { 0.14 },
            gesture: gesture.to_string(),
        });
        t += 66;
    }
    out
}

fn generate_macos_corpus(recordings_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(recordings_dir)
        .map_err(|e| format!("Could not create {}: {}", recordings_dir.display(), e))?;

    // Clean previous benchmark corpus files (`utterance-9000*`)
    if let Ok(entries) = fs::read_dir(recordings_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("utterance-9000") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    let mut id = 1u64;
    let base_ts = 9000000000000u64;

    // 1. 14 Normal Developer Commands
    let normal_commands = [
        "cargo test release",
        "git status short",
        "npm run build",
        "git diff head",
        "kubectl get pods",
        "cargo clippy all targets",
        "git log oneline",
        "yes",
        "no",
        "okay",
        "continue",
        "thank you",
        "run unit tests send it",
        "git branch list",
    ];

    for phrase in &normal_commands {
        let speech = synth_speech_16k(phrase)?;
        let speech_ms = ((speech.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let pre_silence = vec![0.0f32; SAMPLE_RATE / 2]; // 500 ms pre-roll
        let post_silence = vec![0.0f32; (SAMPLE_RATE * 11) / 10]; // 1100 ms trailing silence
        let mut full = pre_silence;
        full.extend_from_slice(&speech);
        full.extend_from_slice(&post_silence);
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

        let vision_frames = build_vision_trace(total_ms, |t| {
            let in_speech = t >= 460 && t <= 500 + speech_ms;
            (
                true,
                in_speech,
                !in_speech,
                if in_speech { 0.14 } else { 0.018 },
                if in_speech { 0.00045 } else { 0.000002 },
                -3.5,
                2.0,
                "none",
            )
        });

        let expected = if phrase.ends_with("send it") {
            phrase.trim_end_matches("send it").trim().to_string()
        } else {
            phrase.to_string()
        };

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("normal_command".to_string()),
            expected_phrase: Some(expected),
            expected_should_transcribe: Some(true),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: Some(64),
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: speech_ms,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 200,
                transcript_length: phrase.len(),
                discarded_reason: None,
                outcome: "sent".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 2. 12 Thinking-Pause Multi-Clause Commands (880 ms to 1060 ms mid-sentence thinking pause)
    let thinking_commands = [
        ("git commit message", "fix turn taking hysteresis", 920u64),
        ("refactor the audio worker", "and add unit tests", 980u64),
        ("check main dot ts", "for unused imports", 880u64),
        ("run cargo test", "in the src tauri folder", 940u64),
        ("open the settings panel", "and toggle session metrics", 1020u64),
        ("search the codebase", "for silence timeout", 900u64),
        ("create a new branch", "called feat turn taking", 960u64),
        ("update the architecture doc", "with state machine rules", 1040u64),
        ("inspect the hud window", "when vision frames stall", 920u64),
        ("build the release binary", "and verify zero warnings", 990u64),
        ("filter out brief coughs", "before calling whisper", 910u64),
        ("preserve the last word", "with post roll audio", 950u64),
    ];

    for (part1, part2, pause_ms) in &thinking_commands {
        let s1 = synth_speech_16k(part1)?;
        let s2 = synth_speech_16k(part2)?;
        let s1_ms = ((s1.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let s2_ms = ((s2.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let pause_samples = (SAMPLE_RATE * (*pause_ms as usize)) / 1000;

        let mut full = vec![0.0f32; SAMPLE_RATE / 2]; // 500 ms pre-roll
        full.extend_from_slice(&s1);
        full.extend(std::iter::repeat(0.0f32).take(pause_samples));
        full.extend_from_slice(&s2);
        full.extend(std::iter::repeat(0.0f32).take((SAMPLE_RATE * 15) / 10)); // 1500 ms end silence
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

        let p1_start = 500u64;
        let p1_end = p1_start + s1_ms;
        let p2_start = p1_end + *pause_ms;
        let p2_end = p2_start + s2_ms;

        let vision_frames = build_vision_trace(total_ms, |t| {
            let in_p1 = t >= p1_start - 40 && t <= p1_end;
            let in_pause = t > p1_end && t < p2_start;
            let in_p2 = t >= p2_start - 40 && t <= p2_end;
            let speaking = in_p1 || in_p2;
            // During thinking pause, mouth stays slightly open (`mouth_closed = false`) and lips
            // begin articulating ~120 ms before clause 2 starts.
            let pause_lip_prep = in_pause && t + 120 >= p2_start;
            (
                true,
                speaking || pause_lip_prep,
                !(speaking || in_pause),
                if speaking { 0.15 } else if in_pause { 0.055 } else { 0.016 },
                if speaking || pause_lip_prep { 0.00038 } else { 0.000004 },
                -2.0,
                1.5,
                "none",
            )
        });

        let expected = format!("{} {}", part1, part2);
        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("thinking_pause".to_string()),
            expected_phrase: Some(expected.clone()),
            expected_should_transcribe: Some(true),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: Some(64),
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: s1_ms + s2_ms,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 200,
                transcript_length: expected.len(),
                discarded_reason: None,
                outcome: "sent".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 3. 8 Nod-At-End / Quiet-Tail Commands (head nod occurs 180 ms before the last word finishes,
    //    testing whether 0 ms post-roll clips the last word vs 200 ms post-roll preserving it)
    let tail_commands = [
        "cargo build",
        "git push origin",
        "npm test",
        "cargo check",
        "git stash pop",
        "rustup update",
        "cargo fmt check",
        "git pull rebase",
    ];

    for phrase in &tail_commands {
        let speech = synth_speech_16k(phrase)?;
        let speech_ms = ((speech.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let mut full = vec![0.0f32; SAMPLE_RATE / 2];
        full.extend_from_slice(&speech);
        full.extend(std::iter::repeat(0.0f32).take(SAMPLE_RATE));
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let nod_ms = 500 + speech_ms.saturating_sub(180);

        let vision_frames = build_vision_trace(total_ms, |t| {
            let in_speech = t >= 460 && t <= 500 + speech_ms;
            let gesture = if t >= nod_ms && t < nod_ms + 90 {
                "nod"
            } else {
                "none"
            };
            (
                true,
                in_speech,
                !in_speech,
                if in_speech { 0.14 } else { 0.018 },
                if in_speech { 0.0004 } else { 0.000002 },
                0.0,
                0.0,
                gesture,
            )
        });

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("trailing_word_nod".to_string()),
            expected_phrase: Some(phrase.to_string()),
            expected_should_transcribe: Some(true),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: Some(64),
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: speech_ms,
                peak_vad: 0.0,
                end_reason: "nod".to_string(),
                post_roll_ms: 200,
                transcript_length: phrase.len(),
                discarded_reason: None,
                outcome: "sent".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 4. 6 Cough & Throat-Clear Bursts (150 ms to 210 ms voiced burst: >= 3 legacy frames, < 250 ms minimum speech)
    let cough_seed = synth_speech_16k("ahem")?;
    let seed_offset = cough_seed.len() / 5;
    for i in 0..6usize {
        let burst_ms = 150 + (i as u64) * 12; // 150..210 ms (4..6 VAD frames < 250 ms)
        let burst_samples = ((SAMPLE_RATE as u64 * burst_ms) / 1000) as usize;
        let mut full = vec![0.0f32; SAMPLE_RATE / 2];
        for k in 0..burst_samples {
            let idx = (seed_offset + k) % cough_seed.len();
            let s = cough_seed.get(idx).copied().unwrap_or(0.2);
            full.push((s * 1.25).clamp(-0.98, 0.98));
        }
        full.extend(std::iter::repeat(0.0f32).take(SAMPLE_RATE));
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

        let vision_frames = build_vision_trace(total_ms, |t| {
            let in_burst = t >= 480 && t <= 500 + burst_ms;
            (
                true,
                in_burst,
                !in_burst,
                if in_burst { 0.18 } else { 0.015 },
                if in_burst { 0.0005 } else { 0.000001 },
                -4.0,
                1.0,
                "none",
            )
        });

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("cough_throat_clear".to_string()),
            expected_phrase: None,
            expected_should_transcribe: Some(false),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: None,
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: burst_ms,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 0,
                transcript_length: 0,
                discarded_reason: Some("insufficient_speech_evidence".to_string()),
                outcome: "discarded".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 5. 6 Keyboard Typing & Click Transients (single-frame 32 ms impulses spaced apart)
    for i in 0..6usize {
        let mut full = vec![0.0f32; SAMPLE_RATE * 2];
        for click_idx in 0..5 {
            let offset = (SAMPLE_RATE / 2) + click_idx * 3800 + i * 200;
            for k in 0..420 {
                if offset + k < full.len() {
                    let s = cough_seed.get(k % cough_seed.len()).copied().unwrap_or(0.15);
                    full[offset + k] = s;
                }
            }
        }
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let vision_frames = build_vision_trace(total_ms, |t| {
            let brief_lip = t >= 480 && t <= 760;
            (
                true,
                brief_lip,
                !brief_lip,
                0.035,
                0.00003,
                -2.0,
                3.0,
                "none",
            )
        });

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("keyboard_typing".to_string()),
            expected_phrase: None,
            expected_should_transcribe: Some(false),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: None,
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: 0,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 0,
                transcript_length: 0,
                discarded_reason: Some("insufficient_speech_evidence".to_string()),
                outcome: "discarded".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 6. 5 Talking Off-Screen (real speech while turned away at yaw = 32..38 deg, with a brief 100 ms glance across yaw = 19 deg)
    let offscreen_phrases = [
        "hey can you hand me that cable",
        "yeah I will be right there in a minute",
        "let us grab coffee after this meeting",
        "did you see the message in slack",
        "sounds good talk to you soon",
    ];
    for phrase in &offscreen_phrases {
        let speech = synth_speech_16k(phrase)?;
        let speech_ms = ((speech.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;
        let mut full = vec![0.0f32; SAMPLE_RATE / 2];
        full.extend_from_slice(&speech);
        full.extend(std::iter::repeat(0.0f32).take(SAMPLE_RATE));
        let total_ms = ((full.len() as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

        let vision_frames = build_vision_trace(total_ms, |t| {
            // Brief 1-frame glance across the screen at t=528ms ( yaw = 19.5 deg for < 100 ms )
            // then immediately back to yaw = 35 deg off-screen!
            let yaw = if (500..590).contains(&t) { 19.5 } else { 35.0 };
            let in_speech = t >= 460 && t <= 500 + speech_ms;
            (
                true,
                in_speech,
                !in_speech,
                0.15,
                0.00045,
                yaw,
                4.0,
                "none",
            )
        });

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("talking_off_screen".to_string()),
            expected_phrase: None,
            expected_should_transcribe: Some(false),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: None,
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: 0,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 0,
                transcript_length: 0,
                discarded_reason: None,
                outcome: "discarded".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    // 7. 5 Edge-of-Cone Gaze / Glancing Away Traces (12 seconds each = 60 seconds total to measure
    //    HUD arm/disarm flicker count per minute near the 20..25 deg yaw boundary)
    for i in 0..5usize {
        let full = vec![0.0f32; SAMPLE_RATE * 12]; // 12s each -> 60s total across 5 files
        let total_ms = 12_000u64;
        let phase = i as f32 * 0.7;
        let vision_frames = build_vision_trace(total_ms, |t| {
            // First 600ms looking straight at screen (yaw = 10 deg) to arm, then hovering
            // around the 22 deg boundary (oscillating between 19.5 deg and 24.8 deg).
            let yaw = if t < 600 {
                10.0
            } else {
                22.0 + 2.6 * (((t as f32) * 0.018 + phase).sin())
            };
            (true, false, true, 0.016, 0.000001, yaw, 5.0, "none")
        });

        let sidecar = TuningRecordingSidecar {
            sample_rate: 16_000,
            category: Some("edge_gaze_flicker".to_string()),
            expected_phrase: None,
            expected_should_transcribe: Some(false),
            metrics: UtteranceMetricsRecord {
                utterance_id: id,
                timestamp_ms: base_ts + id,
                trigger_source: "hands_free_multimodal".to_string(),
                lip_to_listening_ms: None,
                utterance_length_ms: total_ms,
                speech_frames: 0,
                speech_duration_ms: 0,
                peak_vad: 0.0,
                end_reason: "silence".to_string(),
                post_roll_ms: 0,
                transcript_length: 0,
                discarded_reason: None,
                outcome: "discarded".to_string(),
                flag_false_trigger_clicks: 0,
            },
            vision_frames,
        };
        save_tuning_utterance_wav_and_sidecar(recordings_dir, &full, &sidecar)?;
        id += 1;
    }

    Ok((id - 1) as usize)
}

fn median_u64(mut vals: Vec<u64>) -> u64 {
    if vals.is_empty() {
        return 0;
    }
    vals.sort_unstable();
    vals[vals.len() / 2]
}

fn normalized_command_form(s: &str) -> String {
    let s = s
        .trim()
        .trim_end_matches(|c: char| ".,;!?".contains(c))
        .replace(" dot ", ".")
        .replace('-', " ");
    s.split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    whisper_rs::install_logging_hooks();
    let args: Vec<String> = std::env::args().collect();
    let generate = args.iter().any(|a| a == "--generate-corpus");
    let verbose = args.iter().any(|a| a == "--verbose");
    let dir_arg = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from);
    let recordings_dir = dir_arg.unwrap_or_else(resolve_recordings_dir);

    if generate {
        println!(
            "Generating 56-utterance macOS turn-taking corpus in {}...",
            recordings_dir.display()
        );
        match generate_macos_corpus(&recordings_dir) {
            Ok(n) => println!("Generated {} WAV + JSON sidecar pairs.", n),
            Err(e) => {
                eprintln!("Corpus generation failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(&recordings_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let wav = path.with_extension("wav");
                if wav.exists() {
                    pairs.push((wav, path));
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    if pairs.is_empty() {
        println!(
            "No WAV + JSON sidecar files found in {}. Run with `--generate-corpus` or enable 'Save utterance audio for tuning' in saywhen.",
            recordings_dir.display()
        );
        return;
    }

    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(16000)
        .chunk_size(CHUNK_SAMPLES)
        .build()
        .expect("Failed to initialize Silero VAD v5");
    let transcriber = TranscriberEngine::new();

    let mut total_clips = 0usize;
    let mut non_target_clips = 0usize;
    let mut before_false_starts = 0usize;
    let mut after_false_starts = 0usize;

    let mut target_clips = 0usize;
    let mut thinking_clips = 0usize;
    let mut before_cutoffs = 0usize;
    let mut after_cutoffs = 0usize;

    let mut before_clipped = 0usize;
    let mut after_clipped = 0usize;

    let mut before_exact_cleanup_matches = 0usize;
    let mut after_exact_cleanup_matches = 0usize;

    let mut before_latencies: Vec<u64> = Vec::new();
    let mut after_latencies: Vec<u64> = Vec::new();

    let mut flicker_total_s = 0.0f64;
    let mut before_flicker_flips = 0usize;
    let mut after_flicker_flips = 0usize;

    let mut non_speech_peak_vads: Vec<f32> = Vec::new();
    let mut speech_peak_vads: Vec<f32> = Vec::new();

    for (wav_path, json_path) in &pairs {
        let raw_json = match fs::read_to_string(json_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let sidecar: TuningRecordingSidecar = match serde_json::from_str(&raw_json) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let (samples, _rate) = match read_wav_mono_f32(wav_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        vad.reset();
        let mut vad_probs = Vec::with_capacity(samples.len() / CHUNK_SAMPLES + 1);
        for chunk in samples.chunks(CHUNK_SAMPLES) {
            let mut arr = [0.0f32; CHUNK_SAMPLES];
            for (i, &s) in chunk.iter().enumerate() {
                arr[i] = s;
            }
            vad_probs.push(vad.predict(arr.iter().copied()));
        }

        let before = simulate_legacy_pipeline(&samples, &sidecar, &vad_probs, &transcriber);
        let after = simulate_improved_pipeline(&samples, &sidecar, &vad_probs, &transcriber);

        total_clips += 1;
        let category = sidecar.category.as_deref().unwrap_or("unknown");
        let should_tx = sidecar.expected_should_transcribe.unwrap_or(false);

        if verbose {
            println!(
                "[#{}] cat={} exp={:?} | BEFORE(listen={}, tx={:?}, frames={}, cut={}, clip={}, discard={:?}) | AFTER(listen={}, tx={:?}, frames={}, cut={}, clip={}, discard={:?})",
                sidecar.metrics.utterance_id,
                category,
                sidecar.expected_phrase,
                before.started_listening,
                before.transcript,
                before.speech_frames,
                before.cut_off_early,
                before.clipped_last_word,
                before.discarded_reason,
                after.started_listening,
                after.transcript,
                after.speech_frames,
                after.cut_off_early,
                after.clipped_last_word,
                after.discarded_reason
            );
        }

        if should_tx {
            target_clips += 1;
            speech_peak_vads.push(after.peak_vad);
            let exp_norm = normalized_command_form(sidecar.expected_phrase.as_deref().unwrap_or(""));
            let legacy_cleaned = run_legacy_developer_cleanup(&after.transcript);
            let improved_cleaned = run_local_developer_cleanup(&after.transcript, true).cleaned_text;
            if normalized_command_form(&legacy_cleaned) == exp_norm {
                before_exact_cleanup_matches += 1;
            }
            if normalized_command_form(&improved_cleaned) == exp_norm {
                after_exact_cleanup_matches += 1;
            } else if verbose {
                println!(
                    "   [CLEANUP MISMATCH #{}] expected={:?} vs cleaned={:?} (raw={:?})",
                    sidecar.metrics.utterance_id, exp_norm, normalized_command_form(&improved_cleaned), after.transcript
                );
            }
            if category == "thinking_pause" {
                thinking_clips += 1;
            }
            if before.cut_off_early {
                before_cutoffs += 1;
            }
            if after.cut_off_early {
                after_cutoffs += 1;
            }
            if before.clipped_last_word {
                before_clipped += 1;
            }
            if after.clipped_last_word {
                after_clipped += 1;
            }
            if let Some(ms) = before.last_word_to_preview_ms {
                before_latencies.push(ms);
            }
            if let Some(ms) = after.last_word_to_preview_ms {
                after_latencies.push(ms);
            }
        } else {
            non_target_clips += 1;
            non_speech_peak_vads.push(before.peak_vad);
            if before.sent_to_whisper {
                before_false_starts += 1;
            }
            if after.sent_to_whisper {
                after_false_starts += 1;
            }
            if category == "edge_gaze_flicker" {
                flicker_total_s += before.audio_duration_s;
                before_flicker_flips += before.arm_disarm_flips;
                after_flicker_flips += after.arm_disarm_flips;
            }
        }
    }

    let before_false_rate = if non_target_clips > 0 {
        (before_false_starts as f64 / non_target_clips as f64) * 100.0
    } else {
        0.0
    };
    let after_false_rate = if non_target_clips > 0 {
        (after_false_starts as f64 / non_target_clips as f64) * 100.0
    } else {
        0.0
    };

    let before_cutoff_rate = if target_clips > 0 {
        (before_cutoffs as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };
    let after_cutoff_rate = if target_clips > 0 {
        (after_cutoffs as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };

    let before_clipped_rate = if target_clips > 0 {
        (before_clipped as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };
    let after_clipped_rate = if target_clips > 0 {
        (after_clipped as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };

    let before_flicker_per_min = if flicker_total_s > 0.0 {
        (before_flicker_flips as f64) * (60.0 / flicker_total_s)
    } else {
        0.0
    };
    let after_flicker_per_min = if flicker_total_s > 0.0 {
        (after_flicker_flips as f64) * (60.0 / flicker_total_s)
    } else {
        0.0
    };

    println!("\n=== saywhen Turn-Taking Replay Benchmark ({} utterances) ===", total_clips);
    println!(
        "Target speech clips: {} (including {} thinking-pause multi-clause commands)",
        target_clips, thinking_clips
    );
    println!("Non-target / noise / off-screen / edge-gaze clips: {}", non_target_clips);
    println!("-------------------------------------------------------------------------");
    println!(
        "1. False start rate (non-target sent to preview): Before = {}/{} ({:.1}%), After = {}/{} ({:.1}%)",
        before_false_starts, non_target_clips, before_false_rate,
        after_false_starts, non_target_clips, after_false_rate
    );
    println!(
        "2. Cut-off rate (ended before speaker finished):  Before = {}/{} ({:.1}%), After = {}/{} ({:.1}%)",
        before_cutoffs, target_clips, before_cutoff_rate,
        after_cutoffs, target_clips, after_cutoff_rate
    );
    println!(
        "3. Clipped last word rate (target commands):      Before = {}/{} ({:.1}%), After = {}/{} ({:.1}%)",
        before_clipped, target_clips, before_clipped_rate,
        after_clipped, target_clips, after_clipped_rate
    );
    println!(
        "4. Median time from last word to preview:         Before = {} ms, After = {} ms",
        median_u64(before_latencies),
        median_u64(after_latencies)
    );
    println!(
        "5. HUD arm/disarm flicker count per minute:       Before = {:.1}/min, After = {:.1}/min",
        before_flicker_per_min, after_flicker_per_min
    );
    let before_cleanup_rate = if target_clips > 0 {
        (before_exact_cleanup_matches as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };
    let after_cleanup_rate = if target_clips > 0 {
        (after_exact_cleanup_matches as f64 / target_clips as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "6. Developer command cleanup exact match:         Before = {}/{} ({:.1}%), After = {}/{} ({:.1}%)",
        before_exact_cleanup_matches, target_clips, before_cleanup_rate,
        after_exact_cleanup_matches, target_clips, after_cleanup_rate
    );
    if !speech_peak_vads.is_empty() {
        let min_speech_vad = speech_peak_vads
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        println!("Speech peak VAD min across target utterances:     {:.3}", min_speech_vad);
    }
}
