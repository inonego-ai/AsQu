// ============================================================
// types.rs — Core data types for AsQu
// ============================================================

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================
// Enums
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

// ============================================================
// Selection Detail (per-choice metadata from the user)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

fn default_true() -> bool {
    true
}

// ============================================================
// MCP Tool Result Types
// ============================================================

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnswerInfo {
    pub id: String,
    pub answer: QuestionAnswer,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeniedInfo {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
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
}

// ============================================================
// MCP Tool Response Types (for structured output schemas)
// ============================================================

/// Response from the `ask` tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AskResponse {
    /// IDs of the newly created questions
    pub ids: Vec<String>,

    /// Total number of pending questions
    pub pending: usize,

    /// Instant answers delivered with this response
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub instant_answers: Vec<AnswerInfo>,
}

/// Response from the `get_answers` and `wait_for_answers` tools
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnswersResponse {
    /// Questions that have been answered
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub answered: Vec<AnswerInfo>,

    /// Questions that were denied by the user
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub denied: Vec<DeniedInfo>,

    /// Question IDs still pending
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pending: Vec<String>,

    /// Whether the wait timed out
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub timed_out: bool,

    /// Instant answers delivered with this response
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub instant_answers: Vec<AnswerInfo>,
}

impl AnswersResponse {
    pub fn from_result(result: GetAnswersResult, instant: Vec<AnswerInfo>) -> Self {
        Self {
            answered: result.answered,
            denied: result.denied,
            pending: result.pending,
            timed_out: result.timed_out,
            instant_answers: instant,
        }
    }
}

/// Question summary for the `list_questions` tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionSummary {
    pub id: String,
    pub text: String,
    pub status: QuestionStatus,
    pub created_at: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<u64>,
}

/// Response from the `list_questions` tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListQuestionsResponse {
    pub questions: Vec<QuestionSummary>,
    pub total: usize,

    /// Instant answers delivered with this response
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub instant_answers: Vec<AnswerInfo>,
}

/// Response from the `dismiss_questions` tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DismissResponse {
    /// IDs of successfully dismissed questions
    pub dismissed: Vec<String>,

    /// IDs that were not found or not pending
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub not_found: Vec<String>,

    /// Instant answers delivered with this response
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub instant_answers: Vec<AnswerInfo>,
}

/// Response from the `open_ui` tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct OpenUiResponse {
    pub ok: bool,
}

// ============================================================
// Events (MCP -> UI)
// ============================================================

#[derive(Debug)]
pub enum McpToUiEvent {
    QuestionAdded {
        question: Question,
    },
    QuestionsBatch {
        questions: Vec<Question>,
    },
    QuestionsDismissed {
        question_ids: Vec<String>,
    },
    ShowWindow,
}
