---
name: guide
description: >
  Async Ask Question Queue — use instead of AskUserQuestion for all user input/decisions.
  CLI-based: run asqu commands in Bash. Session auto-detected from CLAUDE_SESSION_ID env var.
  NOT for: plain-text question lists or brainstorming.
user-invocable: false
---

Re-read these instructions before first use each session.

## Session Setup

If `<asqu-session-id>` is present in your context (injected by the SessionStart hook), run this **once** immediately before the first asqu command:

```bash
export CLAUDE_SESSION_ID=<value from asqu-session-id tag>
```

This ensures all asqu commands (`ask`, `wait`, `get`, `dismiss`) are scoped to the current Claude session automatically.

## Commands

| Command | Description |
|---------|-------------|
| `asqu ask '<json>'` | Submit one or more questions (JSON object or array) |
| `asqu wait [ids...] [options]` | Block until answered. No ids = entire session |
| `asqu get [ids...]` | Non-blocking snapshot. No ids = entire session |
| `asqu dismiss [ids...] [--reason <r>]` | Cancel questions. No ids = all pending in session |
| `asqu open` | Show the UI window |
| `asqu shutdown` | Gracefully shut down the GUI process |

### ask

**Input** — always a JSON array (even for a single question). `choices` accepts string arrays or object arrays with `label`/`description`.

```bash
asqu ask '[
  {"text":"Q1?","choices":["A","B"],"category":"Deploy","priority":"critical"},
  {"text":"Q2?","choices":[{"label":"X","description":"Option X detail"},{"label":"Y"}],"instant":true},
  {"text":"Q3 free text?","header":"Note","context":"background info"}
]'
```

Fields: `text` (required), `header`, `choices`, `allowOther`, `multiSelect`, `instant`, `context`, `category`, `priority`.

**Output**

```json
{ "result": "ask_ok", "ids": ["3", "4", "5"], "pending": 3 }
```

`ids` — use these to reference the question in wait/get/dismiss.  
`pending` — current total pending count across the session.

### wait

**Input**

```bash
asqu wait                  # block until all pending questions in this session are answered
asqu wait 3 4 5            # block until questions 3, 4, 5 are answered
asqu wait 3 --timeout 60   # timeout after 60 seconds
asqu wait --any            # unblock as soon as any one question is answered
```

Options: `--any`, `--timeout <seconds>`

**Output**

```json
{
  "result": "answers_ok",
  "answered": [{ "id": "3", "answer": { "selections": { "0": {} }, "text": "..." } }],
  "denied":   [{ "id": "4", "reason": "dismissed by user" }],
  "pending":  ["5"],
  "timedOut": true,
  "shutdown": true
}
```

`selections` keys are choice indices ("0", "1", ...). Value object may contain `confidence` (0–1) and `note`.  
`denied` = dismissed by user or session removed. `timedOut` = timeout elapsed. `shutdown` = app quit while waiting (treat as cancelled, do not re-ask).

### get

**Input**

```bash
asqu get               # non-blocking snapshot of all questions in this session
asqu get 3 4           # snapshot for specific question IDs
```

**Output** — same shape as `wait`. Use `pending` IDs to recover after context loss: `asqu wait <ids...>`.

### dismiss

**Input**

```bash
asqu dismiss           # cancel all pending questions in this session
asqu dismiss 3 4       # cancel specific questions
asqu dismiss 3 --reason "no longer needed"
```

**Output**

```json
{ "result": "dismiss_ok", "dismissed": ["3", "4"] }
```

## Rules

> **MUST** = mandatory. **SHOULD** = strongly recommended. **NEVER** = forbidden.

### ask

- **MUST** — one topic per question. Never bundle multiple decisions.
- **SHOULD** — provide `choices` whenever possible.
- **SHOULD** — use `category` to group related questions (e.g., `"DB"`, `"Auth"`, `"Deploy"`).
- **SHOULD** — ask hardest, most blocking questions first to buy thinking time.
- **NEVER** — submit more than 8 questions before calling `wait`.

### wait

- **MUST** — call `asqu wait` after submitting questions. Always. No exceptions.
- **SHOULD** — `asqu wait` with no IDs waits for all pending questions in the session.
- **NEVER** — call `wait` after every single `ask`. Submit the full batch, then wait once.

```
# Correct
asqu ask '[{"text":"Q1","choices":["A","B"]}]'
asqu ask '[{"text":"Q2","choices":["X","Y"]}]'
asqu ask '[{"text":"Q3","choices":["P","Q"]}]'
asqu wait                       # waits for all 3 at once

# Wrong
asqu ask '[{"text":"Q1"}]'; asqu wait
asqu ask '[{"text":"Q2"}]'; asqu wait
```

### Batching

Use `pending` from `ask_ok` to calibrate how many to submit next:

| Currently pending | Next batch size |
|-------------------|-----------------|
| 0–2               | 1               |
| 3–4               | 2               |
| 5–6               | 3               |
| 7+                | 4 (max)         |

After `wait` resolves, check if `pending` is non-empty — submit the next batch before processing answers.

### Recovery

- **MUST** — if `wait` returns `"timedOut": true` or `"pending": [...]` is non-empty unexpectedly, use `AskUserQuestion` to check if user wants to continue, then optionally call `asqu open` to resurface the window.
- **MUST** — if `wait` returns `"shutdown": true`, stop the current task and inform the user — the app was quit while waiting, questions are gone.
- **MUST** — process `denied` answers — they mean the user dismissed that question (or the session was removed), so adapt your plan accordingly.
- **SHOULD** — if context was lost (compaction/restart), run `asqu get` with no ids to recover pending IDs, then resume with `asqu wait <recovered ids>` instead of re-asking.
