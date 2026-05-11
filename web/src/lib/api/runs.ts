import { z } from "zod";
import { apiRequest } from "./client";
import { WorkflowRunSchema } from "./schemas";

/**
 * Query params for GET /v1/runs.
 * Backend: ListRunsQuery { workflow_id, status, limit } — no `from`/`to` date
 * filtering in the current implementation (api/src/types.rs:80-84).
 */
export interface ListRunsParams {
  workflow_id?: string;
  status?: string;
  limit?: number;
  [key: string]: string | number | boolean | undefined | null;
}

export const listRuns = (params: ListRunsParams = {}) =>
  apiRequest("/v1/runs", z.array(WorkflowRunSchema), { query: params });

export const getRun = (run_id: string) =>
  apiRequest(`/v1/runs/${run_id}`, WorkflowRunSchema);

/**
 * SSE endpoint URL for streaming run logs.
 *
 * NOTE: The backend does NOT accept a `?token=` query parameter — the auth
 * middleware expects the Authorization header. This helper exists as the
 * single source of truth for the path. Phase B will proxy through a Next.js
 * route handler that re-attaches the Authorization header server-side.
 */
export function runStreamUrl(run_id: string): string {
  const base =
    process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";
  return new URL(`/v1/runs/${run_id}/logs`, base).toString();
}
