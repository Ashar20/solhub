import { z } from "zod";
import { ApiClient } from "../api.js";

const getRunStatusInput = z.object({
  run_id: z.string(),
});

export const tool = {
  name: "sk.get_run_status",
  description: "Get execution status and logs for a workflow run",
  inputSchema: {
    type: "object",
    required: ["run_id"],
    properties: {
      run_id: { type: "string" },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = getRunStatusInput.parse(args);
    return api.get(`/v1/runs/${parsed.run_id}`);
  },
};
