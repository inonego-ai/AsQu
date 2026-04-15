<p align="right">
  <a href="README.ko.md">🇰🇷 한국어</a>
</p>

<h1 align="center">AsQu</h1>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/rust-2024_edition-orange.svg?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/tauri-2-24C8D8.svg?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/claude_code-plugin-8A2BE2.svg" alt="Claude Code Plugin">
</p>

<p align="center">
  <b>Async Ask Question Queue for AI coding agents</b><br>
  Questions queue up in a desktop app — answer at your own pace, no more idle waiting.
</p>

<p align="center">
  <img src="image.png" alt="AsQu Screenshot">
</p>

## What is AsQu?

AsQu is an async question queue for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Instead of the agent blocking on one question at a time, questions accumulate in a persistent desktop UI and you answer them whenever you're ready.

- **Single binary** — no args starts the GUI, subcommands act as a CLI client
- **Named Pipe IPC** — GUI auto-starts in the background on first CLI call
- **Multi-session** — each Claude Code session gets its own panel in the UI
- **Auto-cleanup** — sessions disappear once all their questions are answered

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) toolchain (edition 2024)
- Platform dependencies for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

### Install

```bash
# 1. Install binary
cargo install --git https://github.com/inonego-ai/AsQu.git --bin asqu

# 2. Register marketplace
claude plugin marketplace add inonego-ai/AsQu

# 3. Install plugin
claude plugin install asqu
```

## CLI Commands

| Command | Description |
|---|---|
| `asqu ask '<json array>'` | Submit one or more questions |
| `asqu wait [ids...] [options]` | Block until answered. No ids = entire session |
| `asqu get [ids...]` | Non-blocking snapshot. No ids = entire session |
| `asqu dismiss [ids...] [--reason <r>]` | Cancel questions. No ids = all pending in session |
| `asqu open` | Show the desktop window |
| `asqu shutdown` | Gracefully shut down the GUI process |

### ask

Always pass a JSON array (even for a single question).

```bash
asqu ask '[
  {"text":"Q1?","choices":["A","B"],"category":"Deploy","priority":"critical"},
  {"text":"Q2?","choices":[{"label":"X","description":"Detail for X"},{"label":"Y"}],"instant":true},
  {"text":"Q3?","header":"Note","context":"Background info","multiSelect":true}
]'
```

Fields: `text` (required), `header`, `choices`, `allowOther`, `multiSelect`, `instant`, `context`, `category`, `priority`.  
`choices` accepts string arrays `["A","B"]` or object arrays `[{"label":"A","description":"..."}]`.

```jsonc
{ "result": "ask_ok", "ids": ["3", "4", "5"], "pending": 3 }
```

### wait

```bash
asqu wait                  # block until all pending questions are answered
asqu wait 3 4              # block until specific questions are answered
asqu wait 3 --timeout 60   # timeout after 60 seconds
asqu wait --any            # unblock on first answer
```

```jsonc
{
  "result": "answers_ok",
  "answered": [{ "id": "3", "answer": { "selections": { "0": {} }, "text": "..." } }],
  "denied":   [{ "id": "4", "reason": "dismissed by user" }],
  "pending":  ["5"],
  "timedOut": true,   // present only when timeout elapsed
  "shutdown": true    // present only when app was quit while waiting
}
```

`selections` keys are choice indices (`"0"`, `"1"`, ...). Value may contain `confidence` (0–100) and `note`.

### get

```bash
asqu get        # snapshot of all questions in this session
asqu get 3 4    # snapshot for specific IDs
```

Same response shape as `wait`. Use `pending` IDs to recover context after compaction.

### dismiss

```bash
asqu dismiss           # cancel all pending questions in this session
asqu dismiss 3 4       # cancel specific questions
asqu dismiss 3 --reason "no longer needed"
```

```jsonc
{ "result": "dismiss_ok", "dismissed": ["3", "4"] }
```

## How It Works

```
Claude Code  ──asqu ask──▶  Named Pipe  ──▶  GUI (persistent)
             ◀─ ids ───────                        │
             ──asqu wait──▶                   user answers
             ◀─ answers ───────────────────────────┘
```

1. First CLI call auto-starts the GUI in the background if it isn't running.
2. Session ID is read from `CLAUDE_SESSION_ID` env var (set automatically by Claude Code).
3. `asqu wait` blocks until the user answers in the desktop UI, then returns JSON.
4. Once all questions in a session are answered or dismissed, the session is removed automatically.

## License

[MIT](LICENSE)
