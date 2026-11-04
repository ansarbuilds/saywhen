#[cfg(target_os = "macos")]
mod macos;
#[cfg(test)]
mod tests;
#[cfg(target_os = "windows")]
mod windows;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const SAYWHEN_BUNDLE_ID: &str = "dev.saywhen.desktop";
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
        names: &["terminal", "windows terminal", "powershell", "command prompt"],
        bundle_ids: &["com.apple.terminal"],
        exes: &["terminal", "windowsterminal.exe", "wt.exe", "powershell.exe", "pwsh.exe", "cmd.exe"],
    },
    CanonicalAppEntry {
        names: &["iterm2", "iterm"],
        bundle_ids: &["com.googlecode.iterm2"],
        exes: &["iterm2"],
    },
    CanonicalAppEntry {
        names: &["warp"],
        bundle_ids: &["dev.warp.warp-stable", "dev.warp.warp"],
        exes: &["warp", "warp.exe"],
    },
    CanonicalAppEntry {
        names: &["ghostty"],
        bundle_ids: &["com.mitchellh.ghostty"],
        exes: &["ghostty"],
    },
    CanonicalAppEntry {
        names: &["alacritty"],
        bundle_ids: &["org.alacritty", "io.alacritty"],
        exes: &["alacritty", "alacritty.exe"],
    },
    CanonicalAppEntry {
        names: &["kitty"],
        bundle_ids: &["net.kovidgoyal.kitty"],
        exes: &["kitty", "kitty.exe"],
    },
    CanonicalAppEntry {
        names: &["wezterm"],
        bundle_ids: &["com.github.wez.wezterm"],
        exes: &["wezterm-gui", "wezterm-gui.exe"],
    },
    CanonicalAppEntry {
        names: &["cursor"],
        bundle_ids: &["com.todesktop.230313mzl4w4u92"],
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
        exes: &["zed", "zed.exe"],
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

pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::check_accessibility_permission()
    }
    #[cfg(target_os = "windows")]
    {
        windows::check_accessibility_permission()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        true
    }
}

pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::open_accessibility_settings()
    }
    #[cfg(target_os = "windows")]
    {
        windows::open_accessibility_settings()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        Ok(())
    }
}

pub fn detect_frontmost_app() -> FrontmostApp {
    #[cfg(target_os = "macos")]
    {
        macos::detect_frontmost_app()
    }
    #[cfg(target_os = "windows")]
    {
        windows::detect_frontmost_app()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        FrontmostApp {
            name: "Focused Window".to_string(),
            bundle_id: None,
            exe: None,
            pid: None,
            detection_us: 0,
        }
    }
}

fn check_pre_injection_guard(target: &FrontmostApp) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::check_pre_injection_guard(target)
    }
    #[cfg(target_os = "windows")]
    {
        windows::check_pre_injection_guard(target)
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let _ = target;
        Ok(())
    }
}

