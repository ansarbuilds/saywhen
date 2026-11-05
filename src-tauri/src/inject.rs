pub mod linux;
#[cfg(target_os = "macos")]
mod macos;
pub mod policy;
#[cfg(test)]
mod tests;
#[cfg(target_os = "windows")]
mod windows;

pub use policy::*;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// How long to wait for each main-thread keystroke dispatch before giving up.
const MAIN_THREAD_DISPATCH_TIMEOUT: Duration = Duration::from_secs(3);

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
        linux::detect_frontmost_app()
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
        linux::check_pre_injection_guard(target)
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

enum SavedClipboard {
    Text(String),
    Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
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
    "Timed out waiting 3s for main-thread keystroke dispatch; the queued keystrokes were cancelled so nothing is typed late. Transcript was left on your clipboard.".to_string()
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

    // Step 1: paste (or type) on the main thread. The Enter delay is slept on this worker
    // thread, not the main thread, so the app event loop never blocks for up to 400 ms.
    let text_for_main = text_owned.clone();
    let step_one = dispatch_keystrokes_on_main_thread(app_handle, move |enigo| {
        release_held_modifiers(enigo);
        if use_clipboard_paste {
            let paste_mod = if cfg!(target_os = "macos") {
                Key::Meta
            } else {
                Key::Control
            };
            enigo
                .key(paste_mod, Direction::Press)
                .map_err(|e| format!("Failed to press paste modifier: {}", e))?;
            enigo
                .key(Key::Unicode('v'), Direction::Click)
                .map_err(|e| format!("Failed to press V key: {}", e))?;
            enigo
                .key(paste_mod, Direction::Release)
                .map_err(|e| format!("Failed to release paste modifier: {}", e))?;
        } else {
            enigo
                .text(&text_for_main)
                .map_err(|e| format!("Enigo text injection failed: {}", e))?;
        }
        Ok(())
    });

    // Step 2: wait, re-check focus, then press Enter only if the same app is still frontmost.
    let mut enter_skipped_notice: Option<String> = None;
    let mut pressed_enter = false;
    let result = step_one.and_then(|()| {
        if !press_enter {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(enter_delay_ms));
        let now_front = detect_frontmost_app();
        if !should_press_enter_after_paste(&target_app, &now_front) {
            enter_skipped_notice = Some(format!(
                "Focus moved from {} to {} before Enter: pasted but Enter was not pressed",
                target_app.name, now_front.name
            ));
            return Ok(());
        }
        dispatch_keystrokes_on_main_thread(app_handle, |enigo| {
            release_held_modifiers(enigo);
            enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| format!("Failed to press Return key: {}", e))
        })?;
        pressed_enter = true;
        Ok(())
    });

    match result {
        Ok(()) => {
            if let Some(snapshot) = saved_clipboard {
                let restore_wait_ms = timing
                    .clipboard_restore_delay_ms
                    .max(if press_enter { 150 } else { 200 });
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
                            SavedClipboard::Image {
                                width,
                                height,
                                bytes,
                            } => {
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

            let notice = target_changed_notice.or(enter_skipped_notice);
            if let Some(ref notice) = notice {
                let _ = app_handle.emit("pipeline-error", notice.clone());
            }

            let mut msg = if let Some(notice) = notice {
                notice
            } else if pressed_enter {
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
                pressed_enter,
                target_app: target_app.name,
                policy_applied,
                accessibility_trusted: trusted,
                message: msg,
            })
        }
        Err(err) => {
            copy_transcript_fallback(trimmed);
            Err(err)
        }
    }
}

/// Run keystrokes on the main thread and wait up to `MAIN_THREAD_DISPATCH_TIMEOUT`.
/// If the wait times out, the queued closure is cancelled before it sends any key, so a
/// stalled main thread can never paste or press Enter seconds later into another app.
fn dispatch_keystrokes_on_main_thread<F>(
    app_handle: &AppHandle,
    keystrokes: F,
) -> Result<(), String>
where
    F: FnOnce(&mut Enigo) -> Result<(), String> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_main = Arc::clone(&timed_out);
    let started = Arc::new(AtomicBool::new(false));
    let started_for_main = Arc::clone(&started);

    app_handle
        .run_on_main_thread(move || {
            if timed_out_for_main.load(Ordering::SeqCst) {
                return;
            }
            started_for_main.store(true, Ordering::SeqCst);
            let res = (|| -> Result<(), String> {
                let settings = Settings {
                    open_prompt_to_get_permissions: false,
                    independent_of_keyboard_state: true,
                    ..Settings::default()
                };
                let mut enigo = Enigo::new(&settings)
                    .map_err(|e| format!("Failed to initialize Enigo input controller: {}", e))?;
                keystrokes(&mut enigo)
            })();
            let _ = tx.send(res);
        })
        .map_err(|e| format!("Failed to dispatch injection onto main thread: {}", e))?;

    match rx.recv_timeout(MAIN_THREAD_DISPATCH_TIMEOUT) {
        Ok(res) => res,
        Err(_) => {
            timed_out.store(true, Ordering::SeqCst);
            // The flag is checked before any key is sent; a result that raced in is still honored.
            match rx.try_recv() {
                Ok(res) => res,
                Err(_) if started.load(Ordering::SeqCst) => Err(
                    "Main-thread keystroke dispatch was still running after 3s: injection may have happened late. Transcript was left on your clipboard.".to_string(),
                ),
                Err(_) => Err(format_main_thread_timeout_error()),
            }
        }
    }
}
