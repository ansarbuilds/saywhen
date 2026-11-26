//! Per-app overrides. A matching profile can change the send style, add words,
//! or pick a transcription language for the app in front. Unset fields inherit
//! the global settings.

use crate::turn_taking::SendMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppProfile {
    pub app: String,
    #[serde(default)]
    pub send_mode: Option<SendMode>,
    #[serde(default)]
    pub extra_vocab: String,
    #[serde(default)]
    pub language: Option<String>,
}

pub fn sanitize_profiles(profiles: &mut Vec<AppProfile>) {
    let mut cleaned = Vec::new();
    for mut profile in profiles.drain(..) {
        profile.app = profile.app.trim().to_string();
        profile.extra_vocab = profile.extra_vocab.trim().to_string();
        if let Some(language) = profile.language.as_mut() {
            let trimmed = language.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                profile.language = None;
            } else {
                *language = trimmed;
            }
        }
        if profile.app.is_empty() || profile.app.chars().count() > 200 {
            continue;
        }
        if cleaned
            .iter()
            .any(|existing: &AppProfile| existing.app.eq_ignore_ascii_case(&profile.app))
        {
            continue;
        }
        cleaned.push(profile);
        if cleaned.len() == 40 {
            break;
        }
    }
    *profiles = cleaned;
}

pub fn match_app_profile<'a>(
    profiles: &'a [AppProfile],
    name: &str,
    bundle_id: Option<&str>,
    exe: Option<&str>,
) -> Option<&'a AppProfile> {
    profiles.iter().find(|profile| {
        same(&profile.app, name)
            || bundle_id.is_some_and(|value| same(&profile.app, value))
            || exe.is_some_and(|value| {
                same(&profile.app, value)
                    || value
                        .rsplit(['/', '\\'])
                        .next()
                        .is_some_and(|file| same(&profile.app, file))
            })
    })
}

pub fn merge_vocab(base: &str, extra: &str) -> String {
    let extra = extra.trim();
    if extra.is_empty() {
        return base.trim().to_string();
    }
    let base = base.trim();
    if base.is_empty() {
        extra.to_string()
    } else {
        format!("{base}, {extra}")
    }
}

fn same(left: &str, right: &str) -> bool {
    !left.trim().is_empty() && left.trim().eq_ignore_ascii_case(right.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_name_bundle_or_exe_and_merges_words() {
        let profiles = vec![
            AppProfile {
                app: "Cursor".into(),
                send_mode: Some(SendMode::Phrase),
                extra_vocab: "MediaPipe".into(),
                language: None,
            },
            AppProfile {
                app: "Slack.exe".into(),
                send_mode: Some(SendMode::PasteOnly),
                extra_vocab: String::new(),
                language: Some("en".into()),
            },
        ];
        let cursor = match_app_profile(
            &profiles,
            "Cursor",
            Some("com.todesktop.230313mzl4w4u92"),
            None,
        )
        .unwrap();
        assert_eq!(cursor.send_mode, Some(SendMode::Phrase));
        assert_eq!(
            merge_vocab("saywhen", &cursor.extra_vocab),
            "saywhen, MediaPipe"
        );
        let slack = match_app_profile(
            &profiles,
            "Slack",
            None,
            Some(r"C:\Program Files\Slack\Slack.exe"),
        );
        assert_eq!(slack.unwrap().send_mode, Some(SendMode::PasteOnly));
        assert!(match_app_profile(&profiles, "Notes", None, None).is_none());
    }
}