fn clipboard_has_any_content() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::clipboard_has_any_content()
    }
    #[cfg(target_os = "windows")]
    {
        windows::clipboard_has_any_content()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        false
    }
}

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
    let app_bid = app.bundle_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let app_exe = app.exe.as_deref().map(str::trim).filter(|s| !s.is_empty());

    for raw in allowlist {
        let entry = raw.trim();
        if entry.is_empty() || entry.eq_ignore_ascii_case("saywhen") {
            continue;
        }
        if app_bid.map(|b| b.eq_ignore_ascii_case(entry)).unwrap_or(false) {
            return true;
        }
        if let Some(exe) = app_exe {
            if exe.eq_ignore_ascii_case(entry) {
                return true;
            }
            if let Some(stem) = entry.strip_suffix(".exe").or_else(|| entry.strip_suffix(".EXE")) {
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
    expected.name.trim().eq_ignore_ascii_case(actual.name.trim())
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
            reason: "Cannot inject into saywhen itself: focus a terminal or editor window".to_string(),
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
            policy: if wants_enter { "auto_send" } else { "paste_only" }.to_string(),
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
                if wants_enter { "Paste + Enter" } else { "Paste Only" }
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
            || target.bundle_id.as_deref().map(|b| b.trim().eq_ignore_ascii_case(key)).unwrap_or(false)
            || target.exe.as_deref().map(|x| x.trim().eq_ignore_ascii_case(key)).unwrap_or(false)
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

enum SavedClipboard {
    Text(String),
    Image { width: usize, height: usize, bytes: Vec<u8> },
    NonTextOrEmpty,
}

fn copy_transcript_fallback(text: &str) {
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

fn release_held_modifiers(enigo: &mut Enigo) {
    for key in [Key::Shift, Key::Control, Key::Alt, Key::Meta] {
        let _ = enigo.key(key, Direction::Release);
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    for lr in [Key::LShift, Key::RShift, Key::LControl, Key::RControl] {
        let _ = enigo.key(lr, Direction::Release);
    }
}

pub fn format_main_thread_timeout_error() -> String {
    "Timed out waiting 3s for main-thread keystroke dispatch (injection may have happened late). Transcript was left on your clipboard.".to_string()
}

pub fn inject_text_into_focused_app(
    app_handle: &AppHandle,
    text: &str,
    preview_policy: &AppTargetPolicy,
    timing: &InjectionTimingConfig,
) -> Result<InjectionOutcome, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Cannot inject empty transcript".to_string());
    }

    let current_app = detect_frontmost_app();
    let (target_app, press_enter, policy_applied, target_changed_notice) =
        match resolve_target_for_injection(preview_policy, &current_app, timing.on_target_changed) {
            PreInjectionDecision::Abort(reason) => {
                copy_transcript_fallback(trimmed);
                return Err(reason);
            }
            PreInjectionDecision::Proceed {
                target_app,
                press_enter,
                policy_applied,
                target_changed_notice,
            } => (target_app, press_enter, policy_applied, target_changed_notice),
        };

    if let Err(guard_err) = check_pre_injection_guard(&target_app) {
        copy_transcript_fallback(trimmed);
        return Err(guard_err);
    }

    let trusted = check_accessibility_permission();
    let enter_delay_ms = compute_enter_delay_ms(
        timing.enter_delay_ms,
        trimmed,
        &target_app,
        &timing.per_app_enter_delay_ms,
    );
    let use_clipboard_paste = timing.use_clipboard_paste;
    let text_owned = trimmed.to_string();
    let method_name = if use_clipboard_paste {
        "clipboard_paste".to_string()
    } else {
        "enigo_keystrokes".to_string()
    };

    let mut clipboard_note: Option<&'static str> = None;
    let saved_clipboard = if use_clipboard_paste {
        let mut cb = Clipboard::new().map_err(|e| format!("Clipboard open failed: {}", e))?;
        let snapshot = if let Ok(prev_text) = cb.get_text() {
            SavedClipboard::Text(prev_text)
        } else if let Ok(img) = cb.get_image() {
            clipboard_note = Some("previous clipboard image preserved");
            SavedClipboard::Image {
                width: img.width,
                height: img.height,
                bytes: img.bytes.into_owned(),
            }
        } else {
            if clipboard_has_any_content() {
                clipboard_note = Some("previous non-text clipboard content could not be preserved");
            }
            SavedClipboard::NonTextOrEmpty
        };
        cb.set_text(text_owned.clone())
            .map_err(|e| format!("Clipboard write failed: {}", e))?;
        Some(snapshot)
    } else {
        None
    };

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let text_for_main = text_owned.clone();
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_main = Arc::clone(&timed_out);
    let app_for_late_warning = app_handle.clone();

    app_handle
        .run_on_main_thread(move || {
            let res = (|| -> Result<(), String> {
                let settings = Settings {
                    open_prompt_to_get_permissions: false,
                    independent_of_keyboard_state: true,
                    ..Settings::default()
                };
                let mut enigo = Enigo::new(&settings)
                    .map_err(|e| format!("Failed to initialize Enigo input controller: {}", e))?;

                release_held_modifiers(&mut enigo);

                if use_clipboard_paste {
                    let paste_mod = if cfg!(target_os = "macos") { Key::Meta } else { Key::Control };
                    enigo.key(paste_mod, Direction::Press).map_err(|e| format!("Failed to press paste modifier: {}", e))?;
                    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| format!("Failed to press V key: {}", e))?;
                    enigo.key(paste_mod, Direction::Release).map_err(|e| format!("Failed to release paste modifier: {}", e))?;
                } else {
                    enigo.text(&text_for_main).map_err(|e| format!("Enigo text injection failed: {}", e))?;
                }

                if press_enter {
                    thread::sleep(Duration::from_millis(enter_delay_ms));
                    release_held_modifiers(&mut enigo);
                    enigo.key(Key::Return, Direction::Click).map_err(|e| format!("Failed to press Return key: {}", e))?;
                }
                Ok(())
            })();

            if timed_out_for_main.load(Ordering::SeqCst) {
                let _ = app_for_late_warning.emit(
                    "pipeline-error",
                    "Main-thread keystroke dispatch finished after 3s timeout: injection may have happened late",
                );
            }
            let _ = tx.send(res);
        })
        .map_err(|e| {
            copy_transcript_fallback(trimmed);
            format!("Failed to dispatch injection onto main thread: {}", e)
        })?;

    match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => {
            if let Some(snapshot) = saved_clipboard {
                let restore_wait_ms = timing
                    .clipboard_restore_delay_ms
                    .max(if press_enter { enter_delay_ms + 150 } else { 200 });
                let injected_copy = text_owned.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(restore_wait_ms));
                    if let Ok(mut cb) = Clipboard::new() {
                        let current = cb.get_text().ok();
                        if !should_restore_clipboard(&injected_copy, current.as_deref()) {
                            return;
                        }
                        match snapshot {
                            SavedClipboard::Text(prev_text) => {
                                let _ = cb.set_text(prev_text);
                            }
                            SavedClipboard::Image { width, height, bytes } => {
                                let _ = cb.set_image(ImageData {
                                    width,
                                    height,
                                    bytes: Cow::Borrowed(&bytes),
                                });
                            }
                            SavedClipboard::NonTextOrEmpty => {}
                        }
                    }
                });
            }

            if let Some(ref notice) = target_changed_notice {
                let _ = app_handle.emit("pipeline-error", notice.clone());
            }

            let mut msg = if let Some(notice) = target_changed_notice {
                notice
            } else if press_enter {
                format!(
                    "Injected into {} and pressed Enter ({} ms delay)",
                    target_app.name, enter_delay_ms
                )
            } else {
                format!("Pasted into {} (Paste-Only)", target_app.name)
            };

            if let Some(note) = clipboard_note {
                msg.push_str(&format!(" ({})", note));
            }

            Ok(InjectionOutcome {
                success: true,
                method: method_name,
                pressed_enter: press_enter,
                target_app: target_app.name,
                policy_applied,
                accessibility_trusted: trusted,
                message: msg,
            })
        }
        Ok(Err(err)) => {
            copy_transcript_fallback(trimmed);
            Err(err)
        }
        Err(_) => {
            timed_out.store(true, Ordering::SeqCst);
            copy_transcript_fallback(trimmed);
            Err(format_main_thread_timeout_error())
        }
    }
}
