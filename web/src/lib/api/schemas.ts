import { z } from "zod";

// ---------------------------------------------------------------------------
// RunStatus — PascalCase, no serde attrs on engine/src/state/run.rs:14-25.
// DB stores/emits these same string literals (db/src/runs.rs:23,88).
// ---------------------------------------------------------------------------
export const RunStatusSchema = z.enum([
  "Pending",
  "Triggered",
  "Simulating",
  "Bundling",
  "Submitted",
  "Confirmed",
  "Retrying",
  "Failed",
  "Skipped",
  "WaitingApproval",
]);
export type RunStatus = z.infer<typeof RunStatusSchema>;

// ---------------------------------------------------------------------------
// StepStatus — PascalCase, no serde attrs on engine/src/state/run.rs:27-34.
// ---------------------------------------------------------------------------
export const StepStatusSchema = z.enum([
  "Pending",
  "Running",
  "Success",
  "Failed",
  "Skipped",
]);
export type StepStatus = z.infer<typeof StepStatusSchema>;

// ---------------------------------------------------------------------------
// TriggerSource — snake_case via #[serde(rename_all = "snake_case")]
// (engine/src/state/run.rs:5). Variants: Cron→"cron", AccountWatch→"account_watch",
// Webhook→"webhook", Manual→"manual", Mcp→"mcp".
// NOTE: DB stores triggered_by as a raw string, not the typed enum — the routes
// insert "manual" (api/src/routes/workflows.rs:161) or "cron"/"webhook" etc.
// ---------------------------------------------------------------------------
export const TriggerSourceSchema = z.enum([
  "cron",
  "account_watch",
  "webhook",
  "manual",
  "mcp",
]);
export type TriggerSource = z.infer<typeof TriggerSourceSchema>;

// ---------------------------------------------------------------------------
// TriggerConfig — stored as free-form JSON (api/src/routes/workflows.rs:18).
// Validated trigger types: "cron","webhook","manual","price_alert","on_chain".
// There is NO typed TriggerConfig enum in the API; the shape is whatever the
// client sends. We model the known variants loosely.
// ---------------------------------------------------------------------------
export const TriggerConfigSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("cron"), schedule: z.string() }),
  z.object({
    type: z.literal("webhook"),
    secret: z.string().optional(),
  }),
  z.object({ type: z.literal("manual") }),
  z.object({
    type: z.literal("price_alert"),
    token: z.string(),
    threshold_usd: z.number(),
    direction: z.enum(["above", "below"]),
  }),
  z.object({
    type: z.literal("on_chain"),
    account: z.string(),
    condition: z.record(z.unknown()).optional(),
  }),
]);
export type TriggerConfig = z.infer<typeof TriggerConfigSchema>;

// ---------------------------------------------------------------------------
// WorkflowStep — stored as JSON in the `steps` column (Value).
// No typed struct in API; free-form params map.
// ---------------------------------------------------------------------------
export const WorkflowStepSchema = z.object({
  id: z.string(),
  plugin: z.string(),
  action: z.string(),
  params: z.record(z.unknown()),
  condition: z.string().nullable().optional(),
  on_error: z
    .discriminatedUnion("kind", [
      z.object({ kind: z.literal("abort") }),
      z.object({ kind: z.literal("skip") }),
      z.object({
        kind: z.literal("retry"),
        max_attempts: z.number().int().positive(),
      }),
    ])
    .optional(),
});
export type WorkflowStep = z.infer<typeof WorkflowStepSchema>;

// ---------------------------------------------------------------------------
// Workflow — db::models::Workflow (db/src/models.rs:44-58).
// Fields: id, org_id, name, trigger_type (string), trigger_config (Value),
// steps (Value), is_active, is_public, onchain_pda, fee_per_exec_usdc (Option<i64>),
// execution_count (i64), created_at (DateTime<Utc>), updated_at (DateTime<Utc>).
// ---------------------------------------------------------------------------
export const WorkflowSchema = z.object({
  id: z.string().uuid(),
  org_id: z.string().uuid(),
  name: z.string(),
  trigger_type: z.string(),
  trigger_config: z.record(z.unknown()),
  steps: z.union([z.array(z.unknown()), z.record(z.unknown())]),
  is_active: z.boolean(),
  is_public: z.boolean(),
  onchain_pda: z.string().nullable().optional(),
  fee_per_exec_usdc: z.number().int().nullable().optional(),
  execution_count: z.number().int().default(0),
  created_at: z.string(),
  updated_at: z.string().optional(),
});
export type Workflow = z.infer<typeof WorkflowSchema>;

// ---------------------------------------------------------------------------
// StepLog — engine/src/state/run.rs:97-105. Serialized into steps_log JSON array.
// Fields: step_id, status (StepStatus — PascalCase), input, output, duration_ms, error.
// ---------------------------------------------------------------------------
export const StepLogSchema = z.object({
  step_id: z.string(),
  status: StepStatusSchema,
  input: z.unknown(),
  output: z.unknown(),
  duration_ms: z.number().int().nonnegative(),
  error: z.string().nullable().optional(),
});
export type StepLog = z.infer<typeof StepLogSchema>;

