use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCleanupConfig {
    pub enabled: bool,
    pub endpoint_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_llm_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_llm_timeout_ms() -> u64 {
    1200
}

impl Default for LlmCleanupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint_url: "http://127.0.0.1:11434/v1/chat/completions".to_string(),
            model: "qwen2.5-coder:1.5b".to_string(),
            api_key: String::new(),
            timeout_ms: default_llm_timeout_ms(),
        }
    }
}

/// Classifies an LLM endpoint URL as `"local_loopback"` (`127.0.0.1`, `localhost`, `[::1]`),
/// `"remote_byok"` (any external host), or `"invalid_url"`.
pub fn classify_llm_endpoint_privacy(endpoint_url: &str) -> &'static str {
    let trimmed = endpoint_url.trim().to_ascii_lowercase();
    let after_scheme = if let Some(rest) = trimmed.strip_prefix("http://") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("https://") {
        rest
    } else {
        return "invalid_url";
    };
    let host_port = after_scheme.split('/').next().unwrap_or("");
    if host_port.is_empty() {
        return "invalid_url";
    }
    let host = if host_port.starts_with('[') {
        host_port.split(']').next().unwrap_or("").trim_start_matches('[')
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    if host == "127.0.0.1" || host == "localhost" || host == "::1" {
        "local_loopback"
    } else {
        "remote_byok"
    }
}

/// Strips markdown code fences, backticks, or conversational preambles that small LLMs
/// sometimes emit around a cleaned developer command. Falls back to `fallback` if the
/// model output is empty or looks like a refusal/conversational explanation.
pub fn sanitize_llm_cleanup_output(raw_llm_output: &str, fallback: &str) -> String {
    let mut s = raw_llm_output.trim();
    if s.is_empty() {
        return fallback.to_string();
    }
    if s.starts_with("```") && s.ends_with("```") {
        let inner = s.trim_start_matches("```");
        let after_lang = if let Some(nl) = inner.find('\n') {
            &inner[nl + 1..]
        } else {
            inner
        };
        s = after_lang.trim_end_matches("```").trim();
    } else if s.starts_with('`') && s.ends_with('`') && s.len() > 2 && !s[1..s.len() - 1].contains('`') {
        s = s[1..s.len() - 1].trim();
    }
    let lower = s.to_ascii_lowercase();
    for bad_prefix in [
        "sure,",
        "sure!",
        "here is the",
        "here's the",
        "cleaned command:",
        "i cannot",
        "i'm sorry",
    ] {
        if lower.starts_with(bad_prefix) {
            return fallback.to_string();
        }
    }
    if s.is_empty() {
        fallback.to_string()
    } else {
        s.to_string()
    }
}

