//! Wake word fallback: lets the user talk to saywhen when the camera cannot see them
//! facing the screen (side-mounted camera, turned away, camera off).
//!
//! Any speech while saywhen is not armed becomes a candidate. The start of the candidate is
//! transcribed locally; if it does not begin with the wake word it is dropped silently.
//! If it does, the wake word is stripped and the rest goes through the normal check step.
//!
//! Matching is deliberately strict about position (the wake word must be the first real word,
//! after at most three filler words) and loose about spelling, because Whisper writes a spoken
//! name in many ways ("Eevee", "Evie", "EV", "IV").

use serde::{Deserialize, Serialize};

/// Trigger source used for utterances started by voice alone.
pub const WAKE_WORD_TRIGGER: &str = "wake_word";

/// Default 3 enrollment suffixes (combined with the active wake word, Siri-style).
pub const VOICE_ENROLL_COMMAND_SUFFIXES: [&str; 3] = [
    "run the tests",
    "check git status",
    "explain this error",
];

/// Personal acoustic + pronunciation profile learned from 3 short spoken phrases.
/// Contains only summary numbers and short wake-word alias tokens (never raw audio).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProfile {
    pub sample_count: u32,
    pub pitch_hz: f32,
    pub pitch_tolerance_hz: f32,
    pub typical_rms: f32,
    pub spectral_bands: [f32; 4],
    #[serde(default)]
    pub learned_wake_aliases: Vec<String>,
}

/// Default wake word. A short, unusual name triggers far less often than "alright" or "okay".
pub const DEFAULT_WAKE_WORD: &str = "eevee";

/// Audio (including the 500 ms pre-roll) needed before the early wake word check runs.
pub const WAKE_CHECK_MIN_SAMPLES: usize = 16_000 * 9 / 5; // 1.8 s
/// When the speaker pauses early, the check still waits for this much audio (pre-roll
/// included), so Whisper gets at least ~1 s of actual speech.
pub const WAKE_CHECK_MIN_SAMPLES_ON_PAUSE: usize = 16_000 * 3 / 2; // 1.5 s
/// The early check only transcribes this much from the start of the candidate.
pub const WAKE_CHECK_MAX_SAMPLES: usize = 16_000 * 5 / 2; // 2.5 s
/// After a dropped candidate, wait for this much quiet before starting another one, so a
/// conversation in the room does not re-trigger transcription every couple of seconds.
pub const WAKE_REARM_QUIET_MS: u64 = 600;
/// Maximum cooldown after a dropped wake candidate when continuous background audio (such as a
/// song on YouTube) prevents a 600 ms silent gap from ever occurring.
pub const WAKE_REARM_COOLDOWN_MS: u64 = 1_200;
/// Hard ceiling on `wake_needs_quiet` even if background audio stays at a constant level.
pub const WAKE_REARM_MAX_WAIT_MS: u64 = 2_400;
/// Consecutive VAD speech frames (32 ms each) needed to start a candidate.
pub const WAKE_START_MIN_VAD_FRAMES: usize = 3;
/// An unconfirmed candidate is not ended by silence for this long, so the early check gets
/// to run even when the user pauses right after the wake word.
pub const WAKE_UNCONFIRMED_HOLD_MS: u64 = 4_000;
/// After only the wake word was heard, wait this long in silence for the command.
pub const WAKE_COMMAND_WAIT_MS: u64 = 2_500;

/// The wake word path uses a lower voice-detector cut-off than the noise filter. Its own
/// "starts with the wake word" check drops anything else, and a voice aimed away from the
/// microphone (side-mounted laptop) often scores below the normal 0.45.
pub const WAKE_VAD_THRESHOLD_CAP: f32 = 0.3;

pub fn wake_vad_threshold(noise_filter: f32) -> f32 {
    noise_filter.min(WAKE_VAD_THRESHOLD_CAP)
}

/// A "something was heard" level for diagnostics: well below any speech threshold.
pub const WAKE_DIAG_SOUND_LEVEL: f32 = 0.12;

/// Wake word decision log: terminal always; development builds also append to
/// `<temp dir>/saywhen-wake.log` so the log survives however the app was started.
/// Lines never contain recognized text (except the dev-only first word).
pub fn wake_log_line(line: &str) {
    eprintln!("{line}");
    if cfg!(debug_assertions) {
        use std::io::Write;
        let path = std::env::temp_dir().join("saywhen-wake.log");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "{secs} {line}");
        }
    }
}

