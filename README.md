# AsQu

![usage](usage.gif)

**Async Ask Question Queue**

MCP-based async ask question queue that replaces blocking `AskUserQuestion`-style interactions. Instead of waiting for the agent to stop and ask you one question at a time, questions queue up in a desktop app and you answer them at your own pace — no more idle waiting while the agent blocks on your response.

## Usage

Any MCP-compatible AI coding agent can use AsQu:

- **Claude Code** — replaces built-in `AskUserQuestion`
- **Any MCP client** — standard MCP tools, no vendor lock-in

## Installation

### Prerequisites

- **Node.js** >= 20
- **Rust** toolchain ([rustup](https://rustup.rs/))
- **Windows 11** (Windows Named Pipes; Unix support planned)

### Build

```bash
# 1. Build the MCP server
cd server
npm install
npm run build

# 2. Build the Tauri app
cd ../app
npm install
npm run tauri build
```

The Tauri binary will be at `app/src-tauri/target/release/asqu.exe`.

### Configure MCP

Add to your MCP settings (e.g. Claude Code `~/.claude.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "AsQu": {
      "command": "node",
      "args": ["<path-to-AsQu>/server/build/index.js"]
    }
  }
}
```

The MCP server auto-launches the Tauri app on first use.

## MCP Tools

### `ask` — Submit questions (non-blocking)

Push questions to the queue. Returns question IDs immediately.

```json
{
  "questions": [
    {
      "text": "Which database for the session store?",
      "header": "database",
      "choices": [
        { "label": "PostgreSQL", "description": "ACID compliant" },
        { "label": "Redis", "description": "Sub-ms reads", "markdown": "```\nIn-memory key-value store\n```" }
      ],
      "allowOther": true,
      "context": "Need sub-10ms reads, ~100K items",
      "priority": "critical",
      "instant": false
    }
  ]
}
```

| Field | Default | Description |
|---|---|---|
| `text` | required | Question text |
| `header` | — | Tab label (max 12 chars) |
| `choices` | — | Choice list. Omit for freeform text input |
| `choices[].label` | required | Choice label |
| `choices[].description` | — | Shown below label |
| `choices[].markdown` | — | Preview content in inspector panel |
| `choices[].multiSelect` | — | Per-choice multi-select override |
| `allowOther` | `true` | Include "Other..." free text option |
| `context` | — | Additional context shown as info block |
| `priority` | `"normal"` | `critical` / `high` / `normal` / `low` |
| `instant` | `false` | Instant question (see below) |

### `wait_for_answers` — Wait for answers (blocking)

Block until specific questions are answered.

| Field | Default | Description |
|---|---|---|
| `ids` | required | Question IDs to wait for |
| `require_all` | `true` | `true` = wait for all, `false` = return on first answer |
| `timeout_seconds` | — | Timeout in seconds (1–3600). Returns partial results with `timed_out: true` on expiry |

### `get_answers` — Check answers (non-blocking)

Poll for answers without blocking. Same response format as `wait_for_answers`.

| Field | Description |
|---|---|
| `ids` | Question IDs to check |

### `list_questions` — Query queue status

| Field | Description |
|---|---|
| `status` | Filter: `pending` / `answered` / `dismissed` / `denied` (omit for all) |

### `dismiss_questions` — Remove questions

| Field | Description |
|---|---|
| `ids` | Question IDs to dismiss |
| `reason` | Optional reason string |

## Instant Questions

Set `instant: true` on questions where the answer directly unblocks the agent's immediate next step.

**Eager delivery**: Answered instant questions are included as `instant_answers` in ANY tool response — the agent doesn't need to call `wait_for_answers` to receive them.

**Early return from wait**: When `wait_for_answers` is blocking for multiple questions and an instant question gets answered, it returns immediately — even if other questions are still pending.

```
Agent:  ask(q1), ask(q2), ask(q3_instant)
Agent:  ... keeps working ...
Agent:  ask(q4) → response includes instant_answers:[q3 result] if answered
Agent:  ... or ...
Agent:  wait_for_answers([q1, q2, q3])
User:   answers q3 (instant)
Agent:  ← returns immediately with q3 result + q1,q2 still pending
Agent:  process q3 answer
Agent:  wait_for_answers([q1, q2]) → waits for remaining
```

## Basic Usage Pattern

```
Agent:  ask(q1)
Agent:  ask(q2)
Agent:  ask(q3)
Agent:  ... keeps working ...
Agent:  ... keeps working ...
Agent:  wait_for_answers([q1, q2])  → blocks here until user answers
User:   answers q1, q2 in the app
Agent:  ... gets answers, continues ...
Agent:  dismiss_questions([q3])     → q3 no longer needed
```

## UI

Three-column layout: **Sessions** | **Question** | **Inspector**

- **Pending tab**: One question at a time with horizontal tabs for switching
- **History tab**: Card list of answered/dismissed questions
- **Inspector panel**: Markdown preview + confidence slider + notes per choice
- **System tray**: Click to toggle window, tray icon shows pending count
- **Keyboard**: `Enter` submit, `1-9` quick-select, `←/→` switch questions, `Esc` dismiss

## License

MIT