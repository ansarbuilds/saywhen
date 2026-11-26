//! Desk features that sit beside the live audio worker: quiet hours, spoken
//! punctuation, settings bundles, history stats, model advice, and a local
//! diagnostic note list. None of these send data anywhere.

use crate::audio::UtteranceCompleteEvent;
use crate::config::DictationConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SETTINGS_BUNDLE_VERSION: u32 = 1;
pub const DIAGNOSTIC_CAP: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

impl UpdateChannel {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "beta" => UpdateChannel::Beta,
            _ => UpdateChannel::Stable,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Beta => "beta",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HudAnchor {
    BottomCenter,
    BottomLeft,
    BottomRight,
    TopCenter,
}

impl HudAnchor {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bottom-left" => HudAnchor::BottomLeft,
            "bottom-right" => HudAnchor::BottomRight,
            "top-center" => HudAnchor::TopCenter,
            _ => HudAnchor::BottomCenter,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            HudAnchor::BottomCenter => "bottom-center",
            HudAnchor::BottomLeft => "bottom-left",
            HudAnchor::BottomRight => "bottom-right",
            HudAnchor::TopCenter => "top-center",
        }
    }
}

/// True when `minute` (0 to 1439) falls inside the quiet window. A window that
/// crosses midnight, such as 22:00 to 07:00, is included.
pub fn in_quiet_hours(start_minute: u16, end_minute: u16, minute: u16) -> bool {
    let minute = minute.min(1439);
    let start = start_minute.min(1439);
    let end = end_minute.min(1439);
    if start == end {
        return false;
    }
    if start < end {
        minute >= start && minute < end
    } else {
        minute >= start || minute < end
    }
}

pub fn clamp_minute(value: u16) -> u16 {
    value.min(1439)
}

/// Turns a few spoken marks into characters. Only whole words change.
pub fn apply_spoken_punctuation(text: &str) -> String {
    let mut out = String::new();
    for (index, word) in text.split_whitespace().enumerate() {
        let (bare, suffix) = split_trailing_punct(word);
        let replacement = match bare.to_ascii_lowercase().as_str() {
            "period" | "fullstop" => ".",
            "comma" => ",",
            "questionmark" => "?",
            "exclamation" => "!",
            "colon" => ":",
            "semicolon" => ";",
            "newline" | "newparagraph" => "\n",
            _ => {
                if index > 0 && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push_str(word);
                continue;
            }
        };
        if replacement == "\n" {
            out.push('\n');
        } else {
            out.push_str(replacement);
            out.push_str(suffix);
        }
    }
    out
}

fn split_trailing_punct(word: &str) -> (&str, &str) {
    let end = word.trim_end_matches(|ch: char| matches!(ch, '.' | ',' | '!' | '?' | ':' | ';'));
    (&word[..end.len()], &word[end.len()..])
}

pub fn capitalize_sentences(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut cap_next = true;
    for ch in &mut chars {
        if cap_next && ch.is_alphabetic() {
            *ch = ch.to_uppercase().next().unwrap_or(*ch);
            cap_next = false;
        } else if *ch == '.' || *ch == '!' || *ch == '?' || *ch == '\n' {
            cap_next = true;
        } else if !ch.is_whitespace() {
            cap_next = false;
        }
    }
    chars.into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBundle {
    pub version: u32,
    pub config: DictationConfig,
}

pub fn export_settings_bundle(config: &DictationConfig) -> String {
    let mut config = config.clone();
    config.save_tuning_audio_enabled = false;
    config.license_endpoint = config.license_endpoint.trim().to_string();
    let bundle = SettingsBundle {
        version: SETTINGS_BUNDLE_VERSION,
        config,
    };
    serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string())
}

