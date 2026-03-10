// ============================================================
// UI Event Handlers (Keyboard, Click, Navigation)
// ============================================================

import { state, getPendingQuestions, getAnswerState } from './state.js';
import { handleChoiceClick } from './render-question.js';
import { renderAll } from './app.js';

// ============================================================
// Submit & Dismiss Actions
// ============================================================

// ------------------------------------------------------------
// Submit the current answer for the active question
// ------------------------------------------------------------
export async function submitCurrentAnswer() {
  const q = state.questions.get(state.activeQuestionId);
  if (!q) return;

  const ans = getAnswerState(q.id);
  const answer = { selections: {}, text: undefined };

  // Build selections map: key = index string, value = detail object
  ans.selected.forEach(idx => {
    const d = ans.details.get(idx);
    const detail = {};
    if (d) {
      if (d.confidenceOn) detail.confidence = d.confidence / 100;
      if (d.note && d.note.trim()) detail.note = d.note;
    }
    answer.selections[String(idx)] = detail;
  });

  // Unified text field (from "Other..." or freeform)
  if (ans.text && ans.text.trim()) {
    answer.text = ans.text;
  }

  try {
    await window.__TAURI__.core.invoke('submit_answer', {
      questionId: q.id,
      answer,
    });
    q.status = 'answered';
    q.answeredAt = Date.now();
    q.answer = answer;
    state.answers.delete(q.id);
    state.activeQuestionId = null;
    state.focusedChoiceIdx = null;
    renderAll();
  } catch (err) {
    console.error('Submit failed:', err);
  }
}

// ------------------------------------------------------------
// Dismiss the current active question
// ------------------------------------------------------------
export async function dismissCurrentQuestion() {
  const q = state.questions.get(state.activeQuestionId);
  if (!q) return;

  if (window.__TAURI__) {
    try {
      await window.__TAURI__.core.invoke('dismiss_question', { questionId: q.id });
    } catch (e) {
      console.error('Failed to dismiss:', e);
    }
  }

  q.status = 'dismissed';
  state.answers.delete(q.id);
  state.activeQuestionId = null;
  state.focusedChoiceIdx = null;
  renderAll();
}

// ============================================================
// Theme
// ============================================================

// ------------------------------------------------------------
// Initialize and handle theme switching
// ------------------------------------------------------------
function setupTheme() {
  const saved = localStorage.getItem('asqu-theme') || '';
  if (saved) {
    document.documentElement.setAttribute('data-theme', saved);
  }
}

// ============================================================
// Event Setup Entry Point
// ============================================================

// ------------------------------------------------------------
// Wire up all UI event listeners
// ------------------------------------------------------------
export function setupEvents() {
  setupTheme();
  setupButtons();
  setupSettingsOverlay();
  setupWindowControls();
  setupKeyboard();
  setupContextMenu();
}

// ============================================================
// Submit & Dismiss Buttons
// ============================================================

// ------------------------------------------------------------
// Attach click handlers to submit and dismiss buttons
// ------------------------------------------------------------
function setupButtons() {
  document.getElementById('btn-submit')?.addEventListener('click', submitCurrentAnswer);
  document.getElementById('btn-dismiss-q')?.addEventListener('click', dismissCurrentQuestion);
}

// ============================================================
// Settings Overlay
// ============================================================

// ------------------------------------------------------------
// Open, close settings
// ------------------------------------------------------------
function setupSettingsOverlay() {
  const overlay = document.getElementById('settings-overlay');

  document.getElementById('btn-settings')?.addEventListener('click', () => {
    overlay.classList.add('open');
    const current = localStorage.getItem('asqu-theme') || '';
    document.querySelectorAll('.theme-btn').forEach(b => {
      b.classList.toggle('active', b.dataset.theme === current);
    });
  });

  document.getElementById('btn-settings-close')?.addEventListener('click', () => {
    overlay.classList.remove('open');
  });

  overlay?.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) e.currentTarget.classList.remove('open');
  });

  // Theme selection
  document.getElementById('theme-options')?.addEventListener('click', (e) => {
    const btn = e.target.closest('.theme-btn');
    if (!btn) return;

    const theme = btn.dataset.theme;

    if (theme) {
      document.documentElement.setAttribute('data-theme', theme);
    } else {
      document.documentElement.removeAttribute('data-theme');
    }

    localStorage.setItem('asqu-theme', theme);

    document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
  });
}

// ============================================================
// Window Controls
// ============================================================

// ------------------------------------------------------------
// Minimize and close buttons (both hide the window)
// ------------------------------------------------------------
function setupWindowControls() {
  document.getElementById('btn-minimize')?.addEventListener('click', async () => {
    try { await window.__TAURI__.core.invoke('hide_window'); } catch { /* no-op */ }
  });

  document.getElementById('btn-close')?.addEventListener('click', async () => {
    try { await window.__TAURI__.core.invoke('hide_window'); } catch { /* no-op */ }
  });
}

// ============================================================
// Keyboard Shortcuts
// ============================================================

// ------------------------------------------------------------
// Global keyboard handler for navigation and quick actions
// Keys: 1-9 = select choice, Enter = submit, Escape = dismiss,
//       Arrow left/right or up/down = navigate between questions
// ------------------------------------------------------------
function setupKeyboard() {
  document.addEventListener('keydown', (e) => {
    const q = state.questions.get(state.activeQuestionId);
    if (!q || q.status !== 'pending') return;

    // Don't capture keys when typing in an input or textarea
    const tag = document.activeElement?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA') {
      if (e.key === 'Escape') document.activeElement.blur();
      return;
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submitCurrentAnswer();
    } else if (e.key === 'Escape') {
      dismissCurrentQuestion();
    } else if (e.key >= '1' && e.key <= '9') {
      const idx = parseInt(e.key) - 1;
      if (q.choices && idx < q.choices.length) {
        handleChoiceClick(q, idx);
      }
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
      navigateQuestion(-1);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      navigateQuestion(1);
    }
  });
}

// ============================================================
// Question Navigation
// ============================================================

// ------------------------------------------------------------
// Move to the previous or next pending question
// ------------------------------------------------------------
function navigateQuestion(direction) {
  const pending = getPendingQuestions();
  const curIdx = pending.findIndex(p => p.id === state.activeQuestionId);
  const nextIdx = curIdx + direction;

  if (nextIdx >= 0 && nextIdx < pending.length) {
    state.activeQuestionId = pending[nextIdx].id;
    state.focusedChoiceIdx = null;
    renderAll();
  }
}

// ============================================================
// Context Menu (Right-Click)
// ============================================================

// ------------------------------------------------------------
// Block default context menu globally;
// Show custom menu on choice right-click to toggle multi-select
// ------------------------------------------------------------
function setupContextMenu() {
  // Block browser default context menu everywhere
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
}
