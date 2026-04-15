// ============================================================
// Question Display: Sidebar List, Choices, Freeform
// ============================================================

import {
  state,
  getGroupedPendingQuestions,
  getAnswerState,
  isChoiceLocked,
  isMultiSelect,
  PRIORITY_COLORS,
  esc,
  timeAgo,
} from './state.js';

import { renderInspector } from './render-inspector.js';

// ============================================================
// Question List Sidebar (Category-Grouped)
// ============================================================

// -----------------------------------------------------------------------
// Render the left sidebar with category groups and question items
// -----------------------------------------------------------------------
export function renderQuestionList() {
  const el = document.getElementById('question-sidebar');
  const groups = getGroupedPendingQuestions();
  const pending = groups.flatMap(g => g.questions);

  // Auto-select the first question if current selection is invalid
  if (!state.activeQuestionId || !pending.find(q => q.id === state.activeQuestionId)) {
    state.activeQuestionId = pending[0]?.id || null;
  }

  if (pending.length === 0) {
    el.innerHTML = '<div class="sidebar-empty">No pending questions</div>';
    return;
  }

  let html = '';
  for (const group of groups) {
    const catLabel = group.category || 'Uncategorized';

    html += `<div class="sidebar-group">
      <div class="sidebar-group-header">${esc(catLabel)}</div>`;

    for (const q of group.questions) {
      const label = q.header || [...q.text].slice(0, 24).join('');
      const color = PRIORITY_COLORS[q.priority] || PRIORITY_COLORS.normal;
      const active = q.id === state.activeQuestionId ? ' active' : '';

      html += `<div class="sidebar-item${active}" data-qid="${q.id}">
        <div class="sidebar-dot" style="background:${color}"></div>
        <div class="sidebar-label">${esc(label)}</div>
      </div>`;
    }

    html += '</div>';
  }

  el.innerHTML = html;

  // Wire click handlers
  el.querySelectorAll('.sidebar-item').forEach(item => {
    item.addEventListener('click', () => {
      state.activeQuestionId = item.dataset.qid;
      state.focusedChoiceIdx = null;
      el.querySelectorAll('.sidebar-item').forEach(t =>
        t.classList.toggle('active', t.dataset.qid === item.dataset.qid)
      );
      renderQuestionContent();
      renderInspector();
    });
  });
}

// ============================================================
// Question Content (Main Display)
// ============================================================

// ------------------------------------------------------------
// Render the full question: header, text, choices, freeform
// ------------------------------------------------------------
export function renderQuestionContent() {
  const area = document.getElementById('question-area');
  const submitBar = document.getElementById('submit-bar');
  const q = state.questions.get(state.activeQuestionId);

  // Show empty state if no valid pending question
  if (!q || q.status !== 'pending') {
    area.innerHTML = `<div class="empty-state">
      <div class="empty-state-icon">&#10003;</div>
      <div class="empty-state-text">All caught up! No pending questions.</div>
    </div>`;
    submitBar.style.display = 'none';
    return;
  }

  submitBar.style.display = 'flex';
  const ans = getAnswerState(q.id);

  // Build the question header with priority badge, instant badge, multi-select toggle, and time
  const hasChoices = q.choices && q.choices.length > 0;
  let html = `
    <div class="question-header">
      <div class="priority-badge ${q.priority}">${q.priority}</div>
      ${q.instant
        ? '<div class="instant-badge" title="Answering this question immediately unblocks the waiting agent">instant</div>'
        : ''}
      ${hasChoices
        ? `<div class="multi-toggle ${q.multiSelect ? 'on' : ''}" id="multi-toggle"
                title="Toggle multi-select">${q.multiSelect ? 'MULTI' : 'SINGLE'}</div>`
        : ''}
      <div class="question-time">${timeAgo(q.createdAt)}</div>
    </div>
    <div class="question-text">${esc(q.text)}</div>
  `;

  // Optional context block
  if (q.context) {
    html += `<div class="question-context">${esc(q.context)}</div>`;
  }

  // Render choices or freeform input
  if (q.choices && q.choices.length > 0) {
    html += renderChoicesList(q, ans);
  } else {
    html += renderFreeformInput(ans);
  }

  area.innerHTML = html;

  // Wire up interactive elements
  wireMultiToggle(q);
  wireChoiceClicks(area, q);
  wireTextInputs(q, ans);
}

// ============================================================
// Choice List Rendering
// ============================================================

