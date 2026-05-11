import { z } from "zod";
import { apiRequest } from "./client";
import {
  WorkflowSchema,
  CreateWorkflowResponseSchema,
  TriggerWorkflowResponseSchema,
} from "./schemas";

/**
 * Query params for GET /v1/workflows.
 * Backend only accepts `active_only` (api/src/types.rs::ListWorkflowsQuery).
 * The plan mentioned `status` and `trigger_type`, but the actual backend
 * only has `active_only: Option<bool>` — verified in types.rs:86-89.
 */
export interface ListWorkflowsParams {
  active_only?: boolean;
  [key: string]: string | number | boolean | undefined | null;
}

/**
 * Request body for POST /v1/workflows.
 * Backend uses a nested `trigger` object (Value) — NOT the flat `trigger_type`
 * + `trigger_config` shape used in the DB model. The route reads
 * `body.trigger.get("type")` to extract the trigger type (workflows.rs:27-28).
 * The `steps` field is an array of free-form JSON objects.
 */
export interface CreateWorkflowBody {
  name: string;
  trigger: {
    type: "cron" | "webhook" | "manual" | "price_alert" | "on_chain";
    [key: string]: unknown;
  };
  steps: Record<string, unknown>[];
  fee_per_execution_usdc?: number;
  is_public?: boolean;
}

/**
 * Request body for PATCH /v1/workflows/:id.
 * All fields optional (api/src/types.rs::UpdateWorkflowRequest).
 */
export interface UpdateWorkflowBody {
  trigger?: Record<string, unknown>;
  steps?: Record<string, unknown>[];
  is_active?: boolean;
}

export const listWorkflows = (params: ListWorkflowsParams = {}) =>
  apiRequest("/v1/workflows", z.array(WorkflowSchema), { query: params });

export const getWorkflow = (id: string) =>
  apiRequest(`/v1/workflows/${id}`, WorkflowSchema);

export const createWorkflow = (body: CreateWorkflowBody) =>
  apiRequest("/v1/workflows", CreateWorkflowResponseSchema, {
    method: "POST",
    body,
  });

export const updateWorkflow = (id: string, body: UpdateWorkflowBody) =>
  apiRequest(`/v1/workflows/${id}`, WorkflowSchema, {
    method: "PATCH",
    body,
  });

export const deleteWorkflow = (id: string) =>
  apiRequest(`/v1/workflows/${id}`, z.unknown(), { method: "DELETE" });

/**
 * Trigger a manual run for a workflow.
 * The `param_overrides` field is reserved for future engine use
 * (api/src/routes/workflows.rs:147) but can be passed safely.
 */
export const triggerWorkflow = (
  id: string,
  overrides?: Record<string, unknown>,
) =>
  apiRequest(`/v1/workflows/${id}/trigger`, TriggerWorkflowResponseSchema, {
    method: "POST",
    body: overrides !== undefined ? { param_overrides: overrides } : undefined,
  });
