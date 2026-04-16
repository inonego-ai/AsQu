// ============================================================
// types.rs — Core data types for AsQu
// ============================================================

use serde::{Deserialize, Serialize};

// ============================================================
// Enums
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Normal,
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionStatus {
    Pending,
    Answered,
    Dismissed,
    Denied,
}

// ============================================================
// Choice
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionChoice {
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================
// Selection Detail (per-choice metadata from the user)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ============================================================
// Answer
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionAnswer {
    /// Selected choice indices with optional details (key = index string)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub selections: std::collections::HashMap<String, SelectionDetail>,

    /// Free-text answer (freeform input or "Other..." text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

// ============================================================
// Question
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub session_id: String,
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<QuestionChoice>>,

    #[serde(default = "default_true")]
    pub allow_other: bool,

    #[serde(default)]
    pub multi_select: bool,

    #[serde(default)]
    pub instant: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    #[serde(default)]
    pub priority: Priority,

    pub status: QuestionStatus,
    pub created_at: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<QuestionAnswer>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismiss_reason: Option<String>,
}

pub(crate) fn default_true() -> bool {
    true
}

// ============================================================
// Session
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub display_name: String,
    pub created_at: u64,
    pub question_ids: Vec<String>,
}

// ============================================================
// Answer result types (shared between IPC handlers and UI)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerInfo {
    pub id: String,
    pub answer: QuestionAnswer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeniedInfo {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnswersResult {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub answered: Vec<AnswerInfo>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub denied: Vec<DeniedInfo>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pending: Vec<String>,

    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub timed_out: bool,

    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub shutdown: bool,
}

// ============================================================
// Events (IPC -> UI)
// ============================================================

#[derive(Debug)]
pub enum IpcToUiEvent {
    QuestionAdded {
        question: Question,
    },
    QuestionsBatch {
        questions: Vec<Question>,
    },
    QuestionsDismissed {
        question_ids: Vec<String>,
    },
    SessionAdded {
        session: Session,
    },
    SessionRemoved {
        session_id: String,
        /// true  = auto-cleanup (questions kept in memory for in-flight wait/get)
        /// false = explicit removal (X button), questions should be discarded
        keep_questions: bool,
    },
    SessionUpdated {
        session: Session,
    },
    ShowWindow,
    /// Graceful shutdown: wait for in-flight responses to drain, then exit.
    Shutdown,
}
