pub mod audio;
pub mod cleanup;
pub mod hud;
pub mod inject;
pub mod transcribe;
pub mod turn_taking;

use audio::{
    list_input_devices, AudioCommand, AudioPipelineController, DictationConfig,
    VisionTelemetryPayload,
};
use cleanup::{run_local_developer_cleanup, CleanupResult};
use inject::{
    check_accessibility_permission, detect_frontmost_app, evaluate_app_injection_policy,
    inject_text_into_focused_app, open_accessibility_settings, InjectionOutcome,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use transcribe::{ModelInfo, TranscriberEngine};
use turn_taking::{update_false_trigger_flag_in_metrics, EndReason};

pub struct AppState {
    pub audio: Arc<AudioPipelineController>,
    pub transcriber: Arc<TranscriberEngine>,
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
}

#[tauri::command]
fn get_system_status(state: State<'_, AppState>) -> SystemStatusSnapshot {
    SystemStatusSnapshot {
        state: state.audio.current_state(),
        devices: list_input_devices(),
        config: state.audio.get_config(),
        accessibility_trusted: check_accessibility_permission(),
        models: state.transcriber.list_models(),
        selected_model_id: state.transcriber.get_selected_model_id(),
        hotkey_label: "Option + Space".to_string(),
        frontmost_app: detect_frontmost_app().name,
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
fn test_cleanup_pass(sample_text: String) -> CleanupResult {
    run_local_developer_cleanup(&sample_text, true)
}

#[tauri::command]
fn update_dictation_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: DictationConfig,
) -> Result<DictationConfig, String> {
    let updated = state.audio.update_config(config)?;
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
async fn download_whisper_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelInfo, String> {
    let transcriber = Arc::clone(&state.transcriber);
    tokio::task::spawn_blocking(move || transcriber.download_model(&model_id))
        .await
        .map_err(|e| format!("Download worker panic: {}", e))?
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
            let transcriber = Arc::new(TranscriberEngine::new());
            let audio_controller = Arc::new(AudioPipelineController::spawn(
                app_handle.clone(),
                Arc::clone(&transcriber),
            ));

            let audio_for_shortcut = Arc::clone(&audio_controller);
            let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            let escape = hud::escape_shortcut();

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, sc, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        if sc == &shortcut {
                            let _ = audio_for_shortcut.send_command(AudioCommand::ToggleListening(
                                "hotkey (Option+Space)".to_string(),
                            ));
                        } else if sc == &escape {
                            // Only registered while in PREVIEW (see hud::spawn_preview_escape_guard).
                            let _ = audio_for_shortcut.send_command(AudioCommand::CancelPreview(
                                "Esc pressed (global)".to_string(),
                            ));
                        }
                    })
                    .build(),
            )?;

            if let Err(err) = app.global_shortcut().register(shortcut) {
                eprintln!("Warning: could not register Option+Space shortcut: {}", err);
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
            download_whisper_model,
            manual_inject_text,
            request_accessibility_settings,
            flag_false_trigger,
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
