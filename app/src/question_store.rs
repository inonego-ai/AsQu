// ============================================================
// question_store.rs — Question lifecycle and waiter queue
// ============================================================
//
// All question mutations happen through AppState (behind SharedState Mutex).
// The wait_for_answers function is a free async fn that releases the Mutex
// while waiting on Notify, avoiding deadlocks.
// ============================================================

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

use crate::state::{now_millis, SharedState};
use crate::types::{
    AnswerInfo, DeniedInfo, GetAnswersResult, McpToUiEvent, Priority, Question,
    QuestionAnswer, QuestionStatus,
};

// ============================================================
// Question Operations (on AppState, called with Mutex held)
// ============================================================

use crate::state::AppState;

impl AppState {
    // ------------------------------------------------------------
    // Add a new question
    // ------------------------------------------------------------
    pub fn add_question(
        &mut self,
        session_id: &str,
        text: String,
        header: Option<String>,
        choices: Option<Vec<crate::types::QuestionChoice>>,
        allow_other: bool,
        multi_select: bool,
        instant: bool,
        context: Option<String>,
        category: Option<String>,
        priority: Priority,
    ) -> Question {
        let id = self.next_id();

        let question = Question {
            id: id.clone(),
            session_id: session_id.to_string(),
            text,
            header,
            choices,
            allow_other,
            multi_select,
            instant,
            context,
            category,
            priority,
            status: QuestionStatus::Pending,
            created_at: now_millis(),
            answered_at: None,
            answer: None,
            dismiss_reason: None,
        };

        self.questions.insert(id, question.clone());
        question
    }

    // ------------------------------------------------------------
    // Apply an answer to a pending question
    // ------------------------------------------------------------
    pub fn apply_answer(&mut self, question_id: &str, answer: QuestionAnswer) -> bool {
        let Some(question) = self.questions.get_mut(question_id) else {
            return false;
        };
        if question.status != QuestionStatus::Pending {
            return false;
        }

        question.status = QuestionStatus::Answered;
        question.answered_at = Some(now_millis());
        question.answer = Some(answer);

        self.state_changed.notify_waiters();
        true
    }

    // ------------------------------------------------------------
    // Mark questions as denied (by the user via UI)
    // ------------------------------------------------------------
    pub fn apply_denied(&mut self, question_ids: &[String], reason: &str) -> Vec<String> {
        let mut denied = Vec::new();
        for id in question_ids {
            if let Some(q) = self.questions.get_mut(id) {
                if q.status == QuestionStatus::Pending {
                    q.status = QuestionStatus::Denied;
                    q.dismiss_reason = Some(reason.to_string());
                    denied.push(id.clone());
                }
            }
        }
        if !denied.is_empty() {
            self.state_changed.notify_waiters();
        }
        denied
    }

    // ------------------------------------------------------------
    // Dismiss questions (by MCP client)
    // ------------------------------------------------------------
    pub fn dismiss_questions(
        &mut self,
        question_ids: &[String],
        reason: Option<&str>,
    ) -> Vec<String> {
        let mut dismissed = Vec::new();
        for id in question_ids {
            if let Some(q) = self.questions.get_mut(id) {
                if q.status == QuestionStatus::Pending {
                    q.status = QuestionStatus::Dismissed;
                    q.dismiss_reason = reason.map(|r| r.to_string());
                    dismissed.push(id.clone());
                }
            }
        }
        if !dismissed.is_empty() {
            self.state_changed.notify_waiters();

            // Emit UI event
            let _ = self.mcp_to_ui_tx.send(McpToUiEvent::QuestionsDismissed {
                question_ids: dismissed.clone(),
            });
        }
        dismissed
    }

    // ------------------------------------------------------------
    // Non-blocking: get current answers for given question IDs
    // ------------------------------------------------------------
    pub fn get_answers(&self, question_ids: &[String]) -> GetAnswersResult {
        let mut answered = Vec::new();
        let mut denied = Vec::new();
        let mut pending = Vec::new();

        for id in question_ids {
            let Some(q) = self.questions.get(id) else {
                continue;
            };
            match q.status {
                QuestionStatus::Answered => {
                    if let Some(ref answer) = q.answer {
                        answered.push(AnswerInfo {
                            id: q.id.clone(),
                            answer: answer.clone(),
                        });
                    }
                }
                QuestionStatus::Denied => {
                    denied.push(DeniedInfo {
                        id: q.id.clone(),
                        reason: q.dismiss_reason.clone().unwrap_or_default(),
                    });
                }
                QuestionStatus::Pending => {
                    pending.push(q.id.clone());
                }
                _ => {}
            }
        }

        GetAnswersResult {
            answered,
            denied,
            pending,
            timed_out: false,
        }
    }

    // ------------------------------------------------------------
    // Get questions filtered by status
    // ------------------------------------------------------------
    pub fn get_questions_by_status(&self, status: Option<QuestionStatus>) -> Vec<&Question> {
        self.questions
            .values()
            .filter(|q| status.is_none() || Some(q.status) == status)
            .collect()
    }

