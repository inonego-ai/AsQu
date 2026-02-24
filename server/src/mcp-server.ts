import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { QuestionStore } from "./question-store.js";
import type { PipeClient } from "./pipe-client.js";
import { logger } from "./logger.js";
import { SERVER_INSTRUCTIONS, TOOL_DESCRIPTIONS } from "./instructions.js";

// Strip undefined, null, empty arrays, false from response JSON
function compact(obj: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (value === undefined || value === null) continue;
    if (Array.isArray(value) && value.length === 0) continue;
    if (value === false) continue;
    if (value === "") continue;
    result[key] = value;
  }
  return result;
}

// Wrap tool result: append any undelivered instant answers to every response
function withInstantAnswers(
  store: QuestionStore,
  result: Record<string, unknown>,
  excludeIds?: Set<string>
): { content: [{ type: "text"; text: string }] } {
  const instantAnswers = store.collectInstantAnswers(excludeIds);
  if (instantAnswers.length > 0) {
    result.instant_answers = instantAnswers;
  }
  return {
    content: [{ type: "text" as const, text: JSON.stringify(result) }],
  };
}

export function createMcpServer(
  store: QuestionStore,
  pipeClient: PipeClient,
  sessionId: string,
  sessionName: string,
  sessionCwd: string
): McpServer {
  const server = new McpServer(
    {
      name: "AsQu",
      version: "0.0.1",
    },
    {
      instructions: SERVER_INSTRUCTIONS,
    }
  );

  // === Tool 1: ask ===
  server.registerTool(
    "ask",
    {
      title: "Ask Questions",
      description: TOOL_DESCRIPTIONS.ask,
      inputSchema: {
        questions: z
          .preprocess(
            (val) => {
              // Coerce JSON string → array
              if (typeof val === "string") {
                try { val = JSON.parse(val); } catch {}
              }
              // Remap common field name mistakes per item
              if (Array.isArray(val)) {
                return val.map((item: unknown) => {
                  if (item && typeof item === "object") {
                    const q = item as Record<string, unknown>;
                    if (!q.text && (q.question || q.content)) {
                      q.text = q.question ?? q.content;
                    }
                  }
                  return item;
                });
              }
              return val;
            },
            z.array(z.object({
              text: z.string().describe("Question text to display"),
              header: z
                .preprocess(
                  (val) => typeof val === "string" && val.length > 12 ? val.slice(0, 12) : val,
                  z.string().max(12).optional()
                )
                .describe("Short label tag (max 12 chars) shown in tab"),
              choices: z
                .array(
                  z.object({
                    label: z.string().describe("Choice label"),
                    description: z
                      .string()
                      .optional()
                      .describe("Description shown below label"),
                    markdown: z
                      .string()
                      .optional()
                      .describe("Preview content for inspector panel"),
                    multiSelect: z
                      .boolean()
                      .optional()
                      .describe("Allow this choice to be selected alongside others"),
                  })
                )
                .optional()
                .describe("Choice list. Omit for freeform text input"),
              allowOther: z
                .boolean()
                .default(true)
                .describe("Show 'Other...' free-text option"),
              context: z
                .string()
                .optional()
                .describe("Additional context shown as info block"),
              instant: z
                .boolean()
                .default(false)
                .describe("Instant question — answering immediately unblocks wait_for_answers"),
              priority: z
                .enum(["critical", "high", "normal", "low"])
                .default("normal")
                .describe("Question priority — visual indicator shown to user"),
            })
          )
          .min(1)
          .describe("Array of questions to submit")
          ),
      },
      annotations: {
        readOnlyHint: false,
        destructiveHint: false,
        idempotentHint: false,
        openWorldHint: false,
      },
    },
    async ({ questions }) => {
      const added = questions.map((q) => store.addQuestion(sessionId, q));

      // Send to Tauri via pipe
      if (pipeClient.connected) {
        if (added.length === 1) {
          pipeClient.sendQuestion(added[0], sessionName, sessionCwd);
        } else {
          pipeClient.sendQuestionsBatch(added, sessionName, sessionCwd);
        }
      } else {
        logger.warn("Pipe not connected, questions stored locally only");
      }

      const result = compact({
        ids: added.map((q) => q.id),
        pending: store.getPendingCount(),
      });

      return withInstantAnswers(store, result);
    }
  );

  // === Tool 2: get_answers ===
  server.registerTool(
    "get_answers",
    {
      title: "Get Answers",
      description: TOOL_DESCRIPTIONS.get_answers,
      inputSchema: {
        ids: z
          .preprocess(
            (val) => typeof val === "string" ? (() => { try { return JSON.parse(val); } catch { return val; } })() : val,
            z.array(z.string()).min(1)
          )
          .describe("Question IDs to check"),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ ids }) => {
      const result = compact({ ...store.getAnswers(ids) });
      return withInstantAnswers(store, result);
    }
  );

  // === Tool 3: wait_for_answers ===
  server.registerTool(
    "wait_for_answers",
    {
      title: "Wait for Answers",
      description: TOOL_DESCRIPTIONS.wait_for_answers,
      inputSchema: {
        ids: z
          .preprocess(
            (val) => typeof val === "string" ? (() => { try { return JSON.parse(val); } catch { return val; } })() : val,
            z.array(z.string()).min(1)
          )
          .describe("Question IDs to wait for"),
        require_all: z
          .boolean()
          .default(true)
          .describe("Wait for all questions (true) or any (false)"),
        timeout_seconds: z
          .number()
          .int()
          .min(1)
          .max(3600)
          .optional()
          .describe("Timeout in seconds (default: no timeout)"),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ ids, require_all, timeout_seconds }) => {
      const result = await store.waitForAnswers(
        ids,
        require_all,
        timeout_seconds
      );
      // Mark instant answers from wait result as delivered (avoid double-report)
      const answeredIds = new Set(result.answered.map(a => a.id));
      store.markInstantDelivered(
        result.answered.filter(a => store.getQuestion(a.id)?.instant).map(a => a.id)
      );
      return withInstantAnswers(store, compact({ ...result }), answeredIds);
    }
  );

  // === Tool 4: list_questions ===
  server.registerTool(
    "list_questions",
    {
      title: "List Questions",
      description: TOOL_DESCRIPTIONS.list_questions,
      inputSchema: {
        status: z
          .enum(["pending", "answered", "expired", "dismissed", "denied"])
          .optional()
          .describe("Filter by status (omit for all)"),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ status }) => {
      const questions = store.getQuestionsByStatus(status);
      const result: Record<string, unknown> = {
        questions: questions.map((q) => compact({
          id: q.id,
          text: q.text,
          header: q.header,
          priority: q.priority,
          status: q.status,
          created_at: q.created_at,
          answered_at: q.answered_at,
        })),
        total: questions.length,
      };
      return withInstantAnswers(store, result);
    }
  );

  // === Tool 5: dismiss_questions ===
  server.registerTool(
    "dismiss_questions",
    {
      title: "Dismiss Questions",
      description: TOOL_DESCRIPTIONS.dismiss_questions,
      inputSchema: {
        ids: z
          .preprocess(
            (val) => typeof val === "string" ? (() => { try { return JSON.parse(val); } catch { return val; } })() : val,
            z.array(z.string()).min(1)
          )
          .describe("Question IDs to dismiss"),
        reason: z
          .string()
          .optional()
          .describe("Reason for dismissal"),
      },
      annotations: {
        readOnlyHint: false,
        destructiveHint: true,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async ({ ids, reason }) => {
      const dismissed = store.dismissQuestions(ids, reason);

      // Notify Tauri
      if (pipeClient.connected && dismissed.length > 0) {
        pipeClient.sendDismiss(dismissed, reason);
      }

      return withInstantAnswers(store, compact({
        dismissed,
        notFound: ids.filter((id: string) => !dismissed.includes(id)),
      }));
    }
  );

  return server;
}
