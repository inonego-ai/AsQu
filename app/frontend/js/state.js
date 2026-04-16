// ============================================================
// Global Application State & Shared Helpers
// ============================================================

// ------------------------------------------------------------
// Reactive state object shared across all modules
// ------------------------------------------------------------
export const state = {
  questions: new Map(),
  activeQuestionId: null,
  focusedChoiceIdx: null,
  // Per-question answer state:
  // questionId -> { selected: Set<number>, details: Map<number, {confidenceOn, confidence, note}>, text }
  answers: new Map(),

  // Session state
  sessions: new Map(),       // sessionId -> { id, displayName, createdAt, questionIds }
  sessionOrder: [],          // session IDs in insertion order
  activeSessionId: null,     // currently selected session (null = show all)

  // Loading animation state (sessionId -> dot step 0/1/2)
  loadingSessions: new Map(),
};

// ------------------------------------------------------------
// Dot label for loading animation steps
// ------------------------------------------------------------
export function dotLabel(step) {
  return ['.', '..', '...'][step % 3];
}

// ============================================================
// Constants
// ============================================================

// Numeric sort weights for priority levels (lower = more urgent)
export const PRIORITY_ORDER = {
  critical: 0,
  high: 1,
  normal: 2,
  low: 3,
};

// CSS variable references for each priority level
export const PRIORITY_COLORS = {
  critical: 'var(--critical)',
  high: 'var(--high)',
  normal: 'var(--normal)',
  low: 'var(--low)',
};

// ============================================================
// Utility Functions
// ============================================================

// ------------------------------------------------------------
// Escape HTML entities to prevent XSS in dynamic content
// ------------------------------------------------------------
export function esc(s) {
  if (!s) return '';
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// ---------------------------------------------------------------
// Convert a timestamp to a human-readable relative time string
// ---------------------------------------------------------------
export function timeAgo(ts) {
  const diff = Date.now() - ts;
  if (diff < 60000) return 'just now';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  return `${Math.floor(diff / 86400000)}d ago`;
}

// ============================================================
// Query Helpers
// ============================================================

// ------------------------------------------------------------
// Get pending questions sorted by creation time
// Filtered to activeSessionId if one is selected
// ------------------------------------------------------------
export function getPendingQuestions() {
  return Array.from(state.questions.values())
    .filter(q => {
      if (q.status !== 'pending') return false;
      if (state.activeSessionId && q.sessionId !== state.activeSessionId) return false;
      return true;
    })
    .sort((a, b) => a.createdAt - b.createdAt);
}

// -----------------------------------------------------------------------
// Group pending questions by category (preserving first-seen order)
// Returns: [{ category: string|null, questions: Question[] }, ...]
// -----------------------------------------------------------------------
export function getGroupedPendingQuestions() {
  const pending = getPendingQuestions();
  const groups = new Map();
  const order = [];

  for (const q of pending) {
    const cat = q.category || null;
    if (!groups.has(cat)) {
      groups.set(cat, []);
      order.push(cat);
    }
    groups.get(cat).push(q);
  }

  return order.map(cat => ({ category: cat, questions: groups.get(cat) }));
}

// ============================================================
// Answer State Management
// ============================================================

// ------------------------------------------------------------
// Get or create the answer state object for a given question
// ------------------------------------------------------------
export function getAnswerState(qId) {
  if (!state.answers.has(qId)) {
    state.answers.set(qId, {
      selected: new Set(),
      details: new Map(),
      text: '',
    });
  }
  return state.answers.get(qId);
}

// ------------------------------------------------------------
// Check if a question supports multi-select
// (MCP-set or user-toggled via right-click context menu)
// ------------------------------------------------------------
export function isMultiSelect(q) {
  return q.multiSelect || false;
}

// -----------------------------------------------------------------------
// Check if a choice is "locked" (has confidence or notes set)
// -----------------------------------------------------------------------
export function isChoiceLocked(qId, idx) {
  const ans = getAnswerState(qId);
  const d = ans.details.get(idx);
  if (!d) return false;
  return (d.confidenceOn && d.confidence > 0) || (d.note && d.note.trim());
}