/// Calls an opt-in local Ollama (`http://127.0.0.1:11434/v1/chat/completions`) or OpenAI-compatible
/// BYOK endpoint with a strict timeout. Never called unless `llm_cfg.enabled == true`.
pub fn run_opt_in_llm_cleanup(
    deterministic_text: &str,
    custom_vocab: &str,
    llm_cfg: &LlmCleanupConfig,
) -> Result<String, String> {
    if !llm_cfg.enabled || deterministic_text.trim().is_empty() {
        return Ok(deterministic_text.to_string());
    }
    let privacy = classify_llm_endpoint_privacy(&llm_cfg.endpoint_url);
    if privacy == "invalid_url" {
        return Err(format!(
            "Invalid LLM cleanup endpoint URL '{}': kept deterministic transcript",
            llm_cfg.endpoint_url
        ));
    }

    let timeout_secs = format!("{:.2}", (llm_cfg.timeout_ms.clamp(250, 5000) as f64) / 1000.0);

    let vocab_hint = if custom_vocab.trim().is_empty() {
        String::new()
    } else {
        format!(" Preserve these exact custom terms: {}.", custom_vocab.trim())
    };
    let sys_prompt = format!(
        "You are a deterministic developer dictation formatter. Output ONLY the cleaned CLI command or coding agent prompt with no markdown fences, no quotes, and no explanation.{}",
        vocab_hint
    );

    let payload = serde_json::json!({
        "model": llm_cfg.model.trim(),
        "temperature": 0.0,
        "max_tokens": 256,
        "messages": [
            { "role": "system", "content": sys_prompt },
            { "role": "user", "content": deterministic_text }
        ]
    });
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Failed to serialize LLM request payload: {}", e))?;

    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-f",
        "--max-time",
        &timeout_secs,
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        "@-",
    ]);
    if !llm_cfg.api_key.trim().is_empty() {
        cmd.arg("-H")
            .arg(format!("Authorization: Bearer {}", llm_cfg.api_key.trim()));
    }
    cmd.arg(llm_cfg.endpoint_url.trim())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "Opt-in LLM cleanup failed to launch curl ({}): kept deterministic transcript",
            e
        )
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&payload_bytes);
    }

    let output = child.wait_with_output().map_err(|e| {
        format!(
            "Opt-in LLM cleanup failed waiting for response ({}): kept deterministic transcript",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Opt-in LLM cleanup request to '{}' failed ({}): kept deterministic transcript",
            llm_cfg.endpoint_url,
            stderr.trim()
        ));
    }

    let body: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "Opt-in LLM cleanup response parse failed ({}): kept deterministic transcript",
            e
        )
    })?;

    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");
    Ok(sanitize_llm_cleanup_output(content, deterministic_text))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub raw_text: String,
    pub cleaned_text: String,
    pub modified: bool,
    pub mode_used: String,
    pub is_destructive_command: bool,
    #[serde(default)]
    pub force_paste_only: bool,
    #[serde(default)]
    pub canceled_by_voice: bool,
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
    if is_spoken_cancel_command(&raw) {
        return CleanupResult {
            raw_text: raw,
            cleaned_text: String::new(),
            modified: true,
            mode_used: "voice_cancel".to_string(),
            is_destructive_command: false,
            force_paste_only: false,
            canceled_by_voice: true,
        };
    }

    let (after_scratch, had_scratch_that) = apply_scratch_that_correction(&raw);
    if had_scratch_that && after_scratch.is_empty() {
        return CleanupResult {
            raw_text: raw,
            cleaned_text: String::new(),
            modified: true,
            mode_used: "voice_cancel".to_string(),
            is_destructive_command: false,
            force_paste_only: false,
            canceled_by_voice: true,
        };
    }

    let (after_paste_override, force_paste_only) = strip_paste_only_phrase(&after_scratch);
    if force_paste_only && after_paste_override.is_empty() {
        return CleanupResult {
            raw_text: raw,
            cleaned_text: String::new(),
            modified: true,
            mode_used: "none".to_string(),
            is_destructive_command: false,
            force_paste_only: true,
            canceled_by_voice: false,
        };
    }

    if !enabled || after_paste_override.is_empty() {
        let destructive = check_destructive_command(&after_paste_override);
        let modified = after_paste_override != raw;
        return CleanupResult {
            raw_text: raw,
            cleaned_text: after_paste_override,
            modified,
            mode_used: "none".to_string(),
            is_destructive_command: destructive,
            force_paste_only,
            canceled_by_voice: false,
        };
    }

    let mut s = after_paste_override;

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

    // Collapse accidental space after -- or ./ and hyphenate common multi-word flags
    s = s
        .replace("-- ", "--")
        .replace("./ ", "./")
        .replace("--all targets", "--all-targets")
        .replace("--no default features", "--no-default-features")
        .replace("--manifest path", "--manifest-path");

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
        force_paste_only,
        canceled_by_voice: false,
    }
}

/// Checks if the entire utterance is a spoken cancel phrase (`"cancel"`, `"cancel that"`,
/// `"never mind"`).
pub fn is_spoken_cancel_command(input: &str) -> bool {
    let normalized = input
        .trim()
        .trim_matches(|c: char| ".!,?;:\"'".contains(c))
        .trim()
        .to_lowercase();
    matches!(
        normalized.as_str(),
        "cancel" | "cancel that" | "never mind" | "nevermind"
    )
}

