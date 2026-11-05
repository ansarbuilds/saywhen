use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Must match `identifier` in `tauri.conf.json` (checked by a unit test).
pub const SAYWHEN_BUNDLE_ID: &str = "dev.saywhen.app";
pub const DEFAULT_ENTER_DELAY_MS: u64 = 120;
pub const MAX_ENTER_DELAY_MS: u64 = 400;
pub const CHARS_PER_EXTRA_MS: usize = 20;
pub const DEFAULT_CLIPBOARD_RESTORE_DELAY_MS: u64 = 600;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrontmostApp {
    pub name: String,
    #[serde(default)]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub exe: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub detection_us: u64,
    #[serde(default)]
    pub window_center_x: Option<i32>,
    #[serde(default)]
    pub primary_screen_width: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetChangePolicy {
    Abort,
    PasteOnly,
}

impl Default for TargetChangePolicy {
    fn default() -> Self {
        Self::Abort
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTargetPolicy {
    pub app_name: String,
    #[serde(default)]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub exe: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    pub policy: String, // "auto_send" | "paste_only" | "destructive_guard" | "self_guard" | "target_changed_paste_only"
    pub will_press_enter: bool,
    pub reason: String,
}

impl AppTargetPolicy {
    pub fn to_frontmost_app(&self) -> FrontmostApp {
        FrontmostApp {
            name: self.app_name.clone(),
            bundle_id: self.bundle_id.clone(),
            exe: self.exe.clone(),
            pid: self.pid,
            detection_us: 0,
            window_center_x: None,
            primary_screen_width: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectionOutcome {
    pub success: bool,
    pub method: String,
    pub pressed_enter: bool,
    pub target_app: String,
    pub policy_applied: String,
    pub accessibility_trusted: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct InjectionTimingConfig {
    pub use_clipboard_paste: bool,
    pub enter_delay_ms: u64,
    pub per_app_enter_delay_ms: HashMap<String, u64>,
    pub clipboard_restore_delay_ms: u64,
    pub on_target_changed: TargetChangePolicy,
}

impl Default for InjectionTimingConfig {
    fn default() -> Self {
        Self {
            use_clipboard_paste: true,
            enter_delay_ms: DEFAULT_ENTER_DELAY_MS,
            per_app_enter_delay_ms: HashMap::new(),
            clipboard_restore_delay_ms: DEFAULT_CLIPBOARD_RESTORE_DELAY_MS,
            on_target_changed: TargetChangePolicy::Abort,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreInjectionDecision {
    Proceed {
        target_app: FrontmostApp,
        press_enter: bool,
        policy_applied: String,
        target_changed_notice: Option<String>,
    },
    Abort(String),
}

struct CanonicalAppEntry {
    names: &'static [&'static str],
    bundle_ids: &'static [&'static str],
    exes: &'static [&'static str],
}

const KNOWN_DEVELOPER_APPS: &[CanonicalAppEntry] = &[
    CanonicalAppEntry {
        names: &[
            "terminal",
            "windows terminal",
            "powershell",
            "command prompt",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
        ],
        bundle_ids: &["com.apple.terminal", "org.gnome.terminal", "org.kde.konsole"],
        exes: &[
            "terminal",
            "windowsterminal.exe",
            "wt.exe",
            "powershell.exe",
            "pwsh.exe",
            "cmd.exe",
            "gnome-terminal-server",
            "konsole",
            "xfce4-terminal",
        ],
    },
    CanonicalAppEntry {
        names: &["iterm2", "iterm"],
        bundle_ids: &["com.googlecode.iterm2"],
        exes: &["iterm2"],
    },
    CanonicalAppEntry {
        names: &["warp"],
        bundle_ids: &["dev.warp.warp-stable", "dev.warp.warp"],
        exes: &["warp", "warp.exe", "warp-terminal"],
    },
    CanonicalAppEntry {
        names: &["ghostty"],
        bundle_ids: &["com.mitchellh.ghostty"],
        exes: &["ghostty"],
    },
    CanonicalAppEntry {
        names: &["alacritty"],
        bundle_ids: &["org.alacritty", "io.alacritty", "alacritty"],
        exes: &["alacritty", "alacritty.exe"],
    },
    CanonicalAppEntry {
        names: &["kitty"],
        bundle_ids: &["net.kovidgoyal.kitty", "kitty"],
        exes: &["kitty", "kitty.exe"],
    },
    CanonicalAppEntry {
        names: &["wezterm"],
        bundle_ids: &["com.github.wez.wezterm", "org.wezfurlong.wezterm"],
        exes: &["wezterm-gui", "wezterm-gui.exe"],
    },
    CanonicalAppEntry {
        names: &["cursor"],
        bundle_ids: &["com.todesktop.230313mzl4w4u92", "cursor"],
        exes: &["cursor", "cursor.exe"],
    },
    CanonicalAppEntry {
        names: &["code", "visual studio code", "code - insiders"],
        bundle_ids: &["com.microsoft.vscode", "com.microsoft.vscodeinsiders"],
        exes: &["code", "code.exe", "code - insiders.exe"],
    },
    CanonicalAppEntry {
        names: &["zed"],
        bundle_ids: &["dev.zed.zed", "dev.zed.zed-preview"],
        exes: &["zed", "zed.exe", "zed-editor"],
    },
    CanonicalAppEntry {
        names: &["antigravity"],
        bundle_ids: &["com.google.antigravity", "dev.antigravity"],
        exes: &["antigravity", "antigravity.exe"],
    },
    CanonicalAppEntry {
        names: &["claude"],
        bundle_ids: &["com.anthropic.claudefordesktop"],
        exes: &["claude", "claude.exe"],
    },
];

pub fn default_auto_send_allowlist() -> Vec<String> {
    vec![
        "Terminal".to_string(),
        "iTerm2".to_string(),
        "Warp".to_string(),
        "Ghostty".to_string(),
        "Alacritty".to_string(),
        "kitty".to_string(),
        "WezTerm".to_string(),
        "Cursor".to_string(),
        "Code".to_string(),
        "Visual Studio Code".to_string(),
        "Zed".to_string(),
        "Antigravity".to_string(),
        "Claude".to_string(),
        "Windows Terminal".to_string(),
        "PowerShell".to_string(),
    ]
}

pub fn is_saywhen_app(app: &FrontmostApp) -> bool {
    if let Some(pid) = app.pid {
        if pid == std::process::id() {
            return true;
        }
    }
    if let Some(ref bid) = app.bundle_id {
        if bid.trim().eq_ignore_ascii_case(SAYWHEN_BUNDLE_ID) {
            return true;
        }
    }
    if let Some(ref exe) = app.exe {
        let trimmed = exe.trim();
        if trimmed.eq_ignore_ascii_case("saywhen") || trimmed.eq_ignore_ascii_case("saywhen.exe") {
            return true;
        }
    }
    let name = app.name.trim();
    name.eq_ignore_ascii_case("saywhen") || name.eq_ignore_ascii_case("saywhen.exe")
}

pub fn matches_allowlist(app: &FrontmostApp, allowlist: &[String]) -> bool {
    if is_saywhen_app(app) {
        return false;
    }

    let app_name = app.name.trim();
    let app_bid = app
        .bundle_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let app_exe = app.exe.as_deref().map(str::trim).filter(|s| !s.is_empty());

    for raw in allowlist {
        let entry = raw.trim();
        if entry.is_empty() || entry.eq_ignore_ascii_case("saywhen") {
            continue;
        }
        if app_bid
            .map(|b| b.eq_ignore_ascii_case(entry))
            .unwrap_or(false)
        {
            return true;
        }
        if let Some(exe) = app_exe {
            if exe.eq_ignore_ascii_case(entry) {
                return true;
            }
            if let Some(stem) = entry
                .strip_suffix(".exe")
                .or_else(|| entry.strip_suffix(".EXE"))
            {
                if !app_name.is_empty() && app_name.eq_ignore_ascii_case(stem) {
                    return true;
                }
            }
        }

        let entry_lower = entry.to_ascii_lowercase();
        if let Some(canon) = KNOWN_DEVELOPER_APPS
            .iter()
            .find(|c| c.names.contains(&entry_lower.as_str()))
        {
            if app_bid
                .map(|b| canon.bundle_ids.iter().any(|cb| b.eq_ignore_ascii_case(cb)))
                .unwrap_or(false)
            {
                return true;
            }
            if app_exe
                .map(|x| canon.exes.iter().any(|cx| x.eq_ignore_ascii_case(cx)))
                .unwrap_or(false)
            {
                return true;
            }
            if app_bid.is_none() && !app_name.is_empty() && app_name.eq_ignore_ascii_case(entry) {
                return true;
            }
            continue;
        }

        if !app_name.is_empty() && app_name.eq_ignore_ascii_case(entry) {
            return true;
        }
    }
    false
}

pub fn same_target_app(expected: &FrontmostApp, actual: &FrontmostApp) -> bool {
    if let (Some(p1), Some(p2)) = (expected.pid, actual.pid) {
        if p1 != p2 {
            return false;
        }
    }
    if let (Some(b1), Some(b2)) = (&expected.bundle_id, &actual.bundle_id) {
        if !b1.trim().eq_ignore_ascii_case(b2.trim()) {
            return false;
        }
    }
    if let (Some(e1), Some(e2)) = (&expected.exe, &actual.exe) {
        if !e1.trim().eq_ignore_ascii_case(e2.trim()) {
            return false;
        }
    }
    expected
        .name
        .trim()
        .eq_ignore_ascii_case(actual.name.trim())
}

pub fn evaluate_app_injection_policy(
    app: &FrontmostApp,
    wants_enter: bool,
    per_app_rules_enabled: bool,
    is_destructive: bool,
    allowlist: &[String],
) -> AppTargetPolicy {
    if is_saywhen_app(app) {
        return AppTargetPolicy {
            app_name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            exe: app.exe.clone(),
            pid: app.pid,
            policy: "self_guard".to_string(),
            will_press_enter: false,
            reason: "Cannot inject into saywhen itself: focus a terminal or editor window"
                .to_string(),
        };
    }

    if is_destructive {
        return AppTargetPolicy {
            app_name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            exe: app.exe.clone(),
            pid: app.pid,
            policy: "destructive_guard".to_string(),
            will_press_enter: false,
            reason: "Destructive command detected: auto-Enter blocked (paste-only)".to_string(),
        };
    }

    if !per_app_rules_enabled {
        return AppTargetPolicy {
            app_name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            exe: app.exe.clone(),
            pid: app.pid,
            policy: if wants_enter {
                "auto_send"
            } else {
                "paste_only"
            }
            .to_string(),
            will_press_enter: wants_enter,
            reason: format!("Target: {}", app.name),
        };
    }

    if matches_allowlist(app, allowlist) {
        AppTargetPolicy {
            app_name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            exe: app.exe.clone(),
            pid: app.pid,
            policy: "auto_send".to_string(),
            will_press_enter: wants_enter,
            reason: format!(
                "{} is on the Terminal/Agent allowlist ({})",
                app.name,
                if wants_enter {
                    "Paste + Enter"
                } else {
                    "Paste Only"
                }
            ),
        }
    } else {
        AppTargetPolicy {
            app_name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            exe: app.exe.clone(),
            pid: app.pid,
            policy: "paste_only".to_string(),
            will_press_enter: false,
            reason: format!(
                "{} is not on the auto-Enter allowlist: enforcing safe Paste-Only",
                app.name
            ),
        }
    }
}

pub fn resolve_target_for_injection(
    preview_policy: &AppTargetPolicy,
    current_app: &FrontmostApp,
    on_target_changed: TargetChangePolicy,
) -> PreInjectionDecision {
    if is_saywhen_app(current_app) || preview_policy.policy == "self_guard" {
        return PreInjectionDecision::Abort(
            "Refused to inject into saywhen's own window. Focus your terminal or coding agent first; transcript was copied to your clipboard.".to_string(),
        );
    }

    let expected = preview_policy.to_frontmost_app();
    if same_target_app(&expected, current_app) {
        return PreInjectionDecision::Proceed {
            target_app: current_app.clone(),
            press_enter: preview_policy.will_press_enter,
            policy_applied: preview_policy.policy.clone(),
            target_changed_notice: None,
        };
    }

    match on_target_changed {
        TargetChangePolicy::Abort => PreInjectionDecision::Abort(format!(
            "Target app changed during preview ({} -> {}). Injection aborted; transcript was copied to your clipboard.",
            preview_policy.app_name, current_app.name
        )),
        TargetChangePolicy::PasteOnly => PreInjectionDecision::Proceed {
            target_app: current_app.clone(),
            press_enter: false,
            policy_applied: "target_changed_paste_only".to_string(),
            target_changed_notice: Some(format!(
                "Target app changed during preview ({} -> {}): pasted without pressing Enter",
                preview_policy.app_name, current_app.name
            )),
        },
    }
}

pub fn compute_enter_delay_ms(
    base_delay_ms: u64,
    text: &str,
    target: &FrontmostApp,
    per_app_overrides: &HashMap<String, u64>,
) -> u64 {
    let effective_base = lookup_per_app_delay(target, per_app_overrides).unwrap_or(base_delay_ms);
    let extra_ms = (text.chars().count() / CHARS_PER_EXTRA_MS) as u64;
    let cap = MAX_ENTER_DELAY_MS.max(effective_base);
    effective_base.saturating_add(extra_ms).min(cap)
}

fn lookup_per_app_delay(target: &FrontmostApp, overrides: &HashMap<String, u64>) -> Option<u64> {
    for (k, &val) in overrides {
        let key = k.trim();
        if key.is_empty() {
            continue;
        }
        if target.name.trim().eq_ignore_ascii_case(key)
            || target
                .bundle_id
                .as_deref()
                .map(|b| b.trim().eq_ignore_ascii_case(key))
                .unwrap_or(false)
            || target
                .exe
                .as_deref()
                .map(|x| x.trim().eq_ignore_ascii_case(key))
                .unwrap_or(false)
        {
            return Some(val);
        }
    }
    None
}

pub fn should_restore_clipboard(
    injected_transcript: &str,
    current_clipboard_text: Option<&str>,
) -> bool {
    matches!(current_clipboard_text, Some(current) if current == injected_transcript)
}

/// Enter is only pressed if the app we pasted into is still frontmost after the delay.
pub fn should_press_enter_after_paste(
    pasted_into: &FrontmostApp,
    now_frontmost: &FrontmostApp,
) -> bool {
    !is_saywhen_app(now_frontmost) && same_target_app(pasted_into, now_frontmost)
}

/// Checks whether the focused target window's horizontal display position (`window_center_x`)
/// conflicts with the speaker's head yaw (`head_yaw_deg`).
///
/// If the focused target window is on an external left display (`window_center_x < -200`) while
/// the speaker is looking to the right (`head_yaw_deg > 14.0°`), or on an external right display
/// (`window_center_x > primary_screen_width + 200`) while the speaker is looking to the left
/// (`head_yaw_deg < -14.0°`), returns a descriptive Paste-Only downgrade reason.
pub fn evaluate_gaze_window_alignment(
    app: &FrontmostApp,
    head_yaw_deg: f32,
) -> Option<String> {
    let (Some(win_x), Some(screen_w)) = (app.window_center_x, app.primary_screen_width) else {
        return None;
    };
    if screen_w == 0 {
        return None;
    }
    let right_edge = (screen_w as i32) + 200;
    if win_x < -200 && head_yaw_deg > 14.0 {
        return Some(format!(
            "Focused window '{}' is on the left display (x={}) while gaze is directed right (yaw={:+.1}°): enforcing Paste-Only",
            app.name, win_x, head_yaw_deg
        ));
    }
    if win_x > right_edge && head_yaw_deg < -14.0 {
        return Some(format!(
            "Focused window '{}' is on the right display (x={}) while gaze is directed left (yaw={:+.1}°): enforcing Paste-Only",
            app.name, win_x, head_yaw_deg
        ));
    }
    None
}
