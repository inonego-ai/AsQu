// ============================================================
// state.rs — Shared application state and event channels
// ============================================================

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;

use crate::types::{IpcToUiEvent, Question, Session};

// ============================================================
// Type Aliases
// ============================================================

/// Arc<std::sync::Mutex> — usable from both std threads and async tasks.
pub type SharedState = Arc<Mutex<AppState>>;

// ============================================================
// WebviewReadyState — Tracks whether the frontend has initialized
// ============================================================

pub struct WebviewReadyState {
    pub ready: AtomicBool,
    pub pending_show: AtomicBool,
}

// ============================================================
// AppState
// ============================================================

pub struct AppState {
    /// Sessions keyed by session_id, in insertion order via iteration over session_order
    pub sessions: HashMap<String, Session>,

    /// Ordered list of session IDs (insertion order preserved)
    pub session_order: Vec<String>,

    /// All questions across all sessions
    pub questions: HashMap<String, Question>,

    /// Channel: IPC handlers -> UI event bridge (N senders, 1 receiver in lib.rs)
    pub ipc_to_ui_tx: mpsc::UnboundedSender<IpcToUiEvent>,

    /// Condvar pair: wakes blocked wait_for_answers_sync calls when state changes.
    /// The u64 is a generation counter incremented on every answer/dismiss.
    pub state_changed: Arc<(Mutex<u64>, Condvar)>,

    /// ID generation counter
    id_counter: u32,

    /// Set to true when the app is about to exit.
    /// Causes wait_for_answers_sync to unblock immediately.
    pub shutting_down: bool,
}

impl AppState {
    // ------------------------------------------------------------
    // Constructor
    // ------------------------------------------------------------
    pub fn new() -> (Self, mpsc::UnboundedReceiver<IpcToUiEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let state = Self {
            sessions: HashMap::new(),
            session_order: Vec::new(),
            questions: HashMap::new(),
            ipc_to_ui_tx: tx,
            state_changed: Arc::new((Mutex::new(0u64), Condvar::new())),
            id_counter: 0,
            shutting_down: false,
        };

        (state, rx)
    }

    // ------------------------------------------------------------
    // Generate next question ID (simple incrementing integer)
    // ------------------------------------------------------------
    pub fn next_id(&mut self) -> String {
        self.id_counter += 1;
        self.id_counter.to_string()
    }

    // ------------------------------------------------------------
    // Notify all blocked wait_for_answers_sync callers
    // ------------------------------------------------------------
    pub fn notify_state_changed(&self) {
        let (lock, cvar) = &*self.state_changed;
        let mut counter = lock.lock().unwrap();
        *counter += 1;
        cvar.notify_all();
    }

    // ------------------------------------------------------------
    // Signal shutdown: dismiss all pending questions and unblock waiters
    // ------------------------------------------------------------
    pub fn begin_shutdown(&mut self) {
        self.shutting_down = true;
        // Dismiss all pending questions so wait_for_answers_sync returns them as denied
        let pending_ids: Vec<String> = self.questions
            .values()
            .filter(|q| q.status == crate::types::QuestionStatus::Pending)
            .map(|q| q.id.clone())
            .collect();
        for id in &pending_ids {
            if let Some(q) = self.questions.get_mut(id) {
                q.status = crate::types::QuestionStatus::Dismissed;
                q.dismiss_reason = Some("app shutdown".to_string());
            }
        }
        if !pending_ids.is_empty() {
            let _ = self.ipc_to_ui_tx.send(crate::types::IpcToUiEvent::QuestionsDismissed {
                question_ids: pending_ids,
            });
        }
        self.notify_state_changed();
    }
}

// ============================================================
// Helpers
// ============================================================

// ------------------------------------------------------------
// Current time in milliseconds since UNIX epoch
// ------------------------------------------------------------
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