/// Words people say before the wake word ("hmm, alright, Eevee, ...").
const FILLERS: &[&str] = &[
    "hey", "hi", "hello", "ok", "okay", "alright", "all", "right", "so", "um", "uh", "hmm", "hm",
    "mm", "yo", "oh", "well",
];
const MAX_LEADING_FILLERS: usize = 3;

/// Known Whisper spellings of the default wake word.
const EEVEE_ALIASES: &[&str] = &[
    "eevee", "evie", "eevie", "evee", "eevy", "evey", "eevi", "ivy", "ev", "iv", "eve e", "e v",
];

fn tokenize(text: &str) -> Vec<(String, usize)> {
    // (lowercase letters-only token, byte offset just past the token in the original text)
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, ch) in text.char_indices() {
        if ch.is_alphanumeric() {
            cur.extend(ch.to_lowercase());
        } else if ch == '\'' || ch == '\u{2019}' {
            // keep "it's" together
        } else if !cur.is_empty() {
            out.push((std::mem::take(&mut cur), i));
        }
    }
    if !cur.is_empty() {
        out.push((cur, text.len()));
    }
    out
}

/// Rough sound key for the default wake word, so Whisper's many spellings of "ee-vee"
/// (Eevee, Evie, Evy, Ivy, Eve, Eevy, ...) all match without listing each one.
fn sounds_like_eevee(token: &str) -> bool {
    if !(2..=6).contains(&token.len()) || !token.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    let mut key = token.replace("ee", "i").replace("ea", "i").replace("ie", "i").replace("ei", "i");
    key = key.replace('y', "i").replace('e', "i");
    let mut collapsed = String::new();
    for c in key.chars() {
        if !collapsed.ends_with(c) {
            collapsed.push(c);
        }
    }
    collapsed == "ivi" || collapsed == "iv"
}

fn aliases_for_with_learned(wake_word: &str, learned: &[String]) -> Vec<Vec<String>> {
    let normalized: Vec<String> = tokenize(wake_word).into_iter().map(|(t, _)| t).collect();
    let mut list: Vec<Vec<String>> = Vec::new();
    if normalized.is_empty() {
        return list;
    }
    list.push(normalized.clone());
    if normalized.len() == 1 && normalized[0] == DEFAULT_WAKE_WORD {
        for a in EEVEE_ALIASES {
            list.push(a.split(' ').map(str::to_string).collect());
        }
    }
    for extra in learned {
        let toks: Vec<String> = tokenize(extra).into_iter().map(|(t, _)| t).collect();
        if !toks.is_empty() && toks.len() <= 3 && !list.contains(&toks) {
            list.push(toks);
        }
    }
    // Longest first, so "e v" wins over a one-token prefix.
    list.sort_by_key(|a| std::cmp::Reverse(a.len()));
    list
}

fn match_wake_word_at(
    text: &str,
    tokens: &[(String, usize)],
    start_idx: usize,
    aliases: &[Vec<String>],
    is_default: bool,
) -> Option<String> {
    let max_skip = MAX_LEADING_FILLERS.min(tokens.len().saturating_sub(start_idx));
    for skip in 0..=max_skip {
        let idx = start_idx + skip;
        if skip > 0 && !FILLERS.contains(&tokens[idx - 1].0.as_str()) {
            break;
        }
        for alias in aliases {
            let end = idx + alias.len();
            if end > tokens.len() {
                continue;
            }
            let fuzzy_single =
                alias.len() == 1 && is_default && sounds_like_eevee(&tokens[idx].0);
            if fuzzy_single
                || tokens[idx..end]
                    .iter()
                    .zip(alias.iter())
                    .all(|((tok, _), want)| tok == want)
            {
                let cut = tokens[end - 1].1;
                let rest = text[cut..]
                    .trim_start_matches(|c: char| !c.is_alphanumeric())
                    .trim_end()
                    .to_string();
                return Some(capitalize_first(&rest));
            }
        }
    }
    None
}

fn has_sentence_boundary_between(between: &str) -> bool {
    if between.contains('\n') {
        return true;
    }
    let mut chars = between.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '.' | '!' | '?')
            && chars.peek().map(|next| next.is_whitespace()).unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// If `text` starts with the wake word (after up to three filler words), or starts a new
