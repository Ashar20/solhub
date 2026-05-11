import { z } from "zod";
import { ApiClient } from "../api.js";
import { triggerSchema, stepSchema } from "../types.js";

const createWorkflowInput = z.object({
  name: z.string(),
  trigger: triggerSchema,
  steps: z.array(stepSchema),
  fee_per_execution_usdc: z.number().optional(),
  is_public: z.boolean().optional(),
});

export const tool = {
  name: "sk.create_workflow",
  description: "Create a new SolanaKeeper automation workflow on Solana",
  inputSchema: {
    type: "object",
    required: ["name", "trigger", "steps"],
    properties: {
      name: { type: "string" },
      trigger: {
        type: "object",
        required: ["type"],
        properties: {
          type: { type: "string", enum: ["cron", "account_watch", "webhook"] },
          schedule: { type: "string" },
          account: { type: "string" },
          condition: { type: "object" },
          secret: { type: "string" },
        },
      },
      steps: {
        type: "array",
        items: {
          type: "object",
          required: ["plugin", "action", "params"],
          properties: {
            id: { type: "string" },
            plugin: { type: "string" },
            action: { type: "string" },
            params: { type: "object" },
            condition: { type: "string" },
            on_error: {
              oneOf: [
                { type: "string", enum: ["Abort", "Skip"] },
                {
                  type: "object",
                  properties: {
                    Retry: {
                      type: "object",
                      properties: { max_attempts: { type: "number" } },
                    },
                  },
                },
              ],
            },
          },
        },
      },
      fee_per_execution_usdc: { type: "number" },
      is_public: { type: "boolean" },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = createWorkflowInput.parse(args);
    return api.post("/v1/workflows", parsed);
  },
};
