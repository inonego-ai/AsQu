// ============================================================
// ui/commands.rs — Tauri commands (invoked from frontend JS)
// ============================================================

use serde_json::Value;
use tauri::{Manager, State};

use std::sync::atomic::Ordering;

use crate::state::{SharedState, WebviewReadyState};
use crate::types::QuestionAnswer;

// ============================================================
// Answer / Dismiss
// ============================================================

// ------------------------------------------------------------
// Submit an answer to a question (from UI)
// ------------------------------------------------------------
#[tauri::command]
pub fn submit_answer(
    state: State<'_, SharedState>,
    question_id: String,
    answer: QuestionAnswer,
) -> Result<bool, String> {
    let mut st = state.lock().unwrap();
    let ok = st.apply_answer(&question_id, answer);
    Ok(ok)
}

// ------------------------------------------------------------
// Dismiss a question from the UI
// ------------------------------------------------------------
#[tauri::command]
pub fn dismiss_question(
    state: State<'_, SharedState>,
    question_id: String,
    reason: Option<String>,
) -> Result<Vec<String>, String> {
    let mut st = state.lock().unwrap();
    let dismissed = st.apply_denied(
        &[question_id],
        reason.as_deref().unwrap_or("dismissed by user"),
    );
    Ok(dismissed)
}

// ============================================================
// Session Management
// ============================================================

// ------------------------------------------------------------
// Remove a session and all its questions (from UI X button)
// ------------------------------------------------------------
#[tauri::command]
pub fn remove_session(
    state: State<'_, SharedState>,
    session_id: String,
) -> Result<bool, String> {
    let mut st = state.lock().unwrap();
    let ok = st.remove_session_with_questions(&session_id);
    Ok(ok)
}

// ============================================================
// State Query
// ============================================================

// ------------------------------------------------------------
// Get full application state (for initial load)
// ------------------------------------------------------------
#[tauri::command]
pub fn get_state(state: State<'_, SharedState>) -> Result<Value, String> {
    let st = state.lock().unwrap();

    // Return questions in session insertion order, preserving question_ids order
    let questions: Vec<&crate::types::Question> = st
        .session_order
        .iter()
        .filter_map(|sid| st.sessions.get(sid))
        .flat_map(|session| {
            session
                .question_ids
                .iter()
                .filter_map(|qid| st.questions.get(qid))
        })
        .collect();

    // Return sessions in insertion order
    let sessions: Vec<&crate::types::Session> = st
        .session_order
        .iter()
        .filter_map(|id| st.sessions.get(id))
        .collect();

    Ok(serde_json::json!({
        "questions": questions,
        "sessions": sessions,
    }))
}

// ============================================================
// Webview Readiness
// ============================================================

// ------------------------------------------------------------
// Called by the frontend after initialization is complete.
// Processes any buffered show requests.
// ------------------------------------------------------------
#[tauri::command]
pub fn notify_ready(
    app: tauri::AppHandle,
    ready_state: State<'_, WebviewReadyState>,
) -> Result<(), String> {
    ready_state.ready.store(true, Ordering::Release);

    // If a show was requested before the webview was ready, process it now
    if ready_state.pending_show.swap(false, Ordering::AcqRel) {
        super::window::show_window_lazy(&app);
    }
    Ok(())
}

// ============================================================
// Window Management
// ============================================================

// ------------------------------------------------------------
// Show the main window
// ------------------------------------------------------------
#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    super::window::show_window_lazy(&app);
    Ok(())
}

// ------------------------------------------------------------
// Hide the main window
// ------------------------------------------------------------
#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    Ok(())
}