/// sentence with the wake word after background audio/lyrics in the pre-roll, returns the rest
/// of the text with the wake word and any following punctuation removed. Returns `None` otherwise.
/// The returned text may be empty when only the wake word was said.
pub fn strip_wake_word(text: &str, wake_word: &str) -> Option<String> {
    strip_wake_word_with_profile(text, wake_word, None)
}

/// Like `strip_wake_word`, also matching any personal wake-word spellings learned during
/// the user's 3-phrase voice enrollment.
pub fn strip_wake_word_with_profile(
    text: &str,
    wake_word: &str,
    profile: Option<&VoiceProfile>,
) -> Option<String> {
    let tokens = tokenize(text);
    let learned = profile
        .map(|p| p.learned_wake_aliases.as_slice())
        .unwrap_or(&[]);
    let aliases = aliases_for_with_learned(wake_word, learned);
    if aliases.is_empty() || tokens.is_empty() {
        return None;
    }
    let is_default = tokenize(wake_word)
        .iter()
        .map(|(t, _)| t.as_str())
        .eq([DEFAULT_WAKE_WORD]);
    if let Some(rest) = match_wake_word_at(text, &tokens, 0, &aliases, is_default) {
        return Some(rest);
    }
    // If background media (such as a song on YouTube) was captured in the pre-roll before the
    // user spoke, Whisper often finishes that lyric with a sentence break and starts a new
    // sentence for the user's "Eevee, <command>".
    for i in 1..tokens.len() {
        let prev_end = tokens[i - 1].1;
        let cur_end = tokens[i].1;
        let cur_start = cur_end.saturating_sub(tokens[i].0.len()).max(prev_end);
        if has_sentence_boundary_between(&text[prev_end..cur_start]) {
            if let Some(rest) = match_wake_word_at(text, &tokens, i, &aliases, is_default) {
                return Some(rest);
            }
        }
    }
    None
}

/// Pure check for whether the post-drop `wake_needs_quiet` hold should clear.
///
/// Clears when:
/// 1. At least `WAKE_REARM_QUIET_MS` (`600 ms`) of audio below `vad_threshold` has elapsed, OR
/// 2. Continuous background audio (like a YouTube song) prevented a quiet gap, `WAKE_REARM_COOLDOWN_MS`
///    (`1,200 ms`) has passed since the drop, and a voice louder than the background floor arrives, OR
/// 3. `WAKE_REARM_MAX_WAIT_MS` (`2,400 ms`) has elapsed since the drop so the wake word is never
///    permanently locked out by a song.
pub fn should_clear_wake_quiet_wait(
    now_ms: u64,
    last_vad_voice_at_ms: Option<u64>,
    wake_dropped_at_ms: Option<u64>,
    prob: f32,
    vad_threshold: f32,
    rms: f32,
    wake_floor_rms: f32,
) -> bool {
    let quiet_gap_met = prob < vad_threshold
        && last_vad_voice_at_ms
            .map(|t| now_ms.saturating_sub(t) >= WAKE_REARM_QUIET_MS)
            .unwrap_or(true);
    if quiet_gap_met {
        return true;
    }

    let Some(dropped_at) = wake_dropped_at_ms else {
        return false;
    };
    let since_drop = now_ms.saturating_sub(dropped_at);
    if since_drop >= WAKE_REARM_MAX_WAIT_MS {
        return true;
    }
    if since_drop >= WAKE_REARM_COOLDOWN_MS {
        let louder_than_floor =
            wake_floor_rms > 0.0 && rms >= (wake_floor_rms * 1.35).max(0.015);
        if louder_than_floor || prob >= 0.72 {
            return true;
        }
    }
    false
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Words as the matcher counts them.
pub fn word_count(text: &str) -> usize {
    tokenize(text).len()
}

/// First word as the matcher sees it (lowercase letters and digits only). Development
/// builds log this for dropped attempts so a misheard wake word can be added; it is never
/// logged in release builds because it may be someone else talking.
pub fn first_word(text: &str) -> Option<String> {
    tokenize(text).into_iter().next().map(|(t, _)| t)
}

/// What the early check on the first couple of seconds decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarlyWakeVerdict {
    /// Starts with the wake word and the command already follows.
    Confirmed,
    /// Only the wake word so far; wait for the command.
    AwaitingCommand,
    /// Too little was heard to tell (Whisper often mangles a 1 s clip); let the full
    /// transcript decide.
    Unclear,
    /// Clearly someone talking without the wake word: drop it now.
    NotForUs,
}

