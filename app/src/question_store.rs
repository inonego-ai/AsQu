// ============================================================
// question_store.rs — Question and session lifecycle
// ============================================================
//
// All question mutations happen through AppState (behind SharedState Mutex).
// wait_for_answers_sync is a free blocking function that releases the Mutex
// between checks, using a Condvar generation counter to avoid missed wakeups.
// ============================================================

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::ipc::types::AskItem;
use crate::state::{now_millis, AppState, SharedState};
use crate::types::{
    AnswerInfo, DeniedInfo, GetAnswersResult, IpcToUiEvent, Question, QuestionAnswer,
    QuestionStatus, Session,
};

// ============================================================
// Session Operations (on AppState, called with Mutex held)
// ============================================================

impl AppState {
    // ------------------------------------------------------------
    // Ensure a session exists; create it if not. Returns display_name.
    // ------------------------------------------------------------
    pub fn ensure_session(&mut self, session_id: &str, display_name: Option<String>) -> String {
        if let Some(session) = self.sessions.get_mut(session_id) {
            // Session exists — update display_name only if a real title arrived.
            // If the session was consumed and removed by cleanup, this branch is skipped
            // and we fall through to create a fresh session below (safe).
            if let Some(name) = display_name {
                if session.display_name != name {
                    session.display_name = name.clone();
                    let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::SessionUpdated {
                        session: session.clone(),
                    });
                }
            }
            session.display_name.clone()
        } else {
            // Session absent (never created, or cleaned up after all questions resolved).
            // Create fresh — any in-flight wait/get calls use explicit IDs, so no data loss.
            let name = display_name.unwrap_or_else(|| derive_display_name(session_id));
            let session = Session {
                id: session_id.to_string(),
                display_name: name.clone(),
                created_at: now_millis(),
                question_ids: Vec::new(),
            };
            self.sessions.insert(session_id.to_string(), session.clone());
            self.session_order.push(session_id.to_string());
            let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::SessionAdded { session });
            name
        }
    }

    // ------------------------------------------------------------
    // Remove a session from the UI panel.
    // Questions are intentionally kept in state.questions so that
    // in-flight wait/get calls can still read their answers.
    // ------------------------------------------------------------
    pub fn remove_session(&mut self, session_id: &str) -> bool {
        if self.sessions.remove(session_id).is_none() {
            return false;
        }
        self.session_order.retain(|id| id != session_id);

        let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::SessionRemoved {
            session_id: session_id.to_string(),
            keep_questions: true,
        });
        true
    }

    // ------------------------------------------------------------
    // Remove a session AND its questions (explicit UI X-button).
    // Should not be used for auto-cleanup — only for manual removal.
    // ------------------------------------------------------------
    pub fn remove_session_with_questions(&mut self, session_id: &str) -> bool {
        if self.sessions.remove(session_id).is_none() {
            return false;
        };
        self.session_order.retain(|id| id != session_id);

        // Dismiss pending questions so wait_for_answers_sync unblocks immediately.
        // Questions are kept in state (not deleted) for in-flight wait callers.
        let pending_ids: Vec<String> = self.questions
            .values()
            .filter(|q| q.session_id == session_id && q.status == QuestionStatus::Pending)
            .map(|q| q.id.clone())
            .collect();
        self.dismiss_questions(&pending_ids, Some("session removed"));

        let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::SessionRemoved {
            session_id: session_id.to_string(),
            keep_questions: false,
        });
        true
    }

    // ------------------------------------------------------------
    // Remove sessions where every question has been consumed
    // (answered, dismissed, or denied — nothing pending remains)
    // Called automatically after any state-changing operation.
    // ------------------------------------------------------------
    fn cleanup_consumed_sessions(&mut self) {
        let consumed: Vec<String> = self.session_order
            .iter()
            .filter(|sid| {
                if let Some(session) = self.sessions.get(*sid) {
                    !session.question_ids.is_empty()
                        && session.question_ids.iter().all(|qid| {
                            self.questions
                                .get(qid)
                                .map(|q| q.status != QuestionStatus::Pending)
                                .unwrap_or(true)
                        })
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        for sid in consumed {
            self.remove_session(&sid);
        }
    }
}

// ------------------------------------------------------------
// Derive a short display name from a session ID
// ------------------------------------------------------------
fn derive_display_name(_session_id: &str) -> String {
    // Return empty string — UI shows a loading animation until ai-title arrives.
    String::new()
}

// ============================================================
// Question Operations (on AppState, called with Mutex held)
// ============================================================

impl AppState {
    // ------------------------------------------------------------
    // Add a question to a session (ensures session exists)
    // ------------------------------------------------------------
    pub fn add_question_to_session(&mut self, session_id: &str, display_name: Option<String>, item: AskItem) -> Question {
        self.ensure_session(session_id, display_name);

        let id = self.next_id();

        let question = Question {
            id: id.clone(),
            session_id: session_id.to_string(),
            text: item.text,
            header: item.header,
            choices: item.choices,
            allow_other: item.allow_other,
            multi_select: item.multi_select,
            instant: item.instant,
            context: item.context,
            category: item.category,
            priority: item.priority,
            status: QuestionStatus::Pending,
            created_at: now_millis(),
            answered_at: None,
            answer: None,
            dismiss_reason: None,
        };

        self.questions.insert(id.clone(), question.clone());

        if let Some(session) = self.sessions.get_mut(session_id) {
            session.question_ids.push(id);
        }

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

        self.notify_state_changed();
        self.cleanup_consumed_sessions();
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
            self.notify_state_changed();
            self.cleanup_consumed_sessions();
        }
        denied
    }

    // ------------------------------------------------------------
    // Dismiss questions (by IPC client)
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
            self.notify_state_changed();
            let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::QuestionsDismissed {
                question_ids: dismissed.clone(),
            });
            self.cleanup_consumed_sessions();
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
                // Both Denied (UI dismiss button) and Dismissed (IPC dismiss command)
                // are returned as denied — the caller's question was cancelled either way.
                QuestionStatus::Denied | QuestionStatus::Dismissed => {
                    denied.push(DeniedInfo {
                        id: q.id.clone(),
                        reason: q.dismiss_reason.clone().unwrap_or_default(),
                    });
                }
                QuestionStatus::Pending => {
                    pending.push(q.id.clone());
                }
            }
        }

        GetAnswersResult {
            answered,
            denied,
            pending,
            timed_out: false,
            shutdown: false,
        }
    }

    // ------------------------------------------------------------
    // Get all question IDs belonging to a session
    // ------------------------------------------------------------
    pub fn get_session_question_ids(&self, session_id: &str) -> Vec<String> {
        self.sessions
            .get(session_id)
            .map(|s| s.question_ids.clone())
            .unwrap_or_default()
    }

    // ------------------------------------------------------------
    // Resolve question IDs (caller holds the lock).
    // Uses provided ids if non-empty; falls back to session question_ids;
    // then falls back to searching questions by session_id field.
    // ------------------------------------------------------------
    pub fn resolve_question_ids(&self, session_id: &str, ids: Vec<String>) -> Vec<String> {
        if !ids.is_empty() {
            return ids;
        }
        let from_session = self.get_session_question_ids(session_id);
        if !from_session.is_empty() {
            return from_session;
        }
        let mut fallback: Vec<String> = self.questions
            .values()
            .filter(|q| q.session_id == session_id)
            .map(|q| q.id.clone())
            .collect();
        fallback.sort();
        fallback
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
    // Emit IPC -> UI event for new question(s)
    // ------------------------------------------------------------
    pub fn emit_question_added(&self, question: &Question) {
        let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::QuestionAdded {
            question: question.clone(),
        });
    }

    pub fn emit_questions_batch(&self, questions: &[Question]) {
        let _ = self.ipc_to_ui_tx.send(IpcToUiEvent::QuestionsBatch {
            questions: questions.to_vec(),
        });
    }
}

