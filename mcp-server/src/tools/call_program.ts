import { z } from "zod";
import { ApiClient } from "../api.js";

const accountMeta = z.object({
  pubkey: z.string(),
  is_signer: z.boolean(),
  is_writable: z.boolean(),
});

const callProgramInput = z.object({
  program_id: z.string(),
  instruction_data: z.string(), // base64 encoded
  accounts: z.array(accountMeta).optional(),
});

/**
 * POST /v1/execute/program
 * Request:  { program_id, instruction_data (base64), accounts: [{pubkey, is_signer, is_writable}] }
 * Response: { signature }
 */
export const tool = {
  name: "sk.call_program",
  description: "Execute any Solana program instruction via the platform wallet",
  inputSchema: {
    type: "object",
    required: ["program_id", "instruction_data"],
    properties: {
      program_id: { type: "string", description: "Base58 program ID" },
      instruction_data: { type: "string", description: "Base64-encoded instruction data" },
      accounts: {
        type: "array",
        items: {
          type: "object",
          required: ["pubkey", "is_signer", "is_writable"],
          properties: {
            pubkey: { type: "string" },
            is_signer: { type: "boolean" },
            is_writable: { type: "boolean" },
          },
        },
      },
    },
  } as const,
  handler: async (args: unknown, api: ApiClient) => {
    const parsed = callProgramInput.parse(args);
    return api.post("/v1/execute/program", parsed);
  },
};
