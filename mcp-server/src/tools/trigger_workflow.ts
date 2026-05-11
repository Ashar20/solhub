import { z } from "zod";
import { ApiClient } from "../api.js";

const triggerWorkflowInput = z.object({
  workflow_id: z.string(),
  param_overrides: z.record(z.unknown()).optional(),
});

export const tool = {
  name: "sk.trigger_workflow",
  description: "Manually trigger a workflow by ID, optionally overriding parameters",
  inputSchema: {
    type: "object",
    required: ["workflow_id"],
    properties: {
      workflow_id: { type: "string" },
      param_overrides: { type: "object" },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = triggerWorkflowInput.parse(args);
    const { workflow_id, param_overrides } = parsed;
    return api.post(`/v1/workflows/${workflow_id}/trigger`, param_overrides ?? {});
  },
};