pub fn early_wake_verdict(heard: &str, wake_word: &str) -> EarlyWakeVerdict {
    early_wake_verdict_with_profile(heard, wake_word, None)
}

pub fn early_wake_verdict_with_profile(
    heard: &str,
    wake_word: &str,
    profile: Option<&VoiceProfile>,
) -> EarlyWakeVerdict {
    match strip_wake_word_with_profile(heard, wake_word, profile) {
        Some(rest) if rest.trim().is_empty() => EarlyWakeVerdict::AwaitingCommand,
        Some(_) => EarlyWakeVerdict::Confirmed,
        None if tokenize(heard).len() >= 3 => EarlyWakeVerdict::NotForUs,
        None => EarlyWakeVerdict::Unclear,
    }
}

/// Extracts how Whisper spelled the user's spoken wake word at the start of an enrollment
/// phrase (for example when the prompt was `"Eevee, run the tests"` and Whisper transcribed
/// `"Heavy, run the tests"` -> learns `"heavy"`).
pub fn extract_leading_wake_alias(
    heard: &str,
    wake_word: &str,
    expected_command_suffix: &str,
) -> Option<String> {
    if strip_wake_word(heard, wake_word).is_some() {
        return None;
    }
    let tokens: Vec<String> = tokenize(heard).into_iter().map(|(t, _)| t).collect();
    if tokens.is_empty() {
        return None;
    }
    let suffix_tokens: Vec<String> = tokenize(expected_command_suffix)
        .into_iter()
        .map(|(t, _)| t)
        .collect();

    let mut start = 0usize;
    while start < tokens.len()
        && start < MAX_LEADING_FILLERS
        && FILLERS.contains(&tokens[start].as_str())
    {
        start += 1;
    }
    if start >= tokens.len() {
        return None;
    }

    // Find where the command suffix (or its first word) begins after the wake word.
    let first_suffix_word = suffix_tokens.first().map(|s| s.as_str()).unwrap_or("");
    let suffix_pos = if !first_suffix_word.is_empty() {
        tokens[start..]
            .iter()
            .position(|t| t == first_suffix_word)
            .map(|rel| start + rel)
    } else {
        None
    };

    let end = match suffix_pos {
        Some(pos) if pos > start && pos - start <= 2 => pos,
        _ => (start + 1).min(tokens.len()),
    };

    let candidate = tokens[start..end].join(" ");
    if candidate.len() < 2
        || (!first_suffix_word.is_empty() && candidate == first_suffix_word)
        || FILLERS.contains(&candidate.as_str())
    {
        return None;
    }
    Some(candidate)
}

fn goertzel_power(frame: &[f32], freq_hz: f32, sample_rate: f32) -> f32 {
    let k = (0.5 + (frame.len() as f32 * freq_hz) / sample_rate).floor();
    let omega = (2.0 * std::f32::consts::PI * k) / (frame.len() as f32);
    let coeff = 2.0 * omega.cos();
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    for &x in frame {
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0) / (frame.len() as f32)
}

fn estimate_frame_pitch_hz(frame: &[f32]) -> Option<f32> {
    // 16 kHz autocorrelation lag 42..=213 corresponds to ~380 Hz down to ~75 Hz.
    let energy: f32 = frame.iter().map(|s| s * s).sum();
    if energy < 1e-4 {
        return None;
    }
    let mut best_lag = 0usize;
    let mut best_corr = 0.0f32;
    for lag in 42..=213usize.min(frame.len().saturating_sub(1)) {
        let mut num = 0.0f32;
        let mut den_b = 0.0f32;
        let n = frame.len() - lag;
        for i in 0..n {
            let a = frame[i];
            let b = frame[i + lag];
            num += a * b;
            den_b += b * b;
        }
        let denom = (energy * den_b).sqrt();
        if denom > 1e-6 {
            let corr = num / denom;
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }
    }
    if best_corr >= 0.35 && best_lag > 0 {
        Some(16_000.0 / (best_lag as f32))
    } else {
        None
    }
}

