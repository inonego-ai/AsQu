// ============================================================
// Inspector Panel: Choice Details (Confidence, Notes, Preview)
// ============================================================

import { state, getAnswerState, isChoiceLocked, esc } from './state.js';
import { renderQuestionContent } from './render-question.js';

// ============================================================
// Inspector Rendering
// ============================================================

// -----------------------------------------------------------------------
// Render the inspector panel for the currently focused choice
// Shows confidence toggle + slider, and notes
// -----------------------------------------------------------------------
export function renderInspector() {
  const emptyEl = document.getElementById('inspector-empty');
  const contentEl = document.getElementById('inspector-content');
  const q = state.questions.get(state.activeQuestionId);

  // Show empty state when no choice is focused
  if (!q || state.focusedChoiceIdx === null ||
      !q.choices || !q.choices[state.focusedChoiceIdx]) {
    emptyEl.style.display = 'flex';
    contentEl.style.display = 'none';
    return;
  }

  emptyEl.style.display = 'none';
  contentEl.style.display = 'flex';

  const choice = q.choices[state.focusedChoiceIdx];
  const idx = state.focusedChoiceIdx;
  const ans = getAnswerState(q.id);
  const detail = ans.details.get(idx) || { confidenceOn: false, confidence: 100, note: '' };
  const isHistory = q.status !== 'pending';

  // Choice name header
  let html = `<div class="inspector-choice-name">&#9656; ${esc(choice.label)}</div>`;

  // Details section: confidence toggle, slider, notes
  const confVal = detail.confidenceOn ? (detail.confidence ?? 100) : null;
  html += `<div class="inspector-section">
    <div class="inspector-section-header">Details</div>
    <div class="inspector-section-body">
      <div class="confidence-row">
        <div class="confidence-top">
          <div class="toggle ${detail.confidenceOn ? 'on' : ''}" id="conf-toggle"
               ${isHistory ? '' : 'style="cursor:pointer"'}
               role="switch" aria-checked="${detail.confidenceOn}" aria-label="Toggle confidence">
            <div class="toggle-thumb"></div>
          </div>
          <span class="confidence-label">Confidence</span>
          <span class="confidence-value ${detail.confidenceOn ? 'active' : ''}" id="conf-value">${confVal !== null ? confVal + '%' : '--%'}</span>
        </div>
        <input type="range" class="confidence-slider" id="conf-slider"
               min="0" max="100" value="${detail.confidence ?? 100}"
               ${!detail.confidenceOn || isHistory ? 'disabled' : ''}
               aria-label="Confidence level">
      </div>
      <div class="note-label">Note</div>
      <textarea class="note-textarea" id="note-textarea"
                placeholder="Add a note..."
                aria-label="Choice note"
                ${isHistory ? 'readonly' : ''}>${esc(detail.note || '')}</textarea>
      ${!isHistory ? '<button class="clear-btn" id="clear-detail-btn">Clear</button>' : ''}
    </div>
  </div>`;

  contentEl.innerHTML = html;

  // Auto-resize the note textarea to fit its content
  const noteEl = document.getElementById('note-textarea');
  if (noteEl) autoResizeTextarea(noteEl);

  // In history view, inspector is read-only
  if (isHistory) return;

  wireInspectorEvents(q, idx, ans, detail);
}

// ============================================================
// Inspector Event Wiring
// ============================================================

// -----------------------------------------------------------------------
// Attach interactive handlers for confidence and note controls
// -----------------------------------------------------------------------
function wireInspectorEvents(q, idx, ans, detail) {
  // Toggle confidence on/off
  document.getElementById('conf-toggle')?.addEventListener('click', () => {
    detail.confidenceOn = !detail.confidenceOn;
    ans.details.set(idx, detail);
    autoSelectIfDetailed(q, idx);
    renderInspector();
    renderQuestionContent();
  });

  // Adjust confidence slider value
  document.getElementById('conf-slider')?.addEventListener('input', (e) => {
    detail.confidence = parseInt(e.target.value);
    document.getElementById('conf-value').textContent = detail.confidence + '%';
    ans.details.set(idx, detail);
    autoSelectIfDetailed(q, idx);
    renderQuestionContent();
  });

  // Edit the note text
  document.getElementById('note-textarea')?.addEventListener('input', (e) => {
    detail.note = e.target.value;
    ans.details.set(idx, detail);
    autoResizeTextarea(e.target);
    autoSelectIfDetailed(q, idx);
    renderQuestionContent();
  });

  // Clear all detail data for this choice
  document.getElementById('clear-detail-btn')?.addEventListener('click', () => {
    ans.details.delete(idx);
    ans.selected.delete(idx);
    renderInspector();
    renderQuestionContent();
  });
}

// ============================================================
// Auto-Selection Logic
// ============================================================

// -----------------------------------------------------------------------
// Auto-select a choice when details are added; deselect when cleared
// -----------------------------------------------------------------------
function autoSelectIfDetailed(q, idx) {
  const ans = getAnswerState(q.id);
  if (isChoiceLocked(q.id, idx)) {
    ans.selected.add(idx);
  } else {
    // Details cleared back to empty state: auto-deselect
    ans.selected.delete(idx);
  }
}

// ============================================================
// Textarea Auto-Resize
// ============================================================

// -----------------------------------------------------------------------
// Resize a textarea to fit its content, up to 200px max height
// -----------------------------------------------------------------------
function autoResizeTextarea(el) {
  el.style.height = 'auto';
  el.style.height = Math.min(el.scrollHeight, 200) + 'px';
}