    // ------------------------------------------------------------
    // Count pending questions
    // ------------------------------------------------------------
    pub fn get_pending_count(&self) -> usize {
        self.questions
            .values()
            .filter(|q| q.status == QuestionStatus::Pending)
            .count()
    }

    // ------------------------------------------------------------
    // Collect undelivered instant answers
    // ------------------------------------------------------------
    pub fn collect_instant_answers(&mut self, exclude_ids: Option<&HashSet<String>>) -> Vec<AnswerInfo> {
        let mut results = Vec::new();
        for q in self.questions.values() {
            if q.instant
                && q.status == QuestionStatus::Answered
                && q.answer.is_some()
                && !self.delivered_instant_ids.contains(&q.id)
                && !exclude_ids.map_or(false, |ex| ex.contains(&q.id))
            {
                self.delivered_instant_ids.insert(q.id.clone());
                results.push(AnswerInfo {
                    id: q.id.clone(),
                    answer: q.answer.clone().unwrap(),
                });
            }
        }
        results
    }

    // ------------------------------------------------------------
    // Mark instant answers as delivered
    // ------------------------------------------------------------
    pub fn mark_instant_delivered(&mut self, ids: &[String]) {
        for id in ids {
            self.delivered_instant_ids.insert(id.clone());
        }
    }

    // ------------------------------------------------------------
    // Emit MCP -> UI event for new question(s)
    // ------------------------------------------------------------
    pub fn emit_question_added(&self, question: &Question) {
        let _ = self.mcp_to_ui_tx.send(McpToUiEvent::QuestionAdded {
            question: question.clone(),
        });
    }

    pub fn emit_questions_batch(&self, questions: &[Question]) {
        let _ = self.mcp_to_ui_tx.send(McpToUiEvent::QuestionsBatch {
            questions: questions.to_vec(),
        });
    }
}

// ============================================================
// Blocking Wait (free async function — releases Mutex)
// ============================================================

// ------------------------------------------------------------
// Check if a wait condition is resolved
// ------------------------------------------------------------
fn is_resolved(result: &GetAnswersResult, require_all: bool, state: &AppState) -> bool {
    let all_done = result.pending.is_empty();
    let any_done = !result.answered.is_empty() || !result.denied.is_empty();

    // Instant questions resolve require_all early
    let instant_done = result.answered.iter().any(|a| {
        state.questions.get(&a.id).map_or(false, |q| q.instant)
    }) || result.denied.iter().any(|d| {
        state.questions.get(&d.id).map_or(false, |q| q.instant)
    });

    if require_all {
        all_done || instant_done
    } else {
        any_done
    }
}

// ------------------------------------------------------------
// Wait for answers with optional timeout
// Releases Mutex while blocking on Notify.
// ------------------------------------------------------------
pub async fn wait_for_answers(
    shared: &SharedState,
    question_ids: Vec<String>,
    require_all: bool,
    timeout_seconds: Option<u64>,
) -> GetAnswersResult {
    // First check: already resolved?
    let state_notify: Arc<Notify>;
    let close_notify: Arc<Notify>;
    let close_flag: Arc<AtomicBool>;
    {
        let state = shared.lock().await;
        let result = state.get_answers(&question_ids);
        if is_resolved(&result, require_all, &state) {
            return result;
        }
        // Check if window was already closed before we started waiting
        if state.window_closed_flag.load(Ordering::Acquire) {
            return result;
        }
        state_notify = state.state_changed.clone();
        close_notify = state.window_closed.clone();
        close_flag = state.window_closed_flag.clone();
    }

    // Wait loop — Mutex is NOT held during await
    let deadline = timeout_seconds.map(|s| {
        std::time::Instant::now() + std::time::Duration::from_secs(s)
    });

    loop {
        // Check persistent flag before creating futures (catches missed Notify)
        if close_flag.load(Ordering::Acquire) {
            let state = shared.lock().await;
            return state.get_answers(&question_ids);
        }

        let state_future = state_notify.notified();
        let close_future = close_notify.notified();

        if let Some(deadline) = deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                let state = shared.lock().await;
                let mut result = state.get_answers(&question_ids);
                result.timed_out = true;
                return result;
            }
            tokio::select! {
                _ = state_future => {}
                _ = close_future => {
                    let state = shared.lock().await;
                    return state.get_answers(&question_ids);
                }
                _ = tokio::time::sleep(remaining) => {
                    let state = shared.lock().await;
                    let mut result = state.get_answers(&question_ids);
                    result.timed_out = true;
                    return result;
                }
            }
        } else {
            tokio::select! {
                _ = state_future => {}
                _ = close_future => {
                    let state = shared.lock().await;
                    return state.get_answers(&question_ids);
                }
            }
        }

        // Re-check after waking from state_changed
        let state = shared.lock().await;
        let result = state.get_answers(&question_ids);
        if is_resolved(&result, require_all, &state) {
            return result;
        }
    }
}
