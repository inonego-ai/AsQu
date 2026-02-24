export const SERVER_INSTRUCTIONS = `AsQu is an async question queue. Use this instead of AskUserQuestion whenever you need input or answers from the user.
- "Ask me questions" → use AsQu
- "Give me a list of questions" → plain text output
- "What should I ask the team?" → plain text output

MUST re-read these instructions and load tools before first use.

ask, get_answers, wait_for_answers, list_questions, dismiss_questions

MUST = mandatory, always enforce. SHOULD = strongly recommended. CAN = optional.

ASK       | MUST    pending count 0 - 2 → ask only 1 question per ask. 
          | MUST    pending count 3+    → gradually increase. (batch size 2 → 3 → 4(max))
          | SHOULD  ask the hardest, most decision-heavy questions first — to buy the user maximum thinking time.
example)
  DO    < ask([q1]) on pending 0 > → work → ... → < ask([q3]) on pending 2 > → work → < ask([q4, q5]) on pending 3 > → ... → wait()
  NEVER < ask([q1-q20]) > → wait()

WAIT      | NEVER   call wait_for_answers() until the full set — not per batch of questions is submitted
          | MUST    call it at the end. Always. No exceptions.
example)
  DO    ask(q1) → ... → ask(qN) → wait([q1...qN])
  NEVER ask(q1) → wait([q1]) → ask(q2) → wait([q2])

CONTENT   | SHOULD  focus on question per item. { text: "Which DB?" }
          | NEVER   bundle multiple into one.   { text: "Which DB and auth method and deploy strategy?" }

INSTANT   | SHOULD  check "instant_answers" in every tool response — delivered in any ask/get/wait response automatically.
          | NEVER   call wait_for_answers() just to receive an instant answer. just keep calling ask() and check the response.

DISMISS   | CAN     cancel unneeded questions.`;

export const TOOL_DESCRIPTIONS = {
  ask: `Submit questions to the async queue. Returns IDs immediately (non-blocking).
Omit choices for freeform text input. Per-choice multiSelect to mix single/multi-select.
"Other" free-text option is always shown — do not add it as a choice.
Priority is a visual indicator only. header is a tab label (max 12 chars).

instant: true — use when (a) the answer is needed immediately, or (b) you want to ask follow-ups based on it while other questions remain pending.
Instant answers appear in "instant_answers" of ANY tool response and unblock wait_for_answers() early — call it again for remaining IDs.`,

  get_answers: `Non-blocking poll — returns current state: answered, denied, or pending.
Prefer wait_for_answers() when you need to block.`,

  wait_for_answers: `Block until answers are available or timeout expires.
require_all=true: wait for all. require_all=false: return on first answer.
Returns partial results with timed_out=true on expiry.
Denied questions are returned as normal results — handle gracefully.`,

  list_questions: `Non-blocking queue status. Filter by: pending, answered, dismissed, denied.
Returns metadata only — no full answer details.`,

  dismiss_questions: `Cancel pending questions no longer needed.`,
};
