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
  } catch (err) {
    console.error('Failed to load initial state:', err);
  }
}
