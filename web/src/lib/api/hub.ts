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
 * Call (trigger) a public hub workflow.
 * POST /v1/hub/:id/call — requires auth (api/src/app.rs:46).
 * Returns { run_id, status } (api/src/routes/hub.rs:114-117).
 *
 * NOTE: GET /v1/hub/:id does not exist in the current backend — only
 * the list and call+publish endpoints are registered. Phase E will add
 * a detail endpoint once the backend grows it.
 */
export const callHubWorkflow = (id: string) =>
  apiRequest(
    `/v1/hub/${id}/call`,
    z.object({ run_id: z.string().uuid(), status: z.string() }),
    { method: "POST" },
  );
