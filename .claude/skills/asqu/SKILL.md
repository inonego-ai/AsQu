---
name: asqu
description: >
  Async Ask Question Queue — use instead of AskUserQuestion for all user input/decisions.
  NOT for: plain-text question lists or brainstorming.
---

Re-read these instructions and load tools before first use.

## Tools

`ask` `get_answers` `wait_for_answers` `list_questions` `dismiss_questions` `open_ui`

## Rules

> **MUST** = mandatory. **SHOULD** = strongly recommended. **CAN** = optional. **NEVER** = forbidden.

### Tool Rules

#### ask

- **MUST** — batch size by current pending count:

| Pending | Batch size |
|---------|------------|
| 0-2     | 1          |
| 3-4     | 2          |
| 5-6     | 3          |
| 7+      | 4 (max)    |

- **MUST** — when user says "ask N questions": N = **total count**, still batch per above table.
- **NEVER** — dump all questions in a single call.

```
Example: 8 questions total
  ask([q1])       → pending=1
  ask([q2])       → pending=2
  ask([q3])       → pending=3
  ask([q4,q5])    → pending=5
  ask([q6,q7,q8]) → pending=8
  wait([q1..q8])
```

#### wait_for_answers

- **MUST** — call after **all** questions are submitted. Always. No exceptions.
- **NEVER** — call wait per batch. Submit all, then wait once.

```
DO    ask(q1) → ... → ask(qN) → wait([q1...qN])
NEVER ask(q1) → wait([q1]) → ask(q2) → wait([q2])
```

#### dismiss_questions

- **CAN** — cancel unneeded questions.

#### open_ui

- **CAN** — reshow the window if user may have closed it.

### Guidelines

#### Content

- **SHOULD** — one topic per question: `{ text: "Which DB?" }`
- **SHOULD** — provide `choices` whenever possible.
- **NOTE** — no maximum on choice count. Provide as many as needed to cover the realistic options.
- **NEVER** — bundle multiple topics: `{ text: "Which DB and auth and deploy?" }`

#### Priority

- **SHOULD** — ask hardest, most decision-heavy questions first to buy the user thinking time.

#### Category

- **SHOULD** — use `category` to group related questions (e.g. `"DB"`, `"Auth"`, `"Deploy"`).

#### Instant

- **MUST** — check `instant_answers` in every tool response. Process them, then `wait` again for remaining IDs.
- **SHOULD** — mark key questions or the last of each category group `instant: true` to feed follow-ups while others are pending.

#### Recovery

- **MUST** — if `wait_for_answers()` returns early with pending questions (window closed), immediately use `AskUserQuestion` to ask whether to reopen via `open_ui`.
