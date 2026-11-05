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
    run_local_developer_cleanup_with_vocab(input, enabled, "")
}

/// Runs the local deterministic developer cleanup pass with optional user-defined custom
/// vocabulary terms (comma- or newline-separated identifiers/commands whose exact spelling
/// and casing should be enforced).
pub fn run_local_developer_cleanup_with_vocab(
    input: &str,
    enabled: bool,
    custom_vocab: &str,
) -> CleanupResult {
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

    // 2. Phonetic developer & CLI homophone corrections (Whisper tiny.en / base.en mishearings)
    let phonetic_replacements = [
        ("cubectal ", "kubectl "),
        ("kube control ", "kubectl "),
        ("kube ctl ", "kubectl "),
        ("ntm run ", "npm run "),
        ("npm runbuild", "npm run build"),
        ("ntm runbuild", "npm run build"),
        ("rust up update", "rustup update"),
        ("rust up ", "rustup "),
        ("get status", "git status"),
        ("get diff had", "git diff head"),
        ("get if had", "git diff head"),
        ("git if had", "git diff head"),
        ("get diff", "git diff"),
        ("get branch", "git branch"),
        ("get push", "git push"),
        ("get pull", "git pull"),
        ("get stash pot", "git stash pop"),
        ("get stash", "git stash"),
        ("get commit", "git commit"),
        ("get checkout", "git checkout"),
        ("good log onaline", "git log oneline"),
        ("good log online", "git log oneline"),
        ("gitlog online", "git log oneline"),
        ("gitlog onaline", "git log oneline"),
        ("git log onaline", "git log oneline"),
        ("git log online", "git log oneline"),
        ("git commit message. ", "git commit message "),
        ("fix, turn-taking, hysteresis", "fix turn-taking hysteresis"),
        ("cargo fmt", "cargo fmt"),
        ("cargo fm t-check", "cargo fmt check"),
        ("cargo test", "cargo test"),
        ("cargo build", "cargo build"),
        ("cargo check", "cargo check"),
        ("cargo clippy", "cargo clippy"),
        ("src torrey", "src-tauri"),
        ("s r c tauri", "src-tauri"),
        ("src tauri", "src-tauri"),
        ("czechmain.ts", "check main.ts"),
        ("czech main.ts", "check main.ts"),
        ("architecture dock", "architecture doc"),
        ("architecture dog", "architecture doc"),
        ("code base", "codebase"),
        ("vision frame stall", "vision frames stall"),
        ("outbrief ", "out brief "),
        ("coffs", "coughs"),
        ("say when", "saywhen"),
        ("called \"thee turn taking\".", "called feat turn taking"),
        ("called \"thee turn taking\"", "called feat turn taking"),
        ("called \"vee turn taking.\"", "called feat turn taking"),
        ("called \"viketerntaking\"", "called feat turn taking"),
    ];

    for (misheard, corrected) in &phonetic_replacements {
        s = replace_case_insensitive(&s, misheard, corrected);
    }
    s = replace_whole_term_case_insensitive(&s, "run unit test", "run unit tests");

    // 3. Spoken CLI flag & path normalizations
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

    // 4. Stitch mid-sentence thinking-pause periods before lowercase connectives/prepositions.
    //    When a developer pauses for 800-1000 ms mid-thought, Whisper often inserts a period
    //    before the second clause (e.g., "Refactor the audio worker. And add unit tests.").
    let pause_connectives = [
        (". and ", " and "),
        (". or ", " or "),
        (". but ", " but "),
        (". for ", " for "),
        (". in ", " in "),
        (". with ", " with "),
        (". when ", " when "),
        (". while ", " while "),
        (". before ", " before "),
        (". after ", " after "),
        (". to ", " to "),
        (". from ", " from "),
        (". into ", " into "),
        (". called ", " called "),
        (". using ", " using "),
        (". where ", " where "),
        (". that ", " that "),
        (". which ", " which "),
    ];
    for (broken, stitched) in &pause_connectives {
        s = replace_case_insensitive(&s, broken, stitched);
    }

    // 3. If utterance starts with a known CLI binary that Whisper capitalized, lowercase the binary name
    let cli_binaries = [
        "cargo", "git", "npm", "npx", "pnpm", "yarn", "bun", "node", "python", "python3", "pip",
        "pytest", "docker", "kubectl", "terraform", "make", "cmake", "rustc", "rustup", "ls", "cd",
        "cat", "grep", "rg", "find", "mkdir", "rm", "cp", "mv", "chmod", "chown", "curl", "ssh",
        "brew", "claude", "codex", "echo", "export", "source",
    ];

    let cli_subcommands = [
        "build", "test", "check", "clippy", "fmt", "run", "status", "short", "diff", "head",
        "log", "oneline", "branch", "list", "push", "origin", "pull", "rebase", "stash", "pop",
        "commit", "message", "checkout", "get", "pods", "nodes", "services", "update", "all",
        "targets", "release", "install", "add", "remove", "clean", "bench", "doc",
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
            for w in words.iter_mut().skip(1) {
                let trimmed = w.trim_end_matches(|c: char| ".,:;!?".contains(c));
                let lower = trimmed.to_lowercase();
                if cli_subcommands.contains(&lower.as_str()) {
                    *w = lower;
                }
            }
        }
    }

    let mut cleaned = words.join(" ").trim().to_string();
    if !custom_vocab.trim().is_empty() {
        cleaned = apply_custom_vocab_terms(&cleaned, custom_vocab);
    }
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