/// Extracts `(median_pitch_hz, median_voiced_rms, normalized_4band_spectrum)` from a 16 kHz
/// mono speech recording.
pub fn extract_voice_features(samples_16k: &[f32]) -> Option<(f32, f32, [f32; 4])> {
    let mut rms_list = Vec::new();
    let mut pitch_list = Vec::new();
    let mut band_accum = [0.0f32; 4];
    let mut voiced_count = 0usize;

    let band_probes: [[f32; 3]; 4] = [
        [110.0, 180.0, 260.0],    // Fundamental / chest (80-300 Hz)
        [420.0, 650.0, 900.0],    // First formant F1 (300-1000 Hz)
        [1300.0, 1900.0, 2500.0], // Second formant F2 (1000-2800 Hz)
        [3400.0, 4600.0, 6000.0], // Presence / sibilance (2800-6500 Hz)
    ];

    for frame in samples_16k.chunks_exact(512) {
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / 512.0).sqrt();
        if rms < 0.010 {
            continue;
        }
        rms_list.push(rms);
        if let Some(f0) = estimate_frame_pitch_hz(frame) {
            pitch_list.push(f0);
        }
        for (b_idx, freqs) in band_probes.iter().enumerate() {
            let p: f32 = freqs
                .iter()
                .map(|&f| goertzel_power(frame, f, 16_000.0))
                .sum::<f32>()
                / (freqs.len() as f32);
            band_accum[b_idx] += (p + 1e-8).ln();
        }
        voiced_count += 1;
    }

    if voiced_count < 4 {
        return None;
    }

    rms_list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_rms = rms_list[rms_list.len() / 2];

    let median_pitch = if !pitch_list.is_empty() {
        pitch_list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        pitch_list[pitch_list.len() / 2]
    } else {
        155.0
    };

    for b in &mut band_accum {
        *b /= voiced_count as f32;
    }
    let min_b = band_accum
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    for b in &mut band_accum {
        *b = (*b - min_b) + 0.1;
    }
    let norm = band_accum.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for b in &mut band_accum {
        *b /= norm;
    }

    Some((median_pitch, median_rms, band_accum))
}

/// Merges a newly recorded enrollment phrase into an existing (or fresh) `VoiceProfile`.
pub fn merge_into_voice_profile(
    existing: Option<&VoiceProfile>,
    samples_16k: &[f32],
    learned_alias: Option<&str>,
) -> Result<VoiceProfile, String> {
    let (pitch_hz, rms, bands) = extract_voice_features(samples_16k).ok_or_else(|| {
        "Could not hear enough clear voice in that recording. Speak at your normal volume and try again."
            .to_string()
    })?;

    let mut profile = match existing {
        Some(prev) => {
            let n = prev.sample_count.max(1) as f32;
            let new_n = n + 1.0;
            let merged_pitch = (prev.pitch_hz * n + pitch_hz) / new_n;
            let pitch_diff = (pitch_hz - prev.pitch_hz).abs();
            let merged_tol = prev
                .pitch_tolerance_hz
                .max(pitch_diff * 1.35)
                .clamp(38.0, 95.0);
            let merged_rms = (prev.typical_rms * n + rms) / new_n;
            let mut merged_bands = [0.0f32; 4];
            for i in 0..4 {
                merged_bands[i] = (prev.spectral_bands[i] * n + bands[i]) / new_n;
            }
            let norm = merged_bands
                .iter()
                .map(|x| x * x)
                .sum::<f32>()
                .sqrt()
                .max(1e-6);
            for b in &mut merged_bands {
                *b /= norm;
            }
            VoiceProfile {
                sample_count: prev.sample_count.saturating_add(1),
                pitch_hz: (merged_pitch * 10.0).round() / 10.0,
                pitch_tolerance_hz: (merged_tol * 10.0).round() / 10.0,
                typical_rms: (merged_rms * 10000.0).round() / 10000.0,
                spectral_bands: merged_bands,
                learned_wake_aliases: prev.learned_wake_aliases.clone(),
            }
        }
        None => VoiceProfile {
            sample_count: 1,
            pitch_hz: (pitch_hz * 10.0).round() / 10.0,
            pitch_tolerance_hz: 45.0,
            typical_rms: (rms * 10000.0).round() / 10000.0,
            spectral_bands: bands,
            learned_wake_aliases: Vec::new(),
        },
    };

    if let Some(alias) = learned_alias {
        let clean = alias.trim().to_ascii_lowercase();
        if !clean.is_empty()
            && !profile.learned_wake_aliases.contains(&clean)
            && profile.learned_wake_aliases.len() < 8
        {
            profile.learned_wake_aliases.push(clean);
        }
    }

    Ok(profile)
}

