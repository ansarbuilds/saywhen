//! Floating HUD pill and background-run plumbing.
//!
//! The HUD is a small transparent, always-on-top, non-focusable, click-through window
//! (label `hud`, declared in `tauri.conf.json`). It never takes keyboard focus, so the
//! frontmost app stays the injection target. Because it cannot be clicked, cancel during
//! the preview window is done with a global `Esc` shortcut that is registered only while
//! the pipeline is in `PREVIEW`, plus the existing head-shake and look-away paths.

use crate::audio::{AudioCommand, AudioPipelineController};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut};

pub const HUD_LABEL: &str = "hud";
pub const MAIN_LABEL: &str = "main";

/// Logical pixels between the bottom of the work area and the bottom edge of the HUD window.
const HUD_BOTTOM_MARGIN: f64 = 24.0;

/// Set once the user asked to quit, so the main window close handler stops hiding to background.
pub static QUITTING: AtomicBool = AtomicBool::new(false);
/// Set when the tray icon was created. Without a tray (and outside macOS, which has the
/// Dock reopen event) a hidden main window could never come back, so we quit instead.
pub static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn escape_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

/// Place the HUD bottom-center of the primary monitor work area, make it click-through
/// and show it. Errors are returned so the caller can surface them instead of failing silently.
pub fn setup_hud_window(app: &AppHandle) -> Result<(), String> {
    let hud = app
        .get_webview_window(HUD_LABEL)
        .ok_or_else(|| "HUD window `hud` is missing from tauri.conf.json".to_string())?;

    let monitor = hud
        .primary_monitor()
        .map_err(|e| format!("HUD: could not query primary monitor: {}", e))?
        .or(hud.current_monitor().ok().flatten())
        .ok_or_else(|| "HUD: no monitor found".to_string())?;

    let work = monitor.work_area();
    let scale = monitor.scale_factor();
    let size = hud
        .outer_size()
        .map_err(|e| format!("HUD: could not read window size: {}", e))?;

    let x = work.position.x + (work.size.width as i32 - size.width as i32) / 2;
    let y = work.position.y + work.size.height as i32
        - size.height as i32
        - (HUD_BOTTOM_MARGIN * scale).round() as i32;

    hud.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| format!("HUD: could not position window: {}", e))?;
    hud.set_ignore_cursor_events(true)
        .map_err(|e| format!("HUD: could not make window click-through: {}", e))?;
    hud.show()
        .map_err(|e| format!("HUD: could not show window: {}", e))?;

    // Showing the HUD at launch must not leave it as the key window.
    if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        let _ = main.set_focus();
    }
    Ok(())
}

/// Register the global `Esc` cancel shortcut only while a preview countdown is running,
/// so saywhen never swallows `Esc` in other apps outside that short window.
pub fn spawn_preview_escape_guard(app: AppHandle, audio: Arc<AudioPipelineController>) {
    thread::spawn(move || {
        let esc = escape_shortcut();
        let mut registered = false;
        let mut reported_failure = false;
        loop {
            let want = audio.is_in_preview();
            if want != registered {
                let shortcuts = app.global_shortcut();
                let result = if want {
                    shortcuts.register(esc)
                } else {
                    shortcuts.unregister(esc)
                };
                if let Err(err) = result {
                    eprintln!("saywhen: global Esc toggle failed: {}", err);
                    if want && !reported_failure {
                        reported_failure = true;
                        let _ = app.emit(
                            "pipeline-error",
                            format!(
                                "Global Esc cancel could not be registered ({}). Cancel the preview with a head shake or from the main window.",
                                err
                            ),
                        );
                    }
                }
                // Track the desired state either way so a failure does not retry every tick.
                registered = want;
            }
            thread::sleep(Duration::from_millis(30));
        }
    });
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

pub fn request_quit(app: &AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    app.exit(0);
}

/// Whether closing the main window should hide it (keep running with HUD + background
/// vision) rather than quit the app.
pub fn should_hide_on_close() -> bool {
    if QUITTING.load(Ordering::SeqCst) {
        return false;
    }
    cfg!(target_os = "macos") || TRAY_AVAILABLE.load(Ordering::SeqCst)
}

pub fn build_tray(app: &AppHandle, audio: Arc<AudioPipelineController>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open saywhen", true, None::<&str>)?;
    let camera = MenuItem::with_id(
        app,
        "camera",
        "Pause / Resume Camera Gate",
        true,
        None::<&str>,
    )?;
    let hud = MenuItem::with_id(app, "hud", "Show / Hide HUD Pill", true, None::<&str>)?;
    let cancel = MenuItem::with_id(app, "cancel", "Cancel Current Utterance", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit saywhen", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &camera, &hud, &cancel, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("saywhen-tray")
        .tooltip("saywhen")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "camera" => {
                let _ = app.emit_to(MAIN_LABEL, "tray-toggle-camera", ());
            }
            "hud" => {
                let cfg = audio.toggle_hud_enabled();
                let _ = app.emit("dictation-config-updated", cfg);
            }
            "cancel" => {
                let _ = audio.send_command(AudioCommand::CancelPreview(
                    "Canceled from tray menu".to_string(),
                ));
                let _ = audio.send_command(AudioCommand::CancelListening);
            }
            "quit" => request_quit(app),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    TRAY_AVAILABLE.store(true, Ordering::SeqCst);
    Ok(())
}