// ============================================================
// Blocking Wait (free sync function — releases Mutex between checks)
// ============================================================

// ------------------------------------------------------------
// Check if a wait condition is resolved.
// An instant question being answered always resolves the wait,
// regardless of require_all — it signals a blocking decision was made.
// ------------------------------------------------------------
fn is_resolved(
    result: &GetAnswersResult,
    require_all: bool,
    questions: &std::collections::HashMap<String, crate::types::Question>,
) -> bool {
    // If any answered question is marked instant, resolve immediately
    if result.answered.iter().any(|a| {
        questions.get(&a.id).map(|q| q.instant).unwrap_or(false)
    }) {
        return true;
    }

    if require_all {
        result.pending.is_empty()
    } else {
        !result.answered.is_empty() || !result.denied.is_empty()
    }
}

// ------------------------------------------------------------
// Wait for answers with optional timeout.
// Uses Condvar generation counter to avoid missed wakeups.
// Safe to call from std::thread (does not require tokio runtime).
// ------------------------------------------------------------
pub fn wait_for_answers_sync(
    shared: &SharedState,
    question_ids: &[String],
    require_all: bool,
    timeout_secs: Option<u64>,
) -> GetAnswersResult {
    let deadline = timeout_secs.map(|s| Instant::now() + Duration::from_secs(s));

    // Capture Arc<(Mutex<u64>, Condvar)> while holding AppState lock
    let state_changed = {
        let state = shared.lock().unwrap();
        let result = state.get_answers(question_ids);
        if is_resolved(&result, require_all, &state.questions) {
            return result;
        }
        Arc::clone(&state.state_changed)
    };

    loop {
        // Timeout check
        if let Some(d) = deadline {
            if Instant::now() >= d {
                let state = shared.lock().unwrap();
                let mut result = state.get_answers(question_ids);
                result.timed_out = true;
                return result;
            }
        }

        // Check condition + capture current generation under AppState lock.
        //
        // LOCK ORDER: AppState mutex → gen_lock. notify_state_changed() also
        // acquires gen_lock while the caller holds AppState. Both sites must
        // always follow this order — never acquire AppState while holding gen_lock.
        //
        // We read gen while holding AppState to ensure atomicity: if a notify
        // fires between the condition check and the condvar wait, the gen will
        // have already incremented and the `*gen_guard != saved_gen` check below
        // will catch it without blocking. Do NOT separate these two lock sites.
        let saved_gen = {
            let state = shared.lock().unwrap();
            // App is shutting down — return whatever we have immediately
            if state.shutting_down {
                let mut result = state.get_answers(question_ids);
                result.shutdown = true;
                return result;
            }
            let result = state.get_answers(question_ids);
            if is_resolved(&result, require_all, &state.questions) {
                return result;
            }
            let (gen_lock, _) = &*state_changed;
            *gen_lock.lock().unwrap()
        }; // AppState released here

        // Wait on condvar until generation changes (or timeout)
        let (gen_lock, cvar) = &*state_changed;
        let gen_guard = gen_lock.lock().unwrap();

        if *gen_guard != saved_gen {
            // A notification fired while we didn't hold gen_lock — re-check immediately
            continue;
        }

        match deadline {
            None => {
                drop(cvar.wait(gen_guard).unwrap());
            }
            Some(d) => {
                let remaining = d.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    continue;
                }
                let _ = cvar.wait_timeout(gen_guard, remaining).unwrap();
            }
        }
    }
}