/// Applies spoken `"scratch that"` self-correction:
/// - If `"scratch that"` appears mid-utterance (e.g., `"git checkout main, scratch that, git checkout feat"`),
///   discards everything before the last `"scratch that"` and returns `("git checkout feat", true)`.
/// - If `"scratch that"` is at the end of the utterance with nothing after it, returns `("", true)`,
///   signaling that the utterance was canceled by voice.
pub fn apply_scratch_that_correction(input: &str) -> (String, bool) {
    let lower = input.to_lowercase();
    let needle = "scratch that";
    let mut search_end = lower.len();
    while let Some(pos) = lower[..search_end].rfind(needle) {
        let before = &input[..pos];
        let boundary_before = before
            .chars()
            .last()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        let after_slice = &input[pos + needle.len()..];
        let boundary_after = after_slice
            .chars()
            .next()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
        if boundary_before && boundary_after {
            let trimmed_after = after_slice
                .trim_start_matches(|c: char| c.is_whitespace() || ".,:;!?-".contains(c))
                .trim();
            return (trimmed_after.to_string(), true);
        }
        if pos == 0 {
            break;
        }
        search_end = pos;
    }
    (input.to_string(), false)
}

/// Strips trailing `"paste only"` / `"just paste it"` / `"just paste"` / `"no enter"` voice
/// delivery overrides and returns `(remaining_text, true)` when matched.
pub fn strip_paste_only_phrase(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    let stripped_punct = trimmed
        .trim_end_matches(|c: char| ".!,?;:".contains(c))
        .trim_end();
    let lower = stripped_punct.to_lowercase();

    for phrase in ["paste only", "just paste it", "just paste", "no enter"] {
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

    #[test]
    fn test_scratch_that_correction_and_paste_only_voice_override() {
        // 1. Mid-utterance "scratch that" keeps only the corrected command after "scratch that"
        let res = run_local_developer_cleanup(
            "Cargo build, scratch that, Cargo test dash dash all targets.",
            true,
        );
        assert_eq!(res.cleaned_text, "cargo test --all-targets");
        assert!(!res.canceled_by_voice);
        assert!(!res.force_paste_only);

        // 2. Trailing "scratch that" cancels the entire utterance
        let canceled = run_local_developer_cleanup("rm dash rf temp scratch that.", true);
        assert!(canceled.canceled_by_voice);
        assert!(canceled.cleaned_text.is_empty());

        // 3. Spoken cancel variants ("cancel that", "never mind") cancel the utterance
        let cancel_that = run_local_developer_cleanup("Cancel that!", true);
        assert!(cancel_that.canceled_by_voice);
        let never_mind = run_local_developer_cleanup("Never mind.", true);
        assert!(never_mind.canceled_by_voice);

        // 4. Trailing "paste only" / "just paste it" strips the suffix and sets force_paste_only
        let paste_only = run_local_developer_cleanup(
            "Git commit dash m wip, paste only.",
            true,
        );
        assert_eq!(paste_only.cleaned_text, "git commit -m wip");
        assert!(paste_only.force_paste_only);
        assert!(!paste_only.canceled_by_voice);
    }

    #[test]
    fn test_llm_endpoint_privacy_classification_and_output_sanitization() {
        assert_eq!(
            classify_llm_endpoint_privacy("http://127.0.0.1:11434/v1/chat/completions"),
            "local_loopback"
        );
        assert_eq!(
            classify_llm_endpoint_privacy("http://localhost:8080/v1/chat/completions"),
            "local_loopback"
        );
        assert_eq!(
            classify_llm_endpoint_privacy("https://api.openai.com/v1/chat/completions"),
            "remote_byok"
        );
        assert_eq!(classify_llm_endpoint_privacy("not-a-url"), "invalid_url");

        // Strips markdown code fences and backticks
        assert_eq!(
            sanitize_llm_cleanup_output("```bash\ncargo test --release\n```", "cargo test"),
            "cargo test --release"
        );
        assert_eq!(
            sanitize_llm_cleanup_output("`git status --short`", "git status"),
            "git status --short"
        );
        // Rejects conversational preambles and keeps deterministic fallback
        assert_eq!(
            sanitize_llm_cleanup_output("Sure, here is the command: cargo test", "cargo test"),
            "cargo test"
        );
    }
}
