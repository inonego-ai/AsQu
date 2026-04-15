<p align="right">
  <a href="README.md">🇺🇸 English</a>
</p>

<h1 align="center">AsQu</h1>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/rust-2024_edition-orange.svg?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/tauri-2-24C8D8.svg?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/claude_code-plugin-8A2BE2.svg" alt="Claude Code Plugin">
</p>

<p align="center">
  <b>AI 코딩 에이전트를 위한 비동기 질문 큐</b><br>
  질문이 데스크톱 앱에 쌓이고, 원하는 타이밍에 답변 — 에이전트가 멈추며 기다리는 일은 없습니다.
</p>

<p align="center">
  <img src="image.png" alt="AsQu 스크린샷">
</p>

## AsQu란?

AsQu는 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)를 위한 비동기 질문 큐입니다. 에이전트가 질문 하나에 막혀 멈추는 대신, 질문들이 영구 실행되는 데스크톱 UI에 쌓이고 사용자가 편할 때 답변할 수 있습니다.

- **단일 바이너리** — 인자 없이 실행하면 GUI, 서브커맨드가 있으면 CLI 클라이언트
- **Named Pipe IPC** — CLI 첫 호출 시 GUI가 백그라운드에서 자동 실행
- **멀티 세션** — Claude Code 세션마다 UI에 별도 패널로 표시
- **자동 정리** — 세션의 모든 질문이 소비되면 세션이 자동으로 제거

## 설치

### 사전 요구사항

- [Rust](https://rustup.rs/) 툴체인 (2024 에디션)
- [Tauri 2](https://v2.tauri.app/start/prerequisites/) 플랫폼 의존성

### 설치

```bash
# 1. 바이너리 설치
cargo install --git https://github.com/inonego-ai/AsQu.git --bin asqu

# 2. 마켓플레이스 등록
claude plugin marketplace add inonego-ai/AsQu

# 3. 플러그인 설치
claude plugin install asqu
```

## CLI 커맨드

| 커맨드 | 설명 |
|---|---|
| `asqu ask '<json 배열>'` | 질문 제출 |
| `asqu wait [ids...] [옵션]` | 답변까지 대기. ids 없으면 세션 전체 |
| `asqu get [ids...]` | 비차단 상태 조회. ids 없으면 세션 전체 |
| `asqu dismiss [ids...] [--reason <r>]` | 질문 취소. ids 없으면 세션 전체 pending |
| `asqu open` | 데스크톱 창 표시 |
| `asqu shutdown` | GUI 프로세스 종료 |

### ask

항상 JSON 배열로 전달 (단일 질문도 배열).

```bash
asqu ask '[
  {"text":"Q1?","choices":["A","B"],"category":"Deploy","priority":"critical"},
  {"text":"Q2?","choices":[{"label":"X","description":"X에 대한 설명"},{"label":"Y"}],"instant":true},
  {"text":"Q3?","header":"메모","context":"배경 정보","multiSelect":true}
]'
```

필드: `text` (필수), `header`, `choices`, `allowOther`, `multiSelect`, `instant`, `context`, `category`, `priority`.  
`choices`는 문자열 배열 `["A","B"]` 또는 객체 배열 `[{"label":"A","description":"..."}]` 허용.

```jsonc
{ "result": "ask_ok", "ids": ["3", "4", "5"], "pending": 3 }
```

### wait

```bash
asqu wait                  # 세션 전체 질문이 답변될 때까지 대기
asqu wait 3 4              # 특정 질문 대기
asqu wait 3 --timeout 60   # 60초 타임아웃
asqu wait --any            # 첫 번째 답변 즉시 해제
```

```jsonc
{
  "result": "answers_ok",
  "answered": [{ "id": "3", "answer": { "selections": { "0": {} }, "text": "..." } }],
  "denied":   [{ "id": "4", "reason": "dismissed by user" }],
  "pending":  ["5"],
  "timedOut": true,   // 타임아웃 시에만 포함
  "shutdown": true    // 대기 중 앱 종료 시에만 포함
}
```

`selections` 키는 선택지 인덱스(`"0"`, `"1"`, ...). 값에 `confidence` (0–100), `note` 포함 가능.

### get

```bash
asqu get        # 세션 전체 질문 스냅샷
asqu get 3 4    # 특정 ID 스냅샷
```

응답 형식은 `wait`와 동일. `pending` ID로 컨텍스트 유실 후 복구 가능.

### dismiss

```bash
asqu dismiss           # 세션 전체 pending 질문 취소
asqu dismiss 3 4       # 특정 질문 취소
asqu dismiss 3 --reason "더 이상 필요 없음"
```

```jsonc
{ "result": "dismiss_ok", "dismissed": ["3", "4"] }
```

## 동작 방식

```
Claude Code  ──asqu ask──▶  Named Pipe  ──▶  GUI (영구 실행)
             ◀─ ids ───────                        │
             ──asqu wait──▶                   사용자 답변
             ◀─ answers ───────────────────────────┘
```

1. CLI 첫 호출 시 GUI가 실행 중이 아니면 백그라운드에서 자동 시작.
2. 세션 ID는 `CLAUDE_SESSION_ID` 환경 변수에서 읽음 (Claude Code가 자동으로 설정).
3. `asqu wait`는 사용자가 데스크톱 UI에서 답변할 때까지 블로킹 후 JSON 반환.
4. 세션의 모든 질문이 답변되거나 취소되면 세션이 자동으로 제거.

## 라이선스

[MIT](LICENSE)
