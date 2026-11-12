pub mod audio;
pub mod cleanup;
pub mod history;
pub mod hud;
pub mod inject;
pub mod transcribe;
pub mod turn_taking;
pub mod wake_word;

use audio::{
    list_input_devices, unix_timestamp_ms, AudioCommand, AudioPipelineController, DictationConfig,
    UtteranceCompleteEvent, VisionTelemetryPayload,
};
use cleanup::{run_local_developer_cleanup, run_local_developer_cleanup_with_vocab, CleanupResult};
use history::{
    clear_utterance_history_file, load_and_prune_history, update_history_false_trigger_flag,
};
use inject::{
    check_accessibility_permission, detect_frontmost_app, evaluate_app_injection_policy,
    inject_text_into_focused_app, open_accessibility_settings, FrontmostApp, InjectionOutcome,
};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use transcribe::{ModelInfo, TranscriberEngine};
use turn_taking::{update_false_trigger_flag_in_metrics, EndReason};

pub struct AppState {
    pub audio: Arc<AudioPipelineController>,
    pub transcriber: Arc<TranscriberEngine>,
    pub hotkey: Arc<Mutex<Shortcut>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatusSnapshot {
    pub state: String,
    pub devices: Vec<String>,
    pub config: DictationConfig,
    pub accessibility_trusted: bool,
    pub models: Vec<ModelInfo>,
    pub selected_model_id: String,
    pub hotkey_label: String,
    pub frontmost_app: String,
    pub recent_history: Vec<UtteranceCompleteEvent>,
}

#[tauri::command]
fn get_system_status(app: AppHandle, state: State<'_, AppState>) -> SystemStatusSnapshot {
    let config = state.audio.get_config();
    let hotkey_label = config.hotkey_accelerator.clone();
    let recent_history = if config.history_persistence_enabled {
        app.path()
            .app_data_dir()
            .ok()
            .map(|dir| load_and_prune_history(&dir, unix_timestamp_ms()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    SystemStatusSnapshot {
        state: state.audio.current_state(),
        devices: list_input_devices(),
        config,
        accessibility_trusted: check_accessibility_permission(),
        models: state.transcriber.list_models(),
        selected_model_id: state.transcriber.get_selected_model_id(),
        hotkey_label,
        frontmost_app: detect_frontmost_app().name,
        recent_history,
    }
}

#[tauri::command]
fn start_listening(state: State<'_, AppState>, source: Option<String>) -> Result<(), String> {
    state
        .audio
        .send_command(AudioCommand::StartListening(
            source.unwrap_or_else(|| "ui_button".to_string()),
        ))
}

#[tauri::command]
fn stop_and_transcribe(state: State<'_, AppState>) -> Result<(), String> {
    state
        .audio
        .send_command(AudioCommand::StopAndTranscribe(Some(EndReason::ManualStop)))
}

#[tauri::command]
fn flag_false_trigger(app: AppHandle, utterance_id: u64, flagged: bool) -> Result<(), String> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        update_false_trigger_flag_in_metrics(&app_data_dir, utterance_id, flagged)?;
        let _ = update_history_false_trigger_flag(&app_data_dir, utterance_id, flagged);
    }
    Ok(())
}

#[tauri::command]
fn clear_utterance_history(app: AppHandle) -> Result<(), String> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        clear_utterance_history_file(&app_data_dir)?;
    }
    Ok(())
}

#[tauri::command]
fn cancel_listening(state: State<'_, AppState>) -> Result<(), String> {
    state.audio.send_command(AudioCommand::CancelListening)
}

#[tauri::command]
fn confirm_preview_now(
    state: State<'_, AppState>,
    override_text: Option<String>,
) -> Result<(), String> {
    state
        .audio
        .send_command(AudioCommand::ConfirmPreviewNow(override_text))
}

#[tauri::command]
fn cancel_preview(state: State<'_, AppState>, reason: Option<String>) -> Result<(), String> {
    state.audio.send_command(AudioCommand::CancelPreview(
        reason.unwrap_or_else(|| "Canceled by user".to_string()),
    ))
}

#[tauri::command]
fn test_cleanup_pass(state: State<'_, AppState>, sample_text: String) -> CleanupResult {
    let cfg = state.audio.get_config();
    run_local_developer_cleanup_with_vocab(&sample_text, true, &cfg.custom_vocab)
}

