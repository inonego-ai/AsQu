// ============================================================
// ipc/types.rs — IPC request and response types
// ============================================================

use serde::{Deserialize, Deserializer, Serialize};

use crate::types::{AnswerInfo, DeniedInfo, GetAnswersResult, Priority, QuestionChoice};

// ------------------------------------------------------------
// choices accepts either ["A","B"] or [{label:"A",...}]
// ------------------------------------------------------------
fn deserialize_choices<'de, D>(deserializer: D) -> Result<Option<Vec<QuestionChoice>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ChoicesInput {
        Strings(Vec<String>),
        Full(Vec<QuestionChoice>),
    }

    let opt: Option<ChoicesInput> = Option::deserialize(deserializer)?;
    Ok(opt.map(|c| match c {
        ChoicesInput::Strings(strs) => strs
            .into_iter()
            .map(|s| QuestionChoice { label: s, description: None })
            .collect(),
        ChoicesInput::Full(full) => full,
    }))
}

// ============================================================
// Ask item — one question submitted via CLI
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskItem {
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_choices",
        default
    )]
    pub choices: Option<Vec<QuestionChoice>>,

    #[serde(default = "crate::types::default_true")]
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
}

// ============================================================
// IPC Request
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum IpcRequest {
    Ask {
        session_id: String,
        /// ai-title read from transcript JSONL on the CLI side before sending.
        /// None = not yet generated or file unreadable — server keeps existing display_name.
        #[serde(skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        questions: Vec<AskItem>,
    },
    Wait {
        session_id: String,
        ids: Vec<String>,
        #[serde(default = "crate::types::default_true")]
        require_all: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_secs: Option<u64>,
    },
    Get {
        session_id: String,
        ids: Vec<String>,
    },
    Dismiss {
        session_id: String,
        ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    OpenUi,
    /// Graceful shutdown: begin_shutdown() + app.exit(0) after response drain.
    Shutdown,
    Ping,
}

// ============================================================
// IPC Response
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum IpcResponse {
    AskOk {
        ids: Vec<String>,
        pending: usize,
    },
    AnswersOk {
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        answered: Vec<AnswerInfo>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        denied: Vec<DeniedInfo>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pending: Vec<String>,
        #[serde(rename = "timedOut", skip_serializing_if = "std::ops::Not::not", default)]
        timed_out: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not", default)]
        shutdown: bool,
    },
    DismissOk {
        dismissed: Vec<String>,
    },
    UiOk,
    Pong,
    Err {
        message: String,
    },
}

impl IpcResponse {
    pub fn from_answers_result(result: GetAnswersResult) -> Self {
        Self::AnswersOk {
            answered: result.answered,
            denied: result.denied,
            pending: result.pending,
            timed_out: result.timed_out,
            shutdown: result.shutdown,
        }
    }
}
