use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub raw_text: String,
    pub cleaned_text: String,
    pub modified: bool,
    pub mode_used: String,
    pub is_destructive_command: bool,
}

/// Runs the local deterministic developer cleanup pass.
/// Removes spoken filler words, formats spoken CLI flags and file paths,
/// and normalizes capitalized shell command verbs while keeping identifiers intact.
pub fn run_local_developer_cleanup(input: &str, enabled: bool) -> CleanupResult {
    let raw = input.trim().to_string();
    if !enabled || raw.is_empty() {
        let destructive = check_destructive_command(&raw);
        return CleanupResult {
            raw_text: raw.clone(),
            cleaned_text: raw,
            modified: false,
            mode_used: "none".to_string(),
            is_destructive_command: destructive,
        };
    }

    let mut s = raw.clone();

    // 1. Strip leading/inline spoken fillers (case-insensitive word boundaries)
    let filler_patterns = [
        ("uh, ", ""),
        ("um, ", ""),
        ("uh ", ""),
        ("um ", ""),
        ("er, ", ""),
        ("ah, ", ""),
        ("you know, ", ""),
        ("basically, ", ""),
        ("like, ", ""),
    ];

    for (pat, rep) in &filler_patterns {
        s = replace_case_insensitive_word_start(&s, pat, rep);
    }

    // 2. Spoken CLI flag & path normalizations
    let code_replacements = [
        ("dash dash ", "--"),
        ("double dash ", "--"),
        ("dash ", "-"),
        ("dot slash ", "./"),
        (" slash ", "/"),
        (" dot rs", ".rs"),
        (" dot ts", ".ts"),
        (" dot js", ".js"),
        (" dot py", ".py"),
        (" dot md", ".md"),
        (" dot json", ".json"),
        (" dot toml", ".toml"),
        (" dot yaml", ".yaml"),
        (" dot yml", ".yml"),
        (" dot html", ".html"),
        (" dot css", ".css"),
    ];

    for (spoken, token) in &code_replacements {
        s = replace_case_insensitive(&s, spoken, token);
    }

    // Collapse accidental space after -- or ./
    s = s.replace("-- ", "--").replace("./ ", "./");

    // 3. If utterance starts with a known CLI binary that Whisper capitalized, lowercase the binary name
    let cli_binaries = [
        "cargo", "git", "npm", "npx", "pnpm", "yarn", "bun", "node", "python", "python3", "pip",
        "pytest", "docker", "kubectl", "terraform", "make", "cmake", "rustc", "rustup", "ls", "cd",
        "cat", "grep", "rg", "find", "mkdir", "rm", "cp", "mv", "chmod", "chown", "curl", "ssh",
        "brew", "claude", "codex", "echo", "export", "source",
    ];

    let mut words: Vec<String> = s.split_whitespace().map(|w| w.to_string()).collect();
    if let Some(first) = words.first_mut() {
        let trimmed_first = first.trim_end_matches(|c: char| ".,:;!?".contains(c));
        let lower_first = trimmed_first.to_lowercase();
        if cli_binaries.contains(&lower_first.as_str()) {
            *first = lower_first;
            // For pure CLI commands, strip trailing period that Whisper often adds
            if let Some(last) = words.last_mut() {
                if last.ends_with('.') && !last.ends_with("..") {
                    last.pop();
                }
            }
        }
    }

    let cleaned = words.join(" ").trim().to_string();
    let modified = cleaned != raw;
    let destructive = check_destructive_command(&cleaned);

    CleanupResult {
        raw_text: raw,
        cleaned_text: cleaned,
        modified,
        mode_used: "local_dev_rules".to_string(),
        is_destructive_command: destructive,
    }
}

pub fn check_destructive_command(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let dangerous_patterns = [
        "rm -rf",
        "rm -fr",
        "sudo rm",
        "git reset --hard",
        "git push --force",
        "git push -f",
        "git clean -fd",
        "drop table",
        "drop database",
        "truncate table",
        "mkfs",
        "dd if=",
    ];
    dangerous_patterns.iter().any(|p| lower.contains(p))
}

fn replace_case_insensitive_word_start(haystack: &str, needle_lower: &str, replacement: &str) -> String {
    let mut out = haystack.to_string();
    let lower = out.to_lowercase();
    if lower.starts_with(needle_lower) {
        out = format!("{}{}", replacement, &out[needle_lower.len()..]);
    }
    let spaced_needle = format!(" {}", needle_lower);
    let spaced_rep = if replacement.is_empty() {
        " ".to_string()
    } else {
        format!(" {}", replacement)
    };
    replace_case_insensitive(&out, &spaced_needle, &spaced_rep)
}

fn replace_case_insensitive(haystack: &str, needle_lower: &str, replacement: &str) -> String {
    if needle_lower.is_empty() {
        return haystack.to_string();
    }
    let mut result = String::with_capacity(haystack.len());
    let mut remaining = haystack;
    while !remaining.is_empty() {
        let rem_lower = remaining.to_lowercase();
        if let Some(pos) = rem_lower.find(needle_lower) {
            result.push_str(&remaining[..pos]);
            result.push_str(replacement);
            remaining = &remaining[pos + needle_lower.len()..];
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_developer_cleanup_preserves_flags_and_commands() {
        let res = run_local_developer_cleanup(
            "Um, Cargo build dash dash release.",
            true,
        );
        assert_eq!(res.cleaned_text, "cargo build --release");
        assert!(res.modified);
        assert!(!res.is_destructive_command);
    }

    #[test]
    fn test_destructive_guard() {
        let res = run_local_developer_cleanup("rm dash rf slash tmp", true);
        assert!(res.is_destructive_command);
    }
}