pub fn import_settings_bundle(raw: &str) -> Result<DictationConfig, String> {
    let bundle: SettingsBundle = serde_json::from_str(raw)
        .map_err(|err| format!("That settings file could not be read: {err}"))?;
    if bundle.version == 0 || bundle.version > SETTINGS_BUNDLE_VERSION {
        return Err(format!(
            "This settings file is version {}, and this app reads version {SETTINGS_BUNDLE_VERSION}.",
            bundle.version
        ));
    }
    let mut config = bundle.config;
    config.sanitize_after_parse();
    if raw.contains("llmApiKey") || raw.contains("sk-") {
        return Err("Settings files must not contain an API key.".to_string());
    }
    Ok(config)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskStats {
    pub utterances: usize,
    pub words: usize,
    pub canceled: usize,
    pub top_app: String,
    pub top_app_count: usize,
}

pub fn summarize_history(entries: &[UtteranceCompleteEvent]) -> DeskStats {
    let mut apps: BTreeMap<String, usize> = BTreeMap::new();
    let mut words = 0usize;
    let mut canceled = 0usize;
    for entry in entries {
        let text = entry.cleanup.cleaned_text.trim();
        if !text.is_empty() {
            words += text.split_whitespace().count();
        }
        if entry.injection_error.is_some()
            || entry
                .injection
                .as_ref()
                .is_some_and(|outcome| !outcome.success)
        {
            canceled += 1;
        }
        let app = entry.target_policy.app_name.trim();
        if !app.is_empty() {
            *apps.entry(app.to_string()).or_insert(0) += 1;
        }
    }
    let (top_app, top_app_count) = apps
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap_or_else(|| (String::new(), 0));
    DeskStats {
        utterances: entries.len(),
        words,
        canceled,
        top_app,
        top_app_count,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPreference {
    Speed,
    Balanced,
    Accuracy,
}

impl ModelPreference {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "speed" | "fast" => ModelPreference::Speed,
            "accuracy" | "accurate" => ModelPreference::Accuracy,
            _ => ModelPreference::Balanced,
        }
    }
}

/// Picks an installed model. Parakeet is the balanced default when it is present.
/// This is a preference, not a measured accuracy ranking.
pub fn advise_model(installed: &[String], preference: ModelPreference) -> Option<String> {
    let has = |id: &str| installed.iter().any(|item| item == id);
    let order: &[&str] = match preference {
        ModelPreference::Speed => &[
            "tiny.en",
            "parakeet-tdt-0.6b-v2",
            "base.en",
            "small.en",
            "large-v3-turbo",
        ],
        ModelPreference::Balanced => &[
            "parakeet-tdt-0.6b-v2",
            "base.en",
            "small.en",
            "large-v3-turbo",
            "tiny.en",
        ],
        ModelPreference::Accuracy => &[
            "large-v3-turbo",
            "parakeet-tdt-0.6b-v2",
            "small.en",
            "base.en",
            "tiny.en",
        ],
    };
    order
        .iter()
        .find(|id| has(id))
        .map(|id| (*id).to_string())
        .or_else(|| installed.first().cloned())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticNote {
    pub at_ms: u64,
    pub message: String,
}

pub fn push_diagnostic(notes: &mut Vec<DiagnosticNote>, at_ms: u64, message: &str) {
    let message = redact_note(message);
    if message.is_empty() {
        return;
    }
    notes.push(DiagnosticNote { at_ms, message });
    if notes.len() > DIAGNOSTIC_CAP {
        let extra = notes.len() - DIAGNOSTIC_CAP;
        notes.drain(0..extra);
    }
}

pub fn redact_note(message: &str) -> String {
    let mut out = String::new();
    for word in message.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let secret = lower.starts_with("sk-")
            || lower.contains("bearer")
            || lower.starts_with("saywhen-pro-");
        if !out.is_empty() {
            out.push(' ');
        }
        if secret {
            out.push_str("[redacted]");
        } else {
            out.push_str(word);
        }
    }
    out.chars().take(240).collect()
}

pub fn format_diagnostics(notes: &[DiagnosticNote]) -> String {
    if notes.is_empty() {
        return "No diagnostic notes saved on this computer.\n".to_string();
    }
    let mut out = String::from("saywhen diagnostic notes (this computer only)\n");
    for note in notes {
        out.push_str(&format!("{}\t{}\n", note.at_ms, note.message));
    }
    out
}

pub fn history_query_matches(query: &str, app: &str, text: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let mut app_filter: Option<String> = None;
    let mut words = Vec::new();
    for part in query.split_whitespace() {
        if let Some(rest) = part
            .strip_prefix("app:")
            .or_else(|| part.strip_prefix("App:"))
        {
            if !rest.is_empty() {
                app_filter = Some(rest.to_ascii_lowercase());
            }
        } else {
            words.push(part.to_ascii_lowercase());
        }
    }
    if let Some(expected) = app_filter {
        if !app.to_ascii_lowercase().contains(&expected) {
            return false;
        }
    }
    let hay = text.to_ascii_lowercase();
    words.iter().all(|word| hay.contains(word))
}

pub fn shortcut_catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Push to talk",
            "The shortcut in Settings. Default Alt+Space.",
        ),
        (
            "Cancel a preview",
            "Esc, a head shake, or start talking again.",
        ),
        ("Send now", "Enter, or a nod when nod mode is on."),
        ("Command palette", "Cmd+K on Mac, Ctrl+K on Windows."),
        (
            "Paste the last transcript",
            "Cmd+Ctrl+V on Mac, Ctrl+Alt+V on Windows.",
        ),
        ("Open Guide", "Cmd+4 or Ctrl+4."),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::CleanupResult;
    use crate::inject::{AppTargetPolicy, InjectionOutcome};
    use crate::transcribe::TranscriptionResult;

    #[test]
    fn quiet_hours_cross_midnight() {
        assert!(in_quiet_hours(22 * 60, 7 * 60, 23 * 60));
        assert!(in_quiet_hours(22 * 60, 7 * 60, 6 * 60));
        assert!(!in_quiet_hours(22 * 60, 7 * 60, 12 * 60));
        assert!(!in_quiet_hours(9 * 60, 9 * 60, 9 * 60));
        assert!(in_quiet_hours(9 * 60, 17 * 60, 9 * 60));
        assert!(!in_quiet_hours(9 * 60, 17 * 60, 17 * 60));
    }

    #[test]
    fn spoken_marks_and_capitals() {
        assert_eq!(apply_spoken_punctuation("git status period"), "git status.");
        assert_eq!(capitalize_sentences("hello. world"), "Hello. World");
    }

    #[test]
    fn settings_bundle_roundtrip_rejects_keys() {
        let raw = export_settings_bundle(&DictationConfig::default());
        assert!(raw.contains("\"version\": 1"));
        assert!(!raw.contains("llmApiKey"));
        let loaded = import_settings_bundle(&raw).unwrap();
        assert!(loaded.hands_free_enabled);
        assert!(
            import_settings_bundle(r#"{"version":1,"config":{},"llmApiKey":"sk-no"}"#).is_err()
        );
    }

    #[test]
    fn stats_model_advice_and_search() {
        let entry = sample("cargo test", "Terminal", true);
        let stats = summarize_history(&[entry]);
        assert_eq!(stats.words, 2);
        assert_eq!(stats.top_app, "Terminal");
        assert_eq!(stats.canceled, 0);
        let installed = vec!["tiny.en".into(), "large-v3-turbo".into()];
        assert_eq!(
            advise_model(&installed, ModelPreference::Accuracy).as_deref(),
            Some("large-v3-turbo")
        );
        assert_eq!(
            advise_model(&installed, ModelPreference::Speed).as_deref(),
            Some("tiny.en")
        );
        assert!(history_query_matches(
            "app:term cargo",
            "Terminal",
            "cargo test"
        ));
        assert!(!history_query_matches(
            "app:slack cargo",
            "Terminal",
            "cargo test"
        ));
    }

    #[test]
    fn diagnostics_redact_and_cap() {
        let mut notes = Vec::new();
        push_diagnostic(&mut notes, 1, "failed with sk-secret-value");
        assert!(notes[0].message.contains("[redacted]"));
        assert!(!notes[0].message.contains("sk-secret"));
        for index in 0..50 {
            push_diagnostic(&mut notes, index, "camera stalled");
        }
        assert_eq!(notes.len(), DIAGNOSTIC_CAP);
        assert!(format_diagnostics(&notes).contains("this computer only"));
        assert_eq!(UpdateChannel::parse("beta"), UpdateChannel::Beta);
        assert_eq!(HudAnchor::parse("top-center").as_str(), "top-center");
        assert!(!shortcut_catalog().is_empty());
    }

    fn sample(text: &str, app: &str, ok: bool) -> UtteranceCompleteEvent {
        UtteranceCompleteEvent {
            utterance_id: 1,
            transcription: TranscriptionResult {
                text: text.into(),
                audio_duration_ms: 1000,
                inference_ms: 100,
                rtf: 0.1,
                model_used: "tiny.en".into(),
                peak_vad_prob: 0.9,
                end_phrase_triggered: false,
                discarded_reason: None,
            },
            cleanup: CleanupResult {
                raw_text: text.into(),
                cleaned_text: text.into(),
                modified: false,
                mode_used: "local".into(),
                is_destructive_command: false,
                force_paste_only: false,
                canceled_by_voice: false,
            },
            target_policy: AppTargetPolicy {
                app_name: app.into(),
                bundle_id: None,
                exe: None,
                pid: None,
                policy: "paste_only".into(),
                will_press_enter: false,
                reason: "test".into(),
            },
            injection: Some(InjectionOutcome {
                success: ok,
                method: "clipboard".into(),
                pressed_enter: false,
                target_app: app.into(),
                policy_applied: "paste_only".into(),
                accessibility_trusted: true,
                message: "ok".into(),
            }),
            injection_error: None,
            trigger_source: "hotkey".into(),
            timestamp_ms: 1,
            flagged_false_trigger: false,
        }
    }
}
