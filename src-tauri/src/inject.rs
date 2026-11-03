use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTargetPolicy {
    pub app_name: String,
    pub policy: String, // "auto_send" | "paste_only" | "destructive_guard"
    pub will_press_enter: bool,
    pub reason: String,
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

pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        AXIsProcessTrusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| format!("Failed to open macOS Accessibility settings: {}", e))?;
    }
    Ok(())
}

pub fn detect_frontmost_app() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(front_out) = std::process::Command::new("/usr/bin/lsappinfo")
            .arg("front")
            .output()
        {
            let asn = String::from_utf8_lossy(&front_out.stdout).trim().to_string();
            if !asn.is_empty() {
                if let Ok(info_out) = std::process::Command::new("/usr/bin/lsappinfo")
                    .args(["info", "-only", "name", &asn])
                    .output()
                {
                    let raw = String::from_utf8_lossy(&info_out.stdout);
                    if let Some(first_quote) = raw.find('"') {
                        let rest = &raw[first_quote + 1..];
                        if let Some(second_quote) = rest.find('"') {
                            let candidate = &rest[..second_quote];
                            if candidate != "LSDisplayName" && !candidate.is_empty() {
                                return candidate.to_string();
                            } else {
                                let after_eq = &rest[second_quote + 1..];
                                if let Some(q1) = after_eq.find('"') {
                                    let r2 = &after_eq[q1 + 1..];
                                    if let Some(q2) = r2.find('"') {
                                        return r2[..q2].to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        "macOS App".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Process | Where-Object {$_.MainWindowHandle -eq (Add-Type -MemberDefinition '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();' -Name Win32 -Namespace Native -PassThru)::GetForegroundWindow()}).ProcessName",
            ])
            .output()
        {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
        "Windows App".to_string()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        "Focused Window".to_string()
    }
}

pub fn default_auto_send_allowlist() -> Vec<String> {
    vec![
        "saywhen".to_string(),
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
    ]
}

pub fn evaluate_app_injection_policy(
    app_name: &str,
    wants_enter: bool,
    per_app_rules_enabled: bool,
    is_destructive: bool,
    allowlist: &[String],
) -> AppTargetPolicy {
    if is_destructive {
        return AppTargetPolicy {
            app_name: app_name.to_string(),
            policy: "destructive_guard".to_string(),
            will_press_enter: false,
            reason: "Destructive command detected: auto-Enter blocked (paste-only)".to_string(),
        };
    }

    let app_lower = app_name.to_lowercase();
    let is_allowed_terminal_or_agent = allowlist
        .iter()
        .any(|allowed| app_lower.contains(&allowed.to_lowercase()));

    if !per_app_rules_enabled {
        return AppTargetPolicy {
            app_name: app_name.to_string(),
            policy: if wants_enter { "auto_send" } else { "paste_only" }.to_string(),
            will_press_enter: wants_enter,
            reason: format!("Target: {}", app_name),
        };
    }

    if is_allowed_terminal_or_agent {
        AppTargetPolicy {
            app_name: app_name.to_string(),
            policy: "auto_send".to_string(),
            will_press_enter: wants_enter,
            reason: format!(
                "{} is on the Terminal/Agent allowlist ({})",
                app_name,
                if wants_enter { "Paste + Enter" } else { "Paste Only" }
            ),
        }
    } else {
        AppTargetPolicy {
            app_name: app_name.to_string(),
            policy: "paste_only".to_string(),
            will_press_enter: false,
            reason: format!(
                "{} is not on the auto-Enter allowlist: enforcing safe Paste-Only",
                app_name
            ),
        }
    }
}

pub fn inject_text_into_focused_app(
    app_handle: &AppHandle,
    text: &str,
    press_enter: bool,
    use_clipboard_paste: bool,
    target_app: &str,
    policy_applied: &str,
) -> Result<InjectionOutcome, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Cannot inject empty transcript".to_string());
    }

    let trusted = check_accessibility_permission();
    if !trusted {
        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.set_text(trimmed.to_string());
        }
        return Err(
            "macOS Accessibility permission is required to type into terminals. Transcript was copied to your clipboard; grant Accessibility permission and try again.".to_string(),
        );
    }

    let text_owned = trimmed.to_string();
    let method_name = if use_clipboard_paste {
        "clipboard_paste".to_string()
    } else {
        "enigo_keystrokes".to_string()
    };

    let previous_clipboard = if use_clipboard_paste {
        if let Ok(mut cb) = Clipboard::new() {
            let prev = cb.get_text().ok();
            cb.set_text(text_owned.clone())
                .map_err(|e| format!("Clipboard write failed: {}", e))?;
            prev
        } else {
            None
        }
    } else {
        None
    };

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let text_for_main = text_owned.clone();

    app_handle
        .run_on_main_thread(move || {
            let res = (|| -> Result<(), String> {
                let mut enigo = Enigo::new(&Settings::default())
                    .map_err(|e| format!("Failed to initialize Enigo input controller: {}", e))?;

                if use_clipboard_paste {
                    #[cfg(target_os = "macos")]
                    {
                        enigo
                            .key(Key::Meta, Direction::Press)
                            .map_err(|e| format!("Failed to press Command key: {}", e))?;
                        enigo
                            .key(Key::Unicode('v'), Direction::Click)
                            .map_err(|e| format!("Failed to press V key: {}", e))?;
                        enigo
                            .key(Key::Meta, Direction::Release)
                            .map_err(|e| format!("Failed to release Command key: {}", e))?;
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        enigo
                            .key(Key::Control, Direction::Press)
                            .map_err(|e| format!("Failed to press Control key: {}", e))?;
                        enigo
                            .key(Key::Unicode('v'), Direction::Click)
                            .map_err(|e| format!("Failed to press V key: {}", e))?;
                        enigo
                            .key(Key::Control, Direction::Release)
                            .map_err(|e| format!("Failed to release Control key: {}", e))?;
                    }
                } else {
                    enigo
                        .text(&text_for_main)
                        .map_err(|e| format!("Enigo text injection failed: {}", e))?;
                }

                if press_enter {
                    thread::sleep(Duration::from_millis(45));
                    enigo
                        .key(Key::Return, Direction::Click)
                        .map_err(|e| format!("Failed to press Return key: {}", e))?;
                }

                Ok(())
            })();
            let _ = tx.send(res);
        })
        .map_err(|e| format!("Failed to dispatch injection onto main thread: {}", e))?;

    match rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => {
            if use_clipboard_paste {
                if let Some(prev) = previous_clipboard {
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(160));
                        if let Ok(mut cb) = Clipboard::new() {
                            let _ = cb.set_text(prev);
                        }
                    });
                }
            }
            Ok(InjectionOutcome {
                success: true,
                method: method_name,
                pressed_enter: press_enter,
                target_app: target_app.to_string(),
                policy_applied: policy_applied.to_string(),
                accessibility_trusted: trusted,
                message: if press_enter {
                    format!("Injected into {} and pressed Enter", target_app)
                } else {
                    format!("Pasted into {} (Paste-Only)", target_app)
                },
            })
        }
        Ok(Err(err)) => Err(err),
        Err(_) => Err("Timed out waiting for main-thread keystroke injection".to_string()),
    }
}
