import { z } from "zod";
import { apiRequest } from "./client";
import { WorkflowSchema } from "./schemas";

/**
 * Request body for POST /v1/hub/publish.
 * Verified from api/src/types.rs::PublishHubRequest:
 *   { workflow_id, fee_per_execution_usdc, description?, tags? }
 */
export interface PublishHubBody {
  workflow_id: string;
  fee_per_execution_usdc: number;
  description?: string;
  tags?: string[];
}

/**
 * List all public workflows from the hub.
 * GET /v1/hub — PUBLIC, no auth required (api/src/app.rs:57).
 * Returns an array of Workflow objects (api/src/routes/hub.rs:44).
 */
export const listHub = () =>
  apiRequest("/v1/hub", z.array(WorkflowSchema), { anonymous: true });

/**
 * Publish a workflow to the hub.
 * POST /v1/hub/publish — requires auth.
 * Returns the updated Workflow object (api/src/routes/hub.rs:85).
 */
export const publishToHub = (body: PublishHubBody) =>
  apiRequest("/v1/hub/publish", WorkflowSchema, {
    method: "POST",
    body,
  });

/**
 * Public payment requirements for a hub workflow.
 * GET /v1/hub/:id/payment_info — PUBLIC, no auth required.
 *
 * NOTE: Despite the name, fee_per_exec_usdc is reused as lamports for the MVP.
 * The amount_lamports field in the response is the actual SOL lamport amount.
 */
export const PaymentRequirementsSchema = z.object({
  network: z.string(),
  asset: z.string(),
  amount_lamports: z.coerce.number().int().nonnegative(),
  recipient: z.string(),
  memo: z.string(),
});
export type PaymentRequirements = z.infer<typeof PaymentRequirementsSchema>;

export const paymentInfo = (id: string) =>
  apiRequest(`/v1/hub/${id}/payment_info`, PaymentRequirementsSchema, { anonymous: true });

const CallResponse = z.object({
  run_id: z.string().uuid(),
  status: z.string().optional(),
});

export interface CallHubOpts {
  params?: Record<string, unknown>;
  /** Base58 Solana tx signature that paid the fee, if required. */
  paymentSignature?: string;
}

/**
 * Call (trigger) a public hub workflow.
 * POST /v1/hub/:id/call — requires auth (api/src/app.rs:46).
 * Returns { run_id, status } (api/src/routes/hub.rs:114-117).
 *
 * Supports x402 payment retry: if the workflow requires payment, callers should
 * first obtain a tx signature (via buildPaymentTx + sendTransaction) then pass it
 * as paymentSignature. The header format is: X-PAYMENT: solana:devnet:tx:<sig>
 */
export async function callHubWorkflow(id: string, opts: CallHubOpts = {}) {
  const headers: Record<string, string> = {};
  if (opts.paymentSignature) {
    headers["X-PAYMENT"] = `solana:devnet:tx:${opts.paymentSignature}`;
  }
  return apiRequest(`/v1/hub/${id}/call`, CallResponse, {
    method: "POST",
    body: opts.params ?? {},
    extraHeaders: headers,
  });
}