/// Checks whether a candidate audio clip matches the user's enrolled `VoiceProfile`
/// (timbre cosine similarity, pitch range, and near-field loudness).
pub fn matches_voice_profile(samples_16k: &[f32], profile: &VoiceProfile) -> bool {
    let Some((pitch_hz, rms, bands)) = extract_voice_features(samples_16k) else {
        return false;
    };
    if profile.typical_rms > 0.0 && rms < (profile.typical_rms * 0.25).max(0.008) {
        return false;
    }
    let pitch_limit = (profile.pitch_tolerance_hz * 2.2).max(65.0);
    if (pitch_hz - profile.pitch_hz).abs() > pitch_limit {
        return false;
    }
    let dot: f32 = bands
        .iter()
        .zip(profile.spectral_bands.iter())
        .map(|(a, b)| a * b)
        .sum();
    dot >= 0.78
}

/// Whether silence should be ignored for now on a wake word candidate.
pub fn wake_holds_turn(
    confirmed: bool,
    awaiting_command: bool,
    listening_ms: u64,
    silence_ms: u64,
) -> bool {
    if !confirmed {
        return listening_ms < WAKE_UNCONFIRMED_HOLD_MS;
    }
    awaiting_command && silence_ms < WAKE_COMMAND_WAIT_MS
}