// ---------------------------------------------------------------------------
// WorkflowRun — db::models::WorkflowRun (db/src/models.rs:61-76).
// Fields: run_id, workflow_id, org_id, status (string/PascalCase), triggered_by,
// steps_log (Value — JSON array), slot, signature, fee_lamports, jito_tip_lamports,
// error_message (NOT "error"), started_at, completed_at.
// ---------------------------------------------------------------------------
export const WorkflowRunSchema = z.object({
  run_id: z.string().uuid(),
  workflow_id: z.string().uuid(),
  org_id: z.string().uuid(),
  status: RunStatusSchema,
  triggered_by: z.string(),
  steps_log: z.array(StepLogSchema).default([]),
  slot: z.number().int().nullable().optional(),
  signature: z.string().nullable().optional(),
  fee_lamports: z.number().int().nullable().optional(),
  jito_tip_lamports: z.number().int().nullable().optional(),
  error_message: z.string().nullable().optional(),
  started_at: z.string(),
  completed_at: z.string().nullable().optional(),
});
export type WorkflowRun = z.infer<typeof WorkflowRunSchema>;

// ---------------------------------------------------------------------------
// RunLogEvent — SSE events from api/src/routes/runs.rs:stream_run_logs.
// Only two event types are actually emitted: "step_log" and "run_complete".
// "step_log" data = a StepLog entry; "run_complete" data = the full WorkflowRun.
// ---------------------------------------------------------------------------
export const RunLogEventSchema = z.object({
  event: z.enum(["step_log", "run_complete"]),
  data: z.unknown(),
});
export type RunLogEvent = z.infer<typeof RunLogEventSchema>;

// ---------------------------------------------------------------------------
// Organization — db::models::Organization (db/src/models.rs:25-31).
// Fields: id, name, wallet_address (Option<String>), credits_usdc (i64),
// created_at (DateTime<Utc> → serializes as ISO string via Serialize).
// ---------------------------------------------------------------------------
export const OrgSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  wallet_address: z.string().nullable(),
  credits_usdc: z.number().int(),
  created_at: z.string(),
});
export type Org = z.infer<typeof OrgSchema>;

// ---------------------------------------------------------------------------
// ApiKey — db::models::ApiKey (db/src/models.rs:33-41).
// The list_keys endpoint (api/src/routes/orgs.rs:53-65) returns a manual
// json! shape that includes: id, org_id, name, last_used_at, created_at, revoked_at.
// key_hash is deliberately omitted from the response.
// ---------------------------------------------------------------------------
export const ApiKeySchema = z.object({
  id: z.string().uuid(),
  org_id: z.string().uuid(),
  name: z.string().nullable(),
  last_used_at: z.string().nullable(),
  created_at: z.string(),
  revoked_at: z.string().nullable(),
});
export type ApiKey = z.infer<typeof ApiKeySchema>;

// ---------------------------------------------------------------------------
// CreateApiKeyResponse — api/src/types.rs:45-50.
// Fields: id, key (raw plaintext secret — only returned once), name.
// ---------------------------------------------------------------------------
export const CreateApiKeyResponseSchema = z.object({
  id: z.string().uuid(),
  key: z.string(),
  name: z.string().nullable(),
});
export type CreateApiKeyResponse = z.infer<typeof CreateApiKeyResponseSchema>;

// ---------------------------------------------------------------------------
// CreateWorkflowResponse — api/src/types.rs:14-20.
// Fields: workflow_id, status, next_run, onchain_pda.
// ---------------------------------------------------------------------------
export const CreateWorkflowResponseSchema = z.object({
  workflow_id: z.string().uuid(),
  status: z.string(),
  next_run: z.string().nullable().optional(),
  onchain_pda: z.string().nullable().optional(),
});
export type CreateWorkflowResponse = z.infer<typeof CreateWorkflowResponseSchema>;

// ---------------------------------------------------------------------------
// TriggerWorkflowResponse — api/src/types.rs:34-38.
// Fields: run_id, status.
// ---------------------------------------------------------------------------
export const TriggerWorkflowResponseSchema = z.object({
  run_id: z.string().uuid(),
  status: z.string(),
});
export type TriggerWorkflowResponse = z.infer<typeof TriggerWorkflowResponseSchema>;

// ---------------------------------------------------------------------------
// AnalyticsResponse — api/src/types.rs:63-68.
// Fields: total_executions, successful, failed, total_fee_lamports (all i64).
// NOTE: This differs from the plan's schema — no "range", "success_rate",
// "credits_remaining", or "fee_spend_lamports". Exact fields per types.rs.
// ---------------------------------------------------------------------------
export const AnalyticsSchema = z.object({
  total_executions: z.number().int().nonnegative(),
  successful: z.number().int().nonnegative(),
  failed: z.number().int().nonnegative(),
  total_fee_lamports: z.number().int().nonnegative(),
});
export type Analytics = z.infer<typeof AnalyticsSchema>;

// ---------------------------------------------------------------------------
// HubWorkflow — the hub list/detail endpoints return db::Workflow structs
// (api/src/routes/hub.rs:44, hub.rs returns json!(workflows) which are Workflow).
// So HubWorkflow is the same shape as WorkflowSchema.
// ---------------------------------------------------------------------------
export const HubWorkflowSchema = WorkflowSchema;
export type HubWorkflow = Workflow;
