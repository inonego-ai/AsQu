// ============================================================
// ipc/handlers.rs — IPC request dispatching
// ============================================================

use std::sync::{Arc, Mutex};

use inoipc::IpcConnection;
use inoipc::transport::NamedPipeTransport;

use crate::question_store::wait_for_answers_sync;
use crate::state::AppState;
use crate::types::IpcToUiEvent;

use super::types::{IpcRequest, IpcResponse};

// ----------------------------------------------------------------------------
// Handle one connected client: read requests in a loop until disconnect.
// ----------------------------------------------------------------------------
pub fn handle_connection(
    mut conn: IpcConnection<NamedPipeTransport>,
    state: Arc<Mutex<AppState>>,
) {
    loop {
        let json = match conn.receive() {
            Ok(j) => j,
            Err(_) => break,
        };

        let request: IpcRequest = match serde_json::from_str(&json) {
            Ok(r) => r,
            Err(e) => {
                let resp = IpcResponse::Err {
                    message: format!("invalid request: {e}"),
                };
                send_response(&mut conn, &resp);
                break;
            }
        };

        let response = dispatch(request, &state);
        send_response(&mut conn, &response);
    }
}

// ------------------------------------------------------------
// Dispatch a request to the appropriate handler
// ------------------------------------------------------------
fn dispatch(req: IpcRequest, state: &Arc<Mutex<AppState>>) -> IpcResponse {
    match req {
        IpcRequest::Ask {
            session_id,
            display_name,
            questions,
        } => handle_ask(state, &session_id, display_name, questions),

        IpcRequest::Wait {
            session_id,
            ids,
            require_all,
            timeout_secs,
        } => handle_wait(state, &session_id, ids, require_all, timeout_secs),

        IpcRequest::Get { session_id, ids } => handle_get(state, &session_id, ids),

        IpcRequest::Dismiss {
            session_id,
            ids,
            reason,
        } => handle_dismiss(state, &session_id, ids, reason.as_deref()),

        IpcRequest::OpenUi => handle_open_ui(state),

        IpcRequest::Shutdown => handle_shutdown(state),

        IpcRequest::Ping => IpcResponse::Pong,
    }
}

// ------------------------------------------------------------
// Ask: create questions in state, emit UI events
// ------------------------------------------------------------
fn handle_ask(
    state: &Arc<Mutex<AppState>>,
    session_id: &str,
    display_name: Option<String>,
    items: Vec<crate::ipc::types::AskItem>,
) -> IpcResponse {
    let mut locked = state.lock().unwrap();
    let count = items.len();

    if count == 1 {
        let question = locked.add_question_to_session(session_id, display_name, items.into_iter().next().unwrap());
        locked.emit_question_added(&question);
        IpcResponse::AskOk {
            ids: vec![question.id],
            pending: locked.get_pending_count(),
        }
    } else {
        let mut questions = Vec::with_capacity(count);
        let mut dn = display_name;
        for item in items {
            // Pass display_name only on first question to avoid redundant UI events
            questions.push(locked.add_question_to_session(session_id, dn.take(), item));
        }
        let ids: Vec<String> = questions.iter().map(|q| q.id.clone()).collect();
        let pending = locked.get_pending_count();
        locked.emit_questions_batch(&questions);
        IpcResponse::AskOk { ids, pending }
    }
}

// ---------------------------------------------------------------------------------------------
// Resolve ids (acquires its own lock).
// Used by handle_wait where the double-lock is unavoidable: wait_for_answers_sync
// is a blocking loop that releases the lock between checks, so atomicity across
// resolve + wait is not possible — and not needed, since the loop catches any
// state change that occurs in the window between these two lock acquisitions.
// handle_get and handle_dismiss use the locked variant (AppState::resolve_question_ids)
// to avoid the TOCTOU entirely.
// ---------------------------------------------------------------------------------------------
pub(crate) fn resolve_ids(state: &Arc<Mutex<AppState>>, session_id: &str, ids: Vec<String>) -> Vec<String> {
    if !ids.is_empty() {
        return ids;
    }
    state.lock().unwrap().resolve_question_ids(session_id, ids)
}

// ------------------------------------------------------------
// Wait: block until answers arrive or timeout
// ------------------------------------------------------------
fn handle_wait(
    state: &Arc<Mutex<AppState>>,
    session_id: &str,
    ids: Vec<String>,
    require_all: bool,
    timeout_secs: Option<u64>,
) -> IpcResponse {
    let effective_ids = resolve_ids(state, session_id, ids);
    let result = wait_for_answers_sync(state, &effective_ids, require_all, timeout_secs);
    IpcResponse::from_answers_result(result)
}

// -----------------------------------------------------------------------
// Get: non-blocking snapshot of current answer state
// Single lock acquisition covers both resolve and read — no TOCTOU.
// -----------------------------------------------------------------------
fn handle_get(
    state: &Arc<Mutex<AppState>>,
    session_id: &str,
    ids: Vec<String>,
) -> IpcResponse {
    let locked = state.lock().unwrap();
    let effective_ids = locked.resolve_question_ids(session_id, ids);
    let result = locked.get_answers(&effective_ids);
    IpcResponse::from_answers_result(result)
}

// ----------------------------------------------------------------------------------
// Dismiss: cancel pending questions
// Single lock acquisition covers both resolve and mutation — no TOCTOU.
// ----------------------------------------------------------------------------------
fn handle_dismiss(
    state: &Arc<Mutex<AppState>>,
    session_id: &str,
    ids: Vec<String>,
    reason: Option<&str>,
) -> IpcResponse {
    let mut locked = state.lock().unwrap();
    let effective_ids = locked.resolve_question_ids(session_id, ids);
    let dismissed = locked.dismiss_questions(&effective_ids, reason);
    IpcResponse::DismissOk { dismissed }
}

// ------------------------------------------------------------
// OpenUi: show the window
// ------------------------------------------------------------
fn handle_open_ui(state: &Arc<Mutex<AppState>>) -> IpcResponse {
    let locked = state.lock().unwrap();
    let _ = locked.ipc_to_ui_tx.send(IpcToUiEvent::ShowWindow);
    IpcResponse::UiOk
}

// ----------------------------------------------------------------------------------------
// Shutdown: begin_shutdown unblocks all waiters, then signal Tauri to exit.
// The 200ms drain window (in the UI event listener) lets in-flight wait
// responses reach their callers before the process exits.
// ----------------------------------------------------------------------------------------
fn handle_shutdown(state: &Arc<Mutex<AppState>>) -> IpcResponse {
    let mut locked = state.lock().unwrap();
    locked.begin_shutdown();
    let _ = locked.ipc_to_ui_tx.send(IpcToUiEvent::Shutdown);
    IpcResponse::Pong
}

// ------------------------------------------------------------
// Serialize and send a response over the pipe
// ------------------------------------------------------------
fn send_response(conn: &mut IpcConnection<NamedPipeTransport>, resp: &IpcResponse) {
    let json = match serde_json::to_string(resp) {
        Ok(j) => j,
        Err(_) => return,
    };
    let _ = conn.send(&json);
}