/// Wake word to use, falling back to the default for an empty setting.
pub fn effective_wake_word(configured: &str) -> &str {
    if configured.trim().is_empty() {
        DEFAULT_WAKE_WORD
    } else {
        configured.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_whisper_spellings_of_default() {
        for heard in [
            "Eevee, run the tests.",
            "Evie run the tests",
            "EV, run the tests.",
            "E.V. run the tests",
            "IV. Run the tests.",
            "Hmm, alright, Eevee, run the tests.",
            "Hmm, all right, Eevee, run the tests.",
            "Hey Eevee - run the tests",
        ] {
            assert_eq!(
                strip_wake_word(heard, "eevee")
                    .map(|s| s.trim_end_matches('.').to_string())
                    .as_deref(),
                Some("Run the tests"),
                "heard: {heard}"
            );
        }
    }

    #[test]
    fn ignores_speech_without_wake_word() {
        for heard in [
            "Every time I run the tests they fail.",
            "Let me ask Eevee later.",
            "So anyway, okay, Eevee was great.",
            "Event loop is blocked",
            "",
        ] {
            assert_eq!(strip_wake_word(heard, "eevee"), None, "heard: {heard}");
        }
    }

    #[test]
    fn sound_alike_spellings_match_but_common_words_do_not() {
        for heard in ["Evy, run it", "Eve, run it", "Eevy run it", "Eevie, run it"] {
            assert!(strip_wake_word(heard, "eevee").is_some(), "heard: {heard}");
        }
        for heard in ["Even so", "Evil plan", "Eva said", "Ever wonder", "Eleven tests"] {
            assert!(strip_wake_word(heard, "eevee").is_none(), "heard: {heard}");
        }
        // A custom word is matched exactly, without the Eevee sound key.
        assert!(strip_wake_word("Evie, run it", "computer").is_none());
    }

    #[test]
    fn early_verdicts_and_turn_hold() {
        use EarlyWakeVerdict::*;
        assert_eq!(early_wake_verdict("Eevee.", "eevee"), AwaitingCommand);
        assert_eq!(early_wake_verdict("Eevee, run the", "eevee"), Confirmed);
        assert_eq!(early_wake_verdict("", "eevee"), Unclear);
        assert_eq!(early_wake_verdict("TV.", "eevee"), Unclear);
        assert_eq!(early_wake_verdict("so I told him that", "eevee"), NotForUs);
        // Pause right after the wake word, before the check ran: keep listening.
        assert!(wake_holds_turn(false, false, 1_200, 900));
        assert!(!wake_holds_turn(false, false, 4_500, 900));
        // Only the wake word so far: wait for the command, but not forever.
        assert!(wake_holds_turn(true, true, 2_000, 1_500));
        assert!(!wake_holds_turn(true, true, 5_000, 2_600));
        // Command under way: normal end of turn.
        assert!(!wake_holds_turn(true, false, 3_000, 900));
    }

    #[test]
    fn wake_threshold_is_lower_but_never_above_the_noise_filter() {
        assert_eq!(wake_vad_threshold(0.45), WAKE_VAD_THRESHOLD_CAP);
        assert_eq!(wake_vad_threshold(0.25), 0.25);
    }

    #[test]
    fn custom_wake_word_and_bare_wake_word() {
        assert_eq!(
            strip_wake_word("Computer, git status", "computer").as_deref(),
            Some("Git status")
        );
        assert_eq!(strip_wake_word("Eevee.", "eevee").as_deref(), Some(""));
        assert_eq!(effective_wake_word("  "), DEFAULT_WAKE_WORD);
    }

    #[test]
    fn wake_word_after_background_song_sentence_and_quiet_wait_recovery() {
        // Song lyric in pre-roll ends a sentence before the user speaks "Eevee, run the tests".
        assert_eq!(
            strip_wake_word("You're on your own. Eevee, run the tests.", "eevee")
                .map(|s| s.trim_end_matches('.').to_string())
                .as_deref(),
            Some("Run the tests")
        );
        assert_eq!(
            strip_wake_word("I feel it coming! Hey Eevee, cargo check", "eevee").as_deref(),
            Some("Cargo check")
        );

        // During a continuous song (no 600 ms silence gap), `wake_needs_quiet` must not lock out
        // forever: it clears once cooldown passes and a louder near-field voice arrives, or at max wait.
        assert!(!should_clear_wake_quiet_wait(
            1_500,
            Some(1_450),
            Some(1_000),
            0.55,
            0.45,
            0.020,
            0.020
        ));
        assert!(should_clear_wake_quiet_wait(
            2_300,
            Some(2_280),
            Some(1_000),
            0.60,
            0.45,
            0.045,
            0.020
        ));
        assert!(should_clear_wake_quiet_wait(
            3_500,
            Some(3_480),
            Some(1_000),
            0.50,
            0.45,
            0.018,
            0.020
        ));
    }

    #[test]
    fn voice_profile_enrollment_and_matching() {
        let synth_voice = |f0: f32, amp: f32| -> Vec<f32> {
            (0..16_000)
                .map(|i| {
                    let t = i as f32 / 16_000.0;
                    amp * ((2.0 * std::f32::consts::PI * f0 * t).sin() * 0.65
                        + (2.0 * std::f32::consts::PI * (f0 * 2.0) * t).sin() * 0.25
                        + (2.0 * std::f32::consts::PI * (f0 * 3.0) * t).sin() * 0.10)
                })
                .collect()
        };

        let sample_1 = synth_voice(135.0, 0.08);
        let sample_2 = synth_voice(142.0, 0.09);
        let alias = extract_leading_wake_alias(
            "Heavey, run the tests.",
            "eevee",
            VOICE_ENROLL_COMMAND_SUFFIXES[0],
        );
        assert_eq!(alias.as_deref(), Some("heavey"));

        let p1 = merge_into_voice_profile(None, &sample_1, alias.as_deref()).unwrap();
        let p2 = merge_into_voice_profile(Some(&p1), &sample_2, None).unwrap();
        assert_eq!(p2.sample_count, 2);
        assert!((p2.pitch_hz - 138.5).abs() < 10.0);
        assert_eq!(p2.learned_wake_aliases, vec!["heavey".to_string()]);

        // Same speaker matches; distant/high-pitched background vocalist is rejected.
        assert!(matches_voice_profile(&synth_voice(138.0, 0.07), &p2));
        assert!(!matches_voice_profile(&synth_voice(275.0, 0.07), &p2));
        assert!(!matches_voice_profile(&synth_voice(138.0, 0.004), &p2));

        // Personal wake-word spelling learned during enrollment strips cleanly.
        assert_eq!(
            strip_wake_word_with_profile("Heavey, check git status", "eevee", Some(&p2)).as_deref(),
            Some("Check git status")
        );
    }
}
