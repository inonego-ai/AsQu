// ============================================================
// Session Panel Rendering
// ============================================================

import { state, esc, dotLabel } from './state.js';

function triggerRenderAll() {
  window.dispatchEvent(new CustomEvent('asqu:render-all'));
}

// ============================================================
// Render
// ============================================================

// ------------------------------------------------------------
// Render the session list in the left panel
// ------------------------------------------------------------
export function renderSessionPanel() {
  const el = document.getElementById('session-panel');
  if (!el) return;

  if (state.sessionOrder.length === 0) {
    el.innerHTML = '<div class="session-panel-empty">No sessions</div>';
    return;
  }

  let html = '<div class="session-panel-header">Sessions</div>';

  for (const sid of state.sessionOrder) {
    const session = state.sessions.get(sid);
    if (!session) continue;

    const active = sid === state.activeSessionId ? ' active' : '';
    const pendingCount = Array.from(state.questions.values())
      .filter(q => q.sessionId === sid && q.status === 'pending').length;

    const isPending = !session.displayName;
    const dotStep   = isPending ? (state.loadingSessions.get(sid) ?? 0) : 0;
    const nameText  = isPending ? dotLabel(dotStep) : esc(session.displayName);
    const titleAttr = isPending ? '' : ` title="${esc(session.displayName)}"`;

    html += `<div class="session-item${active}" data-sid="${esc(sid)}">
      <div class="session-item-name"${titleAttr}>${nameText}</div>
      ${pendingCount > 0 ? `<div class="session-item-count">${pendingCount}</div>` : ''}
      <button class="session-close" data-close-sid="${esc(sid)}" title="Remove session" aria-label="Remove session">&#10005;</button>
    </div>`;
  }

  el.innerHTML = html;

  // Wire session click (select) and close (remove) handlers
  el.querySelectorAll('.session-item').forEach(item => {
    item.addEventListener('click', (e) => {
      // Don't trigger session select when close button is clicked
      if (e.target.closest('.session-close')) return;
      const sid = item.dataset.sid;
      state.activeSessionId = state.activeSessionId === sid ? null : sid;
      state.activeQuestionId = null;
      state.focusedChoiceIdx = null;
      triggerRenderAll();
    });
  });

  el.querySelectorAll('.session-close').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const sid = btn.dataset.closeSid;
      await removeSession(sid);
    });
  });
}

// ============================================================
// Session Removal
// ============================================================

// ------------------------------------------------------------
// Invoke the Tauri remove_session command, then update local state
// ------------------------------------------------------------
async function removeSession(sessionId) {
  if (window.__TAURI__) {
    try {
      await window.__TAURI__.core.invoke('remove_session', { sessionId });
      // State cleanup is handled by the session_removed event from the backend.
    } catch (err) {
      console.error('Failed to remove session:', err);
      // Invoke failed — fall back to local cleanup so the UI stays consistent.
      cleanupSessionLocally(sessionId);
      triggerRenderAll();
    }
    return;
  }

  // No Tauri context (dev/test): clean up locally.
  cleanupSessionLocally(sessionId);
  renderAll();
}

// ------------------------------------------------------------
// Remove a session and its questions from local state
// ------------------------------------------------------------
function cleanupSessionLocally(sessionId) {
  for (const [qid, q] of state.questions.entries()) {
    if (q.sessionId === sessionId) {
      state.questions.delete(qid);
      state.answers.delete(qid);
    }
  }
  state.sessions.delete(sessionId);
  state.sessionOrder = state.sessionOrder.filter(id => id !== sessionId);
  if (state.activeSessionId === sessionId) {
    state.activeSessionId = null;
    state.activeQuestionId = null;
    state.focusedChoiceIdx = null;
  }
}
