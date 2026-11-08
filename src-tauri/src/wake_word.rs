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

/// Trigger source used for utterances started by voice alone.
pub const WAKE_WORD_TRIGGER: &str = "wake_word";

/// Default wake word. A short, unusual name triggers far less often than "alright" or "okay".
pub const DEFAULT_WAKE_WORD: &str = "eevee";

/// Audio (including the 500 ms pre-roll) needed before the early wake word check runs.
pub const WAKE_CHECK_MIN_SAMPLES: usize = 16_000 * 9 / 5; // 1.8 s
/// The early check only transcribes this much from the start of the candidate.
pub const WAKE_CHECK_MAX_SAMPLES: usize = 16_000 * 5 / 2; // 2.5 s
/// After a dropped candidate, wait for this much quiet before starting another one, so a
/// conversation in the room does not re-trigger transcription every couple of seconds.
pub const WAKE_REARM_QUIET_MS: u64 = 600;
/// Consecutive VAD speech frames (32 ms each) needed to start a candidate.
pub const WAKE_START_MIN_VAD_FRAMES: usize = 3;

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

fn aliases_for(wake_word: &str) -> Vec<Vec<String>> {
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
    // Longest first, so "e v" wins over a one-token prefix.
    list.sort_by_key(|a| std::cmp::Reverse(a.len()));
    list
}

/// If `text` starts with the wake word (after up to three filler words), returns the rest of the
/// text with the wake word and any following punctuation removed. Returns `None` otherwise.
/// The returned text may be empty when only the wake word was said.
pub fn strip_wake_word(text: &str, wake_word: &str) -> Option<String> {
    let tokens = tokenize(text);
    let aliases = aliases_for(wake_word);
    if aliases.is_empty() {
        return None;
    }
    for skip in 0..=MAX_LEADING_FILLERS.min(tokens.len()) {
        if skip > 0 && !FILLERS.contains(&tokens[skip - 1].0.as_str()) {
            break;
        }
        for alias in &aliases {
            let end = skip + alias.len();
            if end > tokens.len() {
                continue;
            }
            if tokens[skip..end]
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

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
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
    fn custom_wake_word_and_bare_wake_word() {
        assert_eq!(
            strip_wake_word("Computer, git status", "computer").as_deref(),
            Some("Git status")
        );
        assert_eq!(strip_wake_word("Eevee.", "eevee").as_deref(), Some(""));
        assert_eq!(effective_wake_word("  "), DEFAULT_WAKE_WORD);
    }
}
