// ============================================================
// Tauri Event Listeners & Initial State Loading
// ============================================================

import { state } from './state.js';
import { renderAll, renderAllExceptContent } from './app.js';

// ============================================================
// Helpers
// ============================================================

// ------------------------------------------------------------
// Check whether the user is currently focused on a text input
// ------------------------------------------------------------
function isUserTyping() {
  const tag = document.activeElement?.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA';
}

// ============================================================
// Tauri Event Setup
// ============================================================

// ------------------------------------------------------------
// Register listeners for all backend-emitted events
// ------------------------------------------------------------
export function setupTauriEvents() {
  if (!window.__TAURI__) return;
  const { listen } = window.__TAURI__.event;

  // A single new question arrived
  listen('question_added', (event) => {
    const { question } = event.payload;
    state.questions.set(question.id, question);
    if (isUserTyping()) renderAllExceptContent(); else renderAll();
  });

  // A batch of questions arrived at once
  listen('questions_batch', (event) => {
    const { questions } = event.payload;
    questions.forEach(q => state.questions.set(q.id, q));
    if (isUserTyping()) renderAllExceptContent(); else renderAll();
  });

  // One or more questions were dismissed by the backend
  listen('questions_dismissed', (event) => {
    const { question_ids } = event.payload;
    question_ids.forEach(id => {
      const q = state.questions.get(id);
      if (q) q.status = 'dismissed';
      state.answers.delete(id);
    });
    renderAll();
  });

  // A new session was registered by the IPC server
  listen('session_added', (event) => {
    const { session } = event.payload;
    state.sessions.set(session.id, session);
    if (!state.sessionOrder.includes(session.id)) {
      state.sessionOrder.push(session.id);
    }
    if (!state.activeSessionId) {
      state.activeSessionId = session.id;
    }
    renderAll();
  });

  // A session was removed.
  // keep_questions=true: auto-cleanup (in-flight wait/get may still need the data)
  // keep_questions=false: explicit X-button removal, discard question data too
  listen('session_removed', (event) => {
    const { session_id, keep_questions } = event.payload;
    if (!keep_questions) {
      // Filter by sessionId directly — session.questionIds is stale
      // (captured at session_added time, before questions were added).
      for (const [qid, q] of state.questions.entries()) {
        if (q.sessionId === session_id) {
          state.questions.delete(qid);
          state.answers.delete(qid);
        }
      }
    }
    state.sessions.delete(session_id);
    state.sessionOrder = state.sessionOrder.filter(id => id !== session_id);
    if (state.activeSessionId === session_id) {
      state.activeSessionId = state.sessionOrder[0] ?? null;
      state.activeQuestionId = null;
      state.focusedChoiceIdx = null;
    }
    renderAll();
  });
}

// ============================================================
// Initial State Loading
// ============================================================

// ------------------------------------------------------------
// Fetch the full current state from the Tauri backend
// ------------------------------------------------------------
export async function loadInitialState() {
  if (!window.__TAURI__) return;

  try {
    const data = await window.__TAURI__.core.invoke('get_state');

    if (data.questions) {
      data.questions.forEach(q => state.questions.set(q.id, q));
    }
    if (data.sessions) {
      data.sessions.forEach(s => {
        state.sessions.set(s.id, s);
        if (!state.sessionOrder.includes(s.id)) {
          state.sessionOrder.push(s.id);
        }
      });
      if (!state.activeSessionId && state.sessionOrder.length > 0) {
        state.activeSessionId = state.sessionOrder[0];
      }
    }
  } catch (err) {
    console.error('Failed to load initial state:', err);
  }
}
