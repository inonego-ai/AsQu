---
name: deep-dive
description: >
  Infinite AsQu questioning loop — keeps generating follow-up questions until the user stops.
  Triggers: "deep dive", "infinite ask", "keep asking", or /deep-dive.
---

Enter plan mode, then run an infinite AsQu questioning loop.

## Goal

- **goal = infinite**: keep asking until user explicitly stops.

## Flow

1. Enter plan mode via `EnterPlanMode`.
2. Resolve topic: check arguments → conversation context → if still unclear, ask via `asqu ask`.
3. Loop until user explicitly stops:
   - Generate follow-up questions based on all answers so far.
   - Submit via `asqu ask` (batch as needed), then `asqu wait` for the batch.
   - Analyze answers and generate next round of follow-up questions.
   - Continue until user says to stop.
4. Write a plan summarizing collected Q&A and insights → `ExitPlanMode`.

## Notes

- Use `--category` to organize question topics across rounds.
- Use `--priority` to surface the most critical questions first.
- `asqu wait` with no IDs waits for the full session — no need to track individual IDs.
