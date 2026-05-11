import { z } from "zod";
import { ApiClient } from "../api.js";

const publishToHubInput = z.object({
  workflow_id: z.string(),
  fee_per_execution_usdc: z.number(),
  description: z.string().optional(),
  tags: z.array(z.string()).optional(),
});

export const tool = {
  name: "sk.publish_to_hub",
  description: "Publish a workflow to the public SolanaKeeper Marketplace",
  inputSchema: {
    type: "object",
    required: ["workflow_id", "fee_per_execution_usdc"],
    properties: {
      workflow_id: { type: "string" },
      fee_per_execution_usdc: { type: "number", description: "Fee charged per execution in USDC" },
      description: { type: "string" },
      tags: { type: "array", items: { type: "string" } },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = publishToHubInput.parse(args);
    return api.post("/v1/hub/publish", parsed);
  },
};