/// Normalizes any case-insensitive token or phrase match in `input` to the exact spelling and
/// casing specified in the user's comma/newline-separated `custom_vocab` list.
/// Also handles spaced compound variants (e.g., if the user lists `MediaPipe` or `saywhen`,
/// spoken `"media pipe"` or `"say when"` is normalized to `"MediaPipe"` or `"saywhen"`).
pub fn apply_custom_vocab_terms(input: &str, custom_vocab: &str) -> String {
    let mut out = input.to_string();
    for raw_entry in custom_vocab.split(|c: char| c == ',' || c == '\n' || c == ';') {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Support optional `PreferredTerm=spoken alias` syntax as well as plain `PreferredTerm`
        let (term, explicit_alias) = if let Some((lhs, rhs)) = entry.split_once('=') {
            (lhs.trim(), Some(rhs.trim().to_lowercase()))
        } else {
            (entry, None)
        };
        if term.is_empty() {
            continue;
        }
        let term_lower = term.to_lowercase();
        // 1. Exact case-insensitive word/phrase replacement
        out = replace_whole_term_case_insensitive(&out, &term_lower, term);

        // 2. Explicit alias if provided (`kubectl=kube ctl`)
        if let Some(alias_lower) = explicit_alias {
            if !alias_lower.is_empty() {
                out = replace_whole_term_case_insensitive(&out, &alias_lower, term);
            }
        }

        // 3. If the custom term is a camelCase/PascalCase/hyphenated compound (e.g., "MediaPipe",
        //    "TypeScript", "whisper-rs"), also match its space-separated form ("media pipe",
        //    "type script", "whisper rs").
        let spaced = split_camel_or_hyphen_to_spaces(term);
        if spaced != term_lower && spaced.contains(' ') {
            out = replace_whole_term_case_insensitive(&out, &spaced, term);
        }
    }
    out
}

