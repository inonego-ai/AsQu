// ============================================================
// Application Entry Point & Render Pipeline
// ============================================================

import { getPendingQuestions } from './state.js';
import { renderQuestionList, renderQuestionContent } from './render-question.js';
import { renderInspector } from './render-inspector.js';
import { renderSessionPanel } from './session-panel.js';
import { setupEvents } from './events.js';
import { setupTauriEvents, loadInitialState } from './tauri-bridge.js';

// ============================================================
// Render Functions
// ============================================================

// -----------------------------------------------------------------------
// Full re-render of all UI panels (debounced — coalesces rapid calls)
// -----------------------------------------------------------------------
let _renderTimer = null;
export function renderAll() {
  if (_renderTimer) return;
  _renderTimer = requestAnimationFrame(() => {
    _renderTimer = null;
    renderSessionPanel();
    renderQuestionList();
    renderQuestionContent();
    renderInspector();
    renderStatusBar();
  });
}

// ------------------------------------------------------------
// Partial render that preserves focused inputs (e.g. textarea)
// Used when user is actively typing to avoid destroying focus
// ------------------------------------------------------------
let _renderExceptTimer = null;
export function renderAllExceptContent() {
  if (_renderExceptTimer) return;
  _renderExceptTimer = requestAnimationFrame(() => {
    _renderExceptTimer = null;
    renderSessionPanel();
    renderQuestionList();
    renderStatusBar();
  });
}

// ------------------------------------------------------------
// Update the status bar pending count
// ------------------------------------------------------------
function renderStatusBar() {
  const total = getPendingQuestions().length;
  const el = document.getElementById('status-pending');
  if (el) el.textContent = `Pending: ${total}`;
}

// ============================================================
// Initialization
// ============================================================

// ------------------------------------------------------------
// Bootstrap the application
// ------------------------------------------------------------
async function init() {
  window.addEventListener('asqu:render-all', () => renderAll());
  setupEvents();
  setupTauriEvents();
  await loadInitialState();
  renderAll();

  // Signal backend that the webview is ready to be shown
  if (window.__TAURI__) {
    await window.__TAURI__.core.invoke('notify_ready');
  }
}

init();