#[tauri::command]
fn update_dictation_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: DictationConfig,
) -> Result<DictationConfig, String> {
    let previous = state.audio.get_config();
    let trimmed_hotkey = config.hotkey_accelerator.trim().to_string();
    if trimmed_hotkey.eq_ignore_ascii_case("escape") || trimmed_hotkey.eq_ignore_ascii_case("esc") {
        return Err("Escape is reserved for canceling previews. Pick a shortcut with a modifier like CommandOrControl+Shift+Space.".to_string());
    }
    let requested_hotkey = trimmed_hotkey
        .parse::<Shortcut>()
        .map_err(|err| format!("That shortcut is not valid: {}", err))?;
    let updated = state.audio.update_config(config)?;
    if previous.hotkey_accelerator != updated.hotkey_accelerator {
        let old_hotkey = state
            .hotkey
            .lock()
            .map_err(|_| "Shortcut lock failed".to_string())?
            .clone();
        if requested_hotkey != old_hotkey {
            let _ = app.global_shortcut().unregister(old_hotkey);
            if let Err(err) = app.global_shortcut().register(requested_hotkey) {
                let _ = app.global_shortcut().register(old_hotkey);
                let _ = state.audio.update_config(previous);
                return Err(format!("That shortcut is already taken: {}", err));
            }
            *state.hotkey.lock().map_err(|_| "Shortcut lock failed".to_string())? =
                requested_hotkey;
        }
    }
    if !updated.history_persistence_enabled {
        if let Ok(app_data_dir) = app.path().app_data_dir() {
            let _ = clear_utterance_history_file(&app_data_dir);
        }
    }
    let _ = app.emit("dictation-config-updated", &updated);
    Ok(updated)
}

#[tauri::command]
fn update_vision_telemetry(
    state: State<'_, AppState>,
    telemetry: VisionTelemetryPayload,
) -> Result<(), String> {
    state
        .audio
        .send_command(AudioCommand::UpdateVision(telemetry))
}

#[tauri::command]
fn set_camera_gate_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .audio
        .send_command(AudioCommand::SetCameraGateEnabled(enabled))
}

#[tauri::command]
fn select_whisper_model(state: State<'_, AppState>, model_id: String) -> Result<String, String> {
    state.transcriber.set_selected_model(&model_id)
}

#[tauri::command]
fn get_frontmost_app() -> FrontmostApp {
    detect_frontmost_app()
}

