//! Spoken shortcuts. A trigger phrase is replaced with the exact expansion
//! the user saved, after the local clean-up pass and before the check step.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub trigger: String,
    pub expansion: String,
}

/// Drops empty or unsafe entries, keeps the first of each trigger, and sorts
/// longer triggers first so "my work email" wins over "my work".
pub fn sanitize_snippets(snippets: &mut Vec<Snippet>) {
    let mut cleaned = Vec::new();
    for snippet in snippets.drain(..) {
        let trigger = normalize_phrase(&snippet.trigger);
        let expansion = snippet.expansion.trim().to_string();
        if trigger.chars().count() < 2 || expansion.is_empty() || expansion.chars().count() > 2_000
        {
            continue;
        }
        if cleaned
            .iter()
            .any(|existing: &Snippet| existing.trigger.eq_ignore_ascii_case(&trigger))
        {
            continue;
        }
        cleaned.push(Snippet { trigger, expansion });
        if cleaned.len() == 100 {
            break;
        }
    }
    cleaned.sort_by(|a, b| b.trigger.len().cmp(&a.trigger.len()));
    *snippets = cleaned;
}

pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    let mut output = text.to_string();
    for snippet in snippets {
        output = replace_phrase(&output, &snippet.trigger, &snippet.expansion);
    }
    output
}

fn normalize_phrase(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace_phrase(haystack: &str, trigger: &str, expansion: &str) -> String {
    if trigger.is_empty() {
        return haystack.to_string();
    }
    let lower_hay = haystack.to_lowercase();
    let lower_trigger = trigger.to_lowercase();
    let mut result = String::with_capacity(haystack.len());
    let mut rest = haystack;
    let mut rest_lower = lower_hay.as_str();
    while let Some(index) = rest_lower.find(&lower_trigger) {
        let end = index + lower_trigger.len();
        let before = rest[..index].chars().next_back();
        let after = rest[end..].chars().next();
        if is_boundary(before) && is_boundary(after) {
            result.push_str(&rest[..index]);
            result.push_str(expansion);
        } else {
            result.push_str(&rest[..end]);
        }
        rest = &rest[end..];
        rest_lower = &rest_lower[end..];
    }
    result.push_str(rest);
    result
}

fn is_boundary(ch: Option<char>) -> bool {
    match ch {
        None => true,
        Some(ch) => !ch.is_alphanumeric() && ch != '_' && ch != '-',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_whole_phrases_and_keeps_longer_triggers() {
        let mut snippets = vec![
            Snippet {
                trigger: "my email".into(),
                expansion: "ada@example.com".into(),
            },
            Snippet {
                trigger: "  my work email ".into(),
                expansion: "ada@work.example".into(),
            },
            Snippet {
                trigger: "pr".into(),
                expansion: "pull request".into(),
            },
        ];
        sanitize_snippets(&mut snippets);
        assert_eq!(snippets[0].trigger, "my work email");
        let text = apply_snippets("Email my work email and print the pr status", &snippets);
        assert_eq!(
            text,
            "Email ada@work.example and print the pull request status"
        );
        assert!(!text.contains("pull requester") || text.contains("print the pull request"));
        let untouched = apply_snippets("print preview", &snippets);
        assert_eq!(untouched, "print preview");
    }

    #[test]
    fn drops_blank_and_duplicate_triggers() {
        let mut snippets = vec![
            Snippet {
                trigger: "ok".into(),
                expansion: "   ".into(),
            },
            Snippet {
                trigger: "Ship it".into(),
                expansion: "please ship".into(),
            },
            Snippet {
                trigger: "ship it".into(),
                expansion: "second".into(),
            },
        ];
        sanitize_snippets(&mut snippets);
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].expansion, "please ship");
    }
}
