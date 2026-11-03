pub mod audio;
pub mod cleanup;
pub mod inject;
pub mod transcribe;

use audio::{
    list_input_devices, AudioCommand, AudioPipelineController, DictationConfig,
    VisionTelemetryPayload,
};
use cleanup::{run_local_developer_cleanup, CleanupResult};
use inject::{
    check_accessibility_permission, detect_frontmost_app, inject_text_into_focused_app,
    open_accessibility_settings, InjectionOutcome,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use transcribe::{ModelInfo, TranscriberEngine};

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
        frontmost_app: detect_frontmost_app(),
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
    state.audio.send_command(AudioCommand::StopAndTranscribe)
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
    state: State<'_, AppState>,
    config: DictationConfig,
) -> Result<DictationConfig, String> {
    state.audio.update_config(config)
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
    let target_app = detect_frontmost_app();
    inject_text_into_focused_app(
        &app,
        &text,
        press_enter,
        use_clipboard_paste,
        &target_app,
        "manual_override",
    )
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

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |_app, sc, event| {
                        if sc == &shortcut && event.state() == ShortcutState::Pressed {
                            let _ = audio_for_shortcut.send_command(AudioCommand::ToggleListening(
                                "hotkey (Option+Space)".to_string(),
                            ));
                        }
                    })
                    .build(),
            )?;

            if let Err(err) = app.global_shortcut().register(shortcut) {
                eprintln!("Warning: could not register Option+Space shortcut: {}", err);
            }

            app.manage(AppState {
                audio: audio_controller,
                transcriber,
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) {
                let state = window.app_handle().state::<AppState>();
                state.transcriber.unload();
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
            select_whisper_model,
            download_whisper_model,
            manual_inject_text,
            request_accessibility_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building saywhen tauri application")
        .run(|app_handle, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                let state = app_handle.state::<AppState>();
                state.transcriber.unload();
            }
        });
}
