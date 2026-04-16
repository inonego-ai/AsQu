// ============================================================
// lib.rs — Tauri setup, IPC integration, event bridge
// ============================================================

mod cli;
mod ipc;
mod question_store;
mod state;
mod types;
mod ui;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

use state::{AppState, SharedState, WebviewReadyState};
use types::IpcToUiEvent;

// ============================================================
// Public API
// ============================================================

pub fn run_gui() {
    run_tauri();
}

pub fn run_cli(args: Vec<String>) {
    cli::run_cli(args);
}

// ============================================================
// UI Event Listener (bridges IPC events to Tauri frontend)
// ============================================================

// ------------------------------------------------------------
// Listen for IPC -> UI events and emit Tauri events
// ------------------------------------------------------------
async fn ui_event_listener(
    app: tauri::AppHandle,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<IpcToUiEvent>,
) {
    while let Some(event) = rx.recv().await {
        match event {
            IpcToUiEvent::QuestionAdded { question } => {
                let _ = app.emit("question_added", serde_json::json!({
                    "question": question,
                }));
                ui::window::show_window_lazy(&app);
            }
            IpcToUiEvent::QuestionsBatch { questions } => {
                let _ = app.emit("questions_batch", serde_json::json!({
                    "questions": questions,
                }));
                ui::window::show_window_lazy(&app);
            }
            IpcToUiEvent::QuestionsDismissed { question_ids } => {
                let _ = app.emit("questions_dismissed", serde_json::json!({
                    "question_ids": question_ids,
                }));
            }
            IpcToUiEvent::SessionAdded { session } => {
                let _ = app.emit("session_added", serde_json::json!({
                    "session": session,
                }));
            }
            IpcToUiEvent::SessionUpdated { session } => {
                let _ = app.emit("session_updated", serde_json::json!({
                    "session": session,
                }));
            }
            IpcToUiEvent::SessionRemoved { session_id, keep_questions } => {
                let _ = app.emit("session_removed", serde_json::json!({
                    "session_id": session_id,
                    "keep_questions": keep_questions,
                }));
            }
            IpcToUiEvent::ShowWindow => {
                ui::window::show_window_lazy(&app);
            }
            IpcToUiEvent::Shutdown => {
                // Give in-flight wait responses 200ms to reach their callers before exit.
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                app.exit(0);
            }
        }
    }
    info!("UI event channel closed");
}

// ============================================================
// Tauri application setup
// ============================================================

// ------------------------------------------------------------
// Build and run the Tauri application
// ------------------------------------------------------------
#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn run_tauri() {
    // Initialize logging (to stderr)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!("AsQu starting (GUI mode)...");

    // Create shared state
    let (app_state, ipc_to_ui_rx) = AppState::new();
    let shared_state: SharedState = Arc::new(Mutex::new(app_state));

    // Start IPC server (Named Pipe) in background thread
    ipc::server::start_ipc_server(Arc::clone(&shared_state));

    // Build and run Tauri app
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(shared_state.clone())
        .manage(WebviewReadyState {
            ready: std::sync::atomic::AtomicBool::new(false),
            pending_show: std::sync::atomic::AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            ui::commands::submit_answer,
            ui::commands::dismiss_question,
            ui::commands::get_state,
            ui::commands::notify_ready,
            ui::commands::show_window,
            ui::commands::hide_window,
            ui::commands::remove_session,
        ])
        .setup(move |app| {
            // Close handler: hide window instead of exiting
            ui::window::setup_close_handler(app.handle());

            // System tray: right-click → Quit
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;
            let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
                tauri::image::Image::new(&[], 0, 0)
            });
            TrayIconBuilder::new()
                .icon(icon)
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        // Unblock any in-flight wait_for_answers_sync before exiting
                        if let Some(state) = app.try_state::<SharedState>() {
                            state.lock().unwrap().begin_shutdown();
                        }
                        // Brief pause so CLI wait callers can drain their IPC response
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // Left-click: show the window
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        ui::window::show_window_lazy(app);
                    }
                })
                .build(app)?;

            // Spawn UI event listener
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(ui_event_listener(app_handle, ipc_to_ui_rx));

            info!("AsQu setup complete — IPC server running on \\\\.\\pipe\\asqu");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to run AsQu");
}
