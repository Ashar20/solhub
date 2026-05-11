import { z } from "zod";
import { ApiClient } from "../api.js";

const listWorkflowsInput = z.object({
  status: z.enum(["active", "inactive", "all"]).optional(),
  limit: z.number().int().optional(),
});

export const tool = {
  name: "sk.list_workflows",
  description: "List all workflows for the authenticated org",
  inputSchema: {
    type: "object",
    properties: {
      status: { type: "string", enum: ["active", "inactive", "all"] },
      limit: { type: "integer", default: 20 },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = listWorkflowsInput.parse(args);
    const params = new URLSearchParams();
    if (parsed.status !== undefined) params.set("status", parsed.status);
    if (parsed.limit !== undefined) params.set("limit", String(parsed.limit));
    const qs = params.toString();
    const path = qs ? `/v1/workflows?${qs}` : "/v1/workflows";
    return api.get(path);
  },
};