#[tauri::command]
fn set_ui_zoom(app: AppHandle, scale: f64) -> Result<(), String> {
    let scale = scale.clamp(0.8, 1.5);
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is not available".to_string())?;
    window
        .set_zoom(scale)
        .map_err(|err| format!("Could not set the window size: {}", err))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelDownloadProgress {
    model_id: String,
    downloaded_bytes: u64,
    total_bytes: u64,
}

#[tauri::command]
async fn download_whisper_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelInfo, String> {
    let transcriber = Arc::clone(&state.transcriber);
    let total_bytes = transcriber
        .list_models()
        .iter()
        .find(|model| model.id == model_id)
        .map(|model| model.size_mb.saturating_mul(1024 * 1024))
        .unwrap_or(0);
    let part_path = transcriber.model_download_path(&model_id);
    let stop = Arc::new(AtomicBool::new(false));
    let progress_app = app.clone();
    let progress_model = model_id.clone();
    let progress_stop = Arc::clone(&stop);
    let poller = std::thread::spawn(move || {
        let mut reported = u64::MAX;
        while !progress_stop.load(Ordering::Relaxed) {
            let downloaded = part_path
                .as_ref()
                .and_then(|path| std::fs::metadata(path).ok())
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if downloaded != reported {
                reported = downloaded;
                let _ = progress_app.emit(
                    "model-download-progress",
                    ModelDownloadProgress {
                        model_id: progress_model.clone(),
                        downloaded_bytes: downloaded,
                        total_bytes,
                    },
                );
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    });

    let download_model = model_id.clone();
    let result = tokio::task::spawn_blocking(move || transcriber.download_model(&download_model))
        .await
        .map_err(|e| format!("Download worker panic: {}", e));
    stop.store(true, Ordering::Relaxed);
    let _ = poller.join();
    let model = result??;
    let _ = app.emit(
        "model-download-progress",
        ModelDownloadProgress {
            model_id,
            downloaded_bytes: total_bytes,
            total_bytes,
        },
    );
    Ok(model)
}

#[tauri::command]
async fn manual_inject_text(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    press_enter: bool,
    use_clipboard_paste: bool,
    delay_ms: Option<u64>,
) -> Result<InjectionOutcome, String> {
    if let Some(ms) = delay_ms {
        if ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }
    }
    let cfg = state.audio.get_config();
    let cleanup = run_local_developer_cleanup(&text, false);
    let target_app = detect_frontmost_app();
    let policy = evaluate_app_injection_policy(
        &target_app,
        press_enter,
        cfg.per_app_rules_enabled,
        cleanup.is_destructive_command,
        &cfg.auto_send_allowlist,
    );
    let mut timing = cfg.to_injection_timing();
    timing.use_clipboard_paste = use_clipboard_paste;

    match inject_text_into_focused_app(&app, &text, &policy, &timing) {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let _ = app.emit("pipeline-error", err.clone());
            Err(err)
        }
    }
}

#[tauri::command]
fn request_accessibility_settings() -> Result<(), String> {
    open_accessibility_settings()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    whisper_rs::install_logging_hooks();

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_data_dir = app_handle.path().app_data_dir().ok();
            let transcriber = Arc::new(TranscriberEngine::with_app_data_dir(app_data_dir));
            let audio_controller = Arc::new(AudioPipelineController::spawn(
                app_handle.clone(),
                Arc::clone(&transcriber),
            ));

            let audio_for_shortcut = Arc::clone(&audio_controller);
            let initial_hotkey = audio_controller
                .get_config()
                .hotkey_accelerator
                .parse::<Shortcut>()
                .unwrap_or_else(|_| Shortcut::new(Some(Modifiers::ALT), Code::Space));
            let current_hotkey = Arc::new(Mutex::new(initial_hotkey));
            let hotkey_for_handler = Arc::clone(&current_hotkey);
            let escape = hud::escape_shortcut();
            #[cfg(target_os = "macos")]
            let paste_last =
                Shortcut::new(Some(Modifiers::SUPER | Modifiers::CONTROL), Code::KeyV);
            #[cfg(not(target_os = "macos"))]
            let paste_last = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV);

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, sc, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        let is_hotkey = {
                            let hotkey = hotkey_for_handler.lock().unwrap();
                            sc == &*hotkey
                        };
                        if is_hotkey {
                            let _ = audio_for_shortcut.send_command(AudioCommand::ToggleListening(
                                "hotkey".to_string(),
                            ));
                        } else if sc == &escape {
                            // Only registered while in PREVIEW (see hud::spawn_preview_escape_guard).
                            let _ = audio_for_shortcut.send_command(AudioCommand::CancelPreview(
                                "Esc pressed (global)".to_string(),
                            ));
                        } else if sc == &paste_last {
                            let _ = app.emit("paste-last-requested", ());
                        }
                    })
                    .build(),
            )?;

            let initial_hotkey = current_hotkey.lock().unwrap().clone();
            if let Err(err) = app.global_shortcut().register(initial_hotkey) {
                eprintln!("Warning: could not register the push-to-talk shortcut: {}", err);
            }
            if let Err(err) = app.global_shortcut().register(paste_last) {
                eprintln!("Warning: could not register the paste-last shortcut: {}", err);
            }

            hud::spawn_preview_escape_guard(app_handle.clone(), Arc::clone(&audio_controller));

            if let Err(err) = hud::build_tray(&app_handle, Arc::clone(&audio_controller)) {
                eprintln!(
                    "Warning: tray icon unavailable ({}); closing the main window will quit.",
                    err
                );
            }

            if let Err(err) = hud::setup_hud_window(&app_handle) {
                eprintln!("Warning: {}", err);
                let _ = app_handle.emit("pipeline-error", err);
            }

            app.manage(AppState {
                audio: audio_controller,
                transcriber,
                hotkey: current_hotkey,
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != hud::MAIN_LABEL {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if hud::should_hide_on_close() {
                    // Keep running in the background: HUD pill + camera gate stay live.
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // The HUD window would otherwise keep an invisible app alive.
                    hud::request_quit(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_system_status,
            start_listening,
            stop_and_transcribe,
            cancel_listening,
            confirm_preview_now,
            cancel_preview,
            test_cleanup_pass,
            update_dictation_config,
            update_vision_telemetry,
            set_camera_gate_enabled,
            select_whisper_model,
            get_frontmost_app,
            set_ui_zoom,
            download_whisper_model,
            manual_inject_text,
            request_accessibility_settings,
            flag_false_trigger,
            clear_utterance_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building saywhen tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. } => {
                let state = app_handle.state::<AppState>();
                state.transcriber.unload();
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => hud::show_main_window(app_handle),
            _ => {}
        });
}
