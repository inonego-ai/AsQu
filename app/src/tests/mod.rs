// ============================================================
// tests/mod.rs — Shared test helpers and submodule declarations
// ============================================================

pub mod handlers;
pub mod question_store;

// ============================================================
// Shared helpers
// ============================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ipc::types::AskItem;
use crate::state::{AppState, SharedState};
use crate::types::{Priority, QuestionAnswer, SelectionDetail};

// ------------------------------------------------------------
// Create a fresh SharedState for tests
// ------------------------------------------------------------
pub fn make_state() -> SharedState {
    let (state, _rx) = AppState::new();
    Arc::new(Mutex::new(state))
}

// ------------------------------------------------------------
// Minimal AskItem with only text set
// ------------------------------------------------------------
pub fn ask_item(text: &str) -> AskItem {
    AskItem {
        text: text.to_string(),
        header: None,
        choices: None,
        allow_other: true,
        multi_select: false,
        instant: false,
        context: None,
        category: None,
        priority: Priority::Normal,
    }
}

// ------------------------------------------------------------
// Minimal answer selecting the first choice
// ------------------------------------------------------------
pub fn answer() -> QuestionAnswer {
    let mut selections = HashMap::new();
    selections.insert("0".to_string(), SelectionDetail { confidence: None, note: None });
    QuestionAnswer { selections, text: None }
}