// ------------------------------------------------------------
// Build HTML for the grouped choices list (single, multi, pinned)
// ------------------------------------------------------------
function renderChoicesList(q, ans) {
  let html = '<div class="choices-list">';

  // Helper to render a divider between choice groups
  const divider = (label) => `<div class="choices-divider">
    <div class="choices-divider-line"></div>
    <span class="choices-divider-label">${label}</span>
    <div class="choices-divider-line"></div>
  </div>`;

  // Group choices: pinned (locked) vs normal
  const allItems = q.choices.map((c, i) => ({ c, i }));
  const multi = isMultiSelect(q);

  const pinnedItems = allItems.filter(({ i }) => isChoiceLocked(q.id, i));
  const normalItems = allItems.filter(({ i }) => !isChoiceLocked(q.id, i));

  // Render normal choices
  if (normalItems.length > 0) {
    normalItems.forEach(({ c, i }) => { html += renderSingleChoice(q, c, i, ans, multi); });
  }

  // Render pinned choices with divider
  if (pinnedItems.length > 0) {
    if (normalItems.length > 0) html += divider('Pinned');
    pinnedItems.forEach(({ c, i }) => { html += renderSingleChoice(q, c, i, ans, true); });
  }

  html += '</div>';

  // "Other" free text input below choices
  if (q.allowOther !== false) {
    html += `<div class="other-input-wrap">
      <div class="other-label">&#9998; Other...</div>
      <textarea class="other-input" id="other-input"
                placeholder="Type your own answer...">${esc(ans.text || '')}</textarea>
    </div>`;
  }

  return html;
}

// ------------------------------------------------------------
// Render a single choice item with radio/checkbox indicator
// ------------------------------------------------------------
function renderSingleChoice(q, c, i, ans, multi) {
  const isSelected = ans.selected.has(i);
  const locked = isChoiceLocked(q.id, i);
  const focused = state.focusedChoiceIdx === i;
  const isMulti = multi || locked;

  const radioClass = `choice-radio${isMulti ? ' multi' : ''}${isSelected ? ' selected' : ''}${locked ? ' locked' : ''}`;
  const itemClass = `choice-item${isSelected ? ' selected' : ''}${focused ? ' focused' : ''}${locked ? ' locked' : ''}`;

  return `<div class="${itemClass}" data-idx="${i}">
    <div class="${radioClass}"></div>
    <div class="choice-content">
      <div class="choice-label">${esc(c.label)}</div>
      ${c.description ? `<div class="choice-desc">${esc(c.description)}</div>` : ''}
    </div>
    <div class="choice-number">${i + 1}</div>
  </div>`;
}

// ============================================================
// Freeform Input Rendering
// ============================================================

// ------------------------------------------------------------
// Render a freeform text area when no choices are present
// ------------------------------------------------------------
function renderFreeformInput(ans) {
  return `<div style="margin-top:12px;">
    <textarea class="freeform-input" id="freeform-input"
              placeholder="Type your answer...">${esc(ans.text || '')}</textarea>
  </div>`;
}

// ============================================================
// Event Wiring
// ============================================================

// ------------------------------------------------------------
// Attach click handler to the multi-select toggle badge
// ------------------------------------------------------------
function wireMultiToggle(q) {
  document.getElementById('multi-toggle')?.addEventListener('click', () => {
    q.multiSelect = !q.multiSelect;
    renderQuestionContent();
    renderInspector();
  });
}

// ------------------------------------------------------------
// Attach click handlers to all choice items
// ------------------------------------------------------------
function wireChoiceClicks(area, q) {
  area.querySelectorAll('.choice-item').forEach(item => {
    item.addEventListener('click', () => {
      handleChoiceClick(q, parseInt(item.dataset.idx));
    });
  });
}

// ------------------------------------------------------------
// Attach input handlers to "other" and freeform textareas
// ------------------------------------------------------------
function wireTextInputs(q, ans) {
  const otherInput = document.getElementById('other-input');
  if (otherInput) {
    otherInput.addEventListener('input', () => { ans.text = otherInput.value; });
    otherInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') e.stopPropagation();
    });
  }

  const freeformInput = document.getElementById('freeform-input');
  if (freeformInput) {
    freeformInput.addEventListener('input', () => { ans.text = freeformInput.value; });
  }
}

// ============================================================
// Choice Click Handler
// ============================================================

// -----------------------------------------------------------------------
// Handle selection logic for a clicked choice
// Supports single-select, multi-select, and locked (pinned) modes
// -----------------------------------------------------------------------
export function handleChoiceClick(q, idx) {
  const ans = getAnswerState(q.id);
  const locked = isChoiceLocked(q.id, idx);
  const multi = isMultiSelect(q);

  if (locked) {
    // Locked choices cannot be toggled; just focus the inspector
    state.focusedChoiceIdx = idx;
  } else if (multi) {
    // Multi-select: toggle selection on/off
    if (ans.selected.has(idx)) {
      ans.selected.delete(idx);
    } else {
      ans.selected.add(idx);
    }
    state.focusedChoiceIdx = idx;
  } else {
    // Single-select: clear non-locked selections, then toggle
    const keepSelections = new Set();
    ans.selected.forEach(i => {
      if (isChoiceLocked(q.id, i)) keepSelections.add(i);
    });
    const wasSelected = ans.selected.has(idx);
    ans.selected = keepSelections;
    if (!wasSelected) {
      ans.selected.add(idx);
    }
    state.focusedChoiceIdx = idx;
  }

  renderQuestionContent();
  renderInspector();
}