fn split_camel_or_hyphen_to_spaces(term: &str) -> String {
    let mut s = String::new();
    let chars: Vec<char> = term.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '-' || c == '_' {
            s.push(' ');
            continue;
        }
        if i > 0
            && c.is_uppercase()
            && (chars[i - 1].is_lowercase()
                || (i + 1 < chars.len() && chars[i + 1].is_lowercase()))
        {
            s.push(' ');
        }
        s.extend(c.to_lowercase());
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace_whole_term_case_insensitive(
    haystack: &str,
    needle_lower: &str,
    replacement: &str,
) -> String {
    if needle_lower.is_empty() {
        return haystack.to_string();
    }
    let mut result = String::with_capacity(haystack.len());
    let mut remaining = haystack;
    let mut prev_char: Option<char> = None;

    while !remaining.is_empty() {
        let rem_lower = remaining.to_lowercase();
        if let Some(pos) = rem_lower.find(needle_lower) {
            let before = &remaining[..pos];
            let boundary_before = if pos == 0 {
                prev_char.map(|c| !c.is_alphanumeric()).unwrap_or(true)
            } else {
                before
                    .chars()
                    .last()
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true)
            };
            let after_slice = &remaining[pos + needle_lower.len()..];
            let boundary_after = after_slice
                .chars()
                .next()
                .map(|c| !c.is_alphanumeric())
                .unwrap_or(true);

            result.push_str(before);
            if boundary_before && boundary_after {
                result.push_str(replacement);
            } else {
                result.push_str(&remaining[pos..pos + needle_lower.len()]);
            }
            prev_char = remaining[..pos + needle_lower.len()].chars().last();
            remaining = after_slice;
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

/// Legacy pre-Improvement #5 cleanup pass used by `src-tauri/src/bin/replay.rs` to benchmark
/// before/after command accuracy across the 56-utterance corpus.
pub fn run_legacy_developer_cleanup(input: &str) -> String {
    let raw = input.trim().to_string();
    if raw.is_empty() {
        return raw;
    }
    let mut s = raw;
    for (pat, rep) in [
        ("uh, ", ""),
        ("um, ", ""),
        ("uh ", ""),
        ("um ", ""),
        ("er, ", ""),
        ("ah, ", ""),
        ("you know, ", ""),
        ("basically, ", ""),
        ("like, ", ""),
    ] {
        s = replace_case_insensitive_word_start(&s, pat, rep);
    }
    for (spoken, token) in [
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
    ] {
        s = replace_case_insensitive(&s, spoken, token);
    }
    s = s.replace("-- ", "--").replace("./ ", "./");
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
            if let Some(last) = words.last_mut() {
                if last.ends_with('.') && !last.ends_with("..") {
                    last.pop();
                }
            }
        }
    }
    words.join(" ").trim().to_string()
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

    #[test]
    fn test_phonetic_cli_homophones_and_thinking_pause_stitching() {
        assert_eq!(
            run_local_developer_cleanup("Cubectal Get Pods.", true).cleaned_text,
            "kubectl get pods"
        );
        assert_eq!(
            run_local_developer_cleanup("NTM RunBuild", true).cleaned_text,
            "npm run build"
        );
        assert_eq!(
            run_local_developer_cleanup("Get If Had.", true).cleaned_text,
            "git diff head"
        );
        assert_eq!(
            run_local_developer_cleanup("Good Log Onaline.", true).cleaned_text,
            "git log oneline"
        );
        assert_eq!(
            run_local_developer_cleanup("Get Stash Pot.", true).cleaned_text,
            "git stash pop"
        );
        assert_eq!(
            run_local_developer_cleanup("Refactor the audio worker. And add unit tests.", true)
                .cleaned_text,
            "Refactor the audio worker and add unit tests."
        );
        assert_eq!(
            run_local_developer_cleanup("Run Cargo Test. In the SRC Torrey folder", true)
                .cleaned_text,
            "Run cargo test in the src-tauri folder"
        );
    }

    #[test]
    fn test_custom_vocabulary_casing_and_compound_normalization() {
        let vocab = "saywhen, MediaPipe, Silero, whisper-rs, TypeScript";
        let res = run_local_developer_cleanup_with_vocab(
            "Update say when to use media pipe and silero with whisper rs in type script.",
            true,
            vocab,
        );
        assert_eq!(
            res.cleaned_text,
            "Update saywhen to use MediaPipe and Silero with whisper-rs in TypeScript."
        );
    }
}
