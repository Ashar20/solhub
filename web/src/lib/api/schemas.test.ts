import { describe, it, expect } from "vitest";
import {
  RunStatusSchema,
  StepStatusSchema,
  TriggerSourceSchema,
  TriggerConfigSchema,
  WorkflowSchema,
  WorkflowRunSchema,
  StepLogSchema,
  RunLogEventSchema,
  OrgSchema,
  ApiKeySchema,
  AnalyticsSchema,
  CreateWorkflowResponseSchema,
  CreateApiKeyResponseSchema,
} from "./schemas";

// ---------------------------------------------------------------------------
// RunStatusSchema
// ---------------------------------------------------------------------------
describe("RunStatusSchema", () => {
  it("accepts all PascalCase variants the backend emits", () => {
    const variants = [
      "Pending",
      "Triggered",
      "Simulating",
      "Bundling",
      "Submitted",
      "Confirmed",
      "Retrying",
      "Failed",
      "Skipped",
    ] as const;
    for (const v of variants) {
      expect(() => RunStatusSchema.parse(v)).not.toThrow();
    }
  });

  it("rejects lowercase variants", () => {
    expect(() => RunStatusSchema.parse("confirmed")).toThrow();
    expect(() => RunStatusSchema.parse("failed")).toThrow();
  });

  it("rejects unknown variants", () => {
    expect(() => RunStatusSchema.parse("bogus")).toThrow();
    expect(() => RunStatusSchema.parse("")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// StepStatusSchema
// ---------------------------------------------------------------------------
describe("StepStatusSchema", () => {
  it("accepts all PascalCase variants", () => {
    for (const v of ["Pending", "Running", "Success", "Failed", "Skipped"]) {
      expect(() => StepStatusSchema.parse(v)).not.toThrow();
    }
  });

  it("rejects lowercase step status", () => {
    expect(() => StepStatusSchema.parse("success")).toThrow();
    expect(() => StepStatusSchema.parse("running")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// TriggerSourceSchema
// ---------------------------------------------------------------------------
describe("TriggerSourceSchema", () => {
  it("accepts snake_case trigger sources per serde(rename_all=snake_case)", () => {
    for (const v of ["cron", "account_watch", "webhook", "manual", "mcp"]) {
      expect(() => TriggerSourceSchema.parse(v)).not.toThrow();
    }
  });

  it("rejects PascalCase trigger sources", () => {
    expect(() => TriggerSourceSchema.parse("Cron")).toThrow();
    expect(() => TriggerSourceSchema.parse("Manual")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// TriggerConfigSchema
// ---------------------------------------------------------------------------
describe("TriggerConfigSchema", () => {
  it("parses a cron trigger", () => {
    const v = TriggerConfigSchema.parse({ type: "cron", schedule: "*/5 * * * *" });
    expect(v.type).toBe("cron");
    if (v.type === "cron") {
      expect(v.schedule).toBe("*/5 * * * *");
    }
  });

  it("parses a webhook trigger without secret", () => {
    const v = TriggerConfigSchema.parse({ type: "webhook" });
    expect(v.type).toBe("webhook");
  });

  it("parses a webhook trigger with secret", () => {
    const v = TriggerConfigSchema.parse({ type: "webhook", secret: "abc123" });
    expect(v.type).toBe("webhook");
  });

  it("parses a manual trigger", () => {
    const v = TriggerConfigSchema.parse({ type: "manual" });
    expect(v.type).toBe("manual");
  });

  it("parses a price_alert trigger", () => {
    const v = TriggerConfigSchema.parse({
      type: "price_alert",
      token: "SOL",
      threshold_usd: 200,
      direction: "above",
    });
    expect(v.type).toBe("price_alert");
  });

  it("parses an on_chain trigger", () => {
    const v = TriggerConfigSchema.parse({
      type: "on_chain",
      account: "11111111111111111111111111111111",
    });
    expect(v.type).toBe("on_chain");
  });

  it("rejects unknown trigger types", () => {
    expect(() => TriggerConfigSchema.parse({ type: "unknown" })).toThrow();
  });

  it("rejects a cron trigger missing schedule", () => {
    expect(() => TriggerConfigSchema.parse({ type: "cron" })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// WorkflowSchema
// ---------------------------------------------------------------------------
describe("WorkflowSchema", () => {
  const minimalWorkflow = {
    id: "11111111-1111-1111-1111-111111111111",
    org_id: "22222222-2222-2222-2222-222222222222",
    name: "test-workflow",
    trigger_type: "cron",
    trigger_config: { type: "cron", schedule: "0 * * * *" },
    steps: [],
    is_active: true,
    is_public: false,
    execution_count: 0,
    created_at: "2026-05-11T00:00:00Z",
  };

  it("parses a minimal workflow with numeric execution_count", () => {
    const w = WorkflowSchema.parse(minimalWorkflow);
    expect(w.execution_count).toBe(0);
    expect(w.is_active).toBe(true);
    expect(w.is_public).toBe(false);
  });

  it("parses optional fields as null/undefined", () => {
    const w = WorkflowSchema.parse({
      ...minimalWorkflow,
      onchain_pda: null,
      fee_per_exec_usdc: null,
    });
    expect(w.onchain_pda).toBeNull();
    expect(w.fee_per_exec_usdc).toBeNull();
  });

  it("parses fee_per_exec_usdc as an integer (microUSDC)", () => {
    const w = WorkflowSchema.parse({
      ...minimalWorkflow,
      fee_per_exec_usdc: 500000,
    });
    expect(w.fee_per_exec_usdc).toBe(500000);
  });

  it("defaults execution_count to 0 when absent", () => {
    const { execution_count: _omit, ...rest } = minimalWorkflow;
    void _omit;
    const w = WorkflowSchema.parse(rest);
    expect(w.execution_count).toBe(0);
  });

  it("rejects non-UUID id", () => {
    expect(() => WorkflowSchema.parse({ ...minimalWorkflow, id: "not-a-uuid" })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// StepLogSchema
// ---------------------------------------------------------------------------
describe("StepLogSchema", () => {
  it("parses a successful step log with PascalCase status", () => {
    const s = StepLogSchema.parse({
      step_id: "step-1",
      status: "Success",
      input: { amount: 100 },
      output: { tx: "abc" },
      duration_ms: 42,
    });
    expect(s.status).toBe("Success");
    expect(s.duration_ms).toBe(42);
  });

  it("parses a failed step log with error message", () => {
    const s = StepLogSchema.parse({
      step_id: "step-2",
      status: "Failed",
      input: {},
      output: null,
      duration_ms: 10,
      error: "insufficient funds",
    });
    expect(s.error).toBe("insufficient funds");
  });

  it("rejects lowercase step status", () => {
    expect(() =>
      StepLogSchema.parse({
        step_id: "s1",
        status: "success",
        input: {},
        output: {},
        duration_ms: 0,
      })
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// WorkflowRunSchema
// ---------------------------------------------------------------------------
describe("WorkflowRunSchema", () => {
  const minimalRun = {
    run_id: "11111111-1111-1111-1111-111111111111",
    workflow_id: "22222222-2222-2222-2222-222222222222",
    org_id: "33333333-3333-3333-3333-333333333333",
    status: "Confirmed",
    triggered_by: "manual",
    steps_log: [],
    started_at: "2026-05-11T00:00:00Z",
  };

  it("parses a confirmed run with PascalCase status", () => {
    const r = WorkflowRunSchema.parse(minimalRun);
    expect(r.status).toBe("Confirmed");
    expect(r.steps_log).toEqual([]);
  });

  it("uses steps_log (not steps) as the field name", () => {
    const r = WorkflowRunSchema.parse(minimalRun);
    expect("steps_log" in r).toBe(true);
    expect("steps" in r).toBe(false);
  });

  it("uses error_message (not error) as the error field", () => {
    const r = WorkflowRunSchema.parse({
      ...minimalRun,
      error_message: "something failed",
    });
    expect(r.error_message).toBe("something failed");
    expect("error" in r).toBe(false);
  });

  it("parses optional numeric fields", () => {
    const r = WorkflowRunSchema.parse({
      ...minimalRun,
      slot: 123456789,
      signature: "5abc",
      fee_lamports: 5000,
      jito_tip_lamports: 1000,
    });
    expect(r.slot).toBe(123456789);
    expect(r.fee_lamports).toBe(5000);
  });

  it("parses all RunStatus variants", () => {
    const statuses = [
      "Pending", "Triggered", "Simulating", "Bundling",
      "Submitted", "Confirmed", "Retrying", "Failed", "Skipped",
    ] as const;
    for (const status of statuses) {
      expect(() => WorkflowRunSchema.parse({ ...minimalRun, status })).not.toThrow();
    }
  });

  it("defaults steps_log to empty array when absent", () => {
    const { steps_log: _omit, ...rest } = minimalRun;
    void _omit;
    const r = WorkflowRunSchema.parse(rest);
    expect(r.steps_log).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// RunLogEventSchema
// ---------------------------------------------------------------------------
describe("RunLogEventSchema", () => {
  it("parses a step_log event", () => {
    const e = RunLogEventSchema.parse({
      event: "step_log",
      data: { step_id: "s1", status: "Success", input: {}, output: {}, duration_ms: 10 },
    });
    expect(e.event).toBe("step_log");
  });

  it("parses a run_complete event", () => {
    const e = RunLogEventSchema.parse({
      event: "run_complete",
      data: { run_id: "11111111-1111-1111-1111-111111111111", status: "Confirmed" },
    });
    expect(e.event).toBe("run_complete");
  });

  it("rejects SSE event types not emitted by the backend", () => {
    // The backend only emits step_log and run_complete (api/src/routes/runs.rs:64,70)
    expect(() => RunLogEventSchema.parse({ event: "step_start", data: {} })).toThrow();
    expect(() => RunLogEventSchema.parse({ event: "error", data: {} })).toThrow();
    expect(() => RunLogEventSchema.parse({ event: "step_complete", data: {} })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// OrgSchema
// ---------------------------------------------------------------------------
describe("OrgSchema", () => {
  it("parses an org with integer credits_usdc", () => {
    const o = OrgSchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      name: "Acme Corp",
      wallet_address: null,
      credits_usdc: 1_000_000,
      created_at: "2026-05-11T00:00:00Z",
    });
    expect(o.credits_usdc).toBe(1_000_000);
    expect(o.wallet_address).toBeNull();
  });

  it("parses an org with a wallet address", () => {
    const o = OrgSchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      name: "Solana Foundation",
      wallet_address: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
      credits_usdc: 0,
      created_at: "2026-05-11T00:00:00Z",
    });
    expect(o.wallet_address).toBe("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU");
  });

  it("rejects missing required fields", () => {
    expect(() => OrgSchema.parse({ id: "11111111-1111-1111-1111-111111111111" })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// ApiKeySchema
// ---------------------------------------------------------------------------
describe("ApiKeySchema", () => {
  it("parses an api key with all optional fields null", () => {
    const k = ApiKeySchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      org_id: "22222222-2222-2222-2222-222222222222",
      name: null,
      last_used_at: null,
      created_at: "2026-05-11T00:00:00Z",
      revoked_at: null,
    });
    expect(k.name).toBeNull();
    expect(k.revoked_at).toBeNull();
  });

  it("includes org_id (returned by list_keys endpoint)", () => {
    const k = ApiKeySchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      org_id: "22222222-2222-2222-2222-222222222222",
      name: "production",
      last_used_at: "2026-05-11T00:00:00Z",
      created_at: "2026-05-10T00:00:00Z",
      revoked_at: null,
    });
    expect(k.org_id).toBe("22222222-2222-2222-2222-222222222222");
  });
});

// ---------------------------------------------------------------------------
// AnalyticsSchema
// ---------------------------------------------------------------------------
describe("AnalyticsSchema", () => {
  it("parses analytics response with all i64 fields", () => {
    const a = AnalyticsSchema.parse({
      total_executions: 100,
      successful: 95,
      failed: 5,
      total_fee_lamports: 50000,
    });
    expect(a.total_executions).toBe(100);
    expect(a.successful).toBe(95);
    expect(a.total_fee_lamports).toBe(50000);
  });

  it("does not have plan-only fields like range or success_rate", () => {
    // These fields are NOT in api/src/types.rs:AnalyticsResponse
    const a = AnalyticsSchema.parse({
      total_executions: 0,
      successful: 0,
      failed: 0,
      total_fee_lamports: 0,
    });
    expect("range" in a).toBe(false);
    expect("success_rate" in a).toBe(false);
    expect("credits_remaining" in a).toBe(false);
  });

  it("rejects missing required fields", () => {
    expect(() =>
      AnalyticsSchema.parse({ total_executions: 10, successful: 9 })
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// CreateWorkflowResponseSchema
// ---------------------------------------------------------------------------
describe("CreateWorkflowResponseSchema", () => {
  it("parses a create workflow response", () => {
    const r = CreateWorkflowResponseSchema.parse({
      workflow_id: "11111111-1111-1111-1111-111111111111",
      status: "created",
      next_run: null,
      onchain_pda: null,
    });
    expect(r.status).toBe("created");
    expect(r.workflow_id).toBe("11111111-1111-1111-1111-111111111111");
  });
});

// ---------------------------------------------------------------------------
// CreateApiKeyResponseSchema
// ---------------------------------------------------------------------------
describe("CreateApiKeyResponseSchema", () => {
  it("parses a create api key response including the raw key", () => {
    const r = CreateApiKeyResponseSchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      key: "sk_abcdef1234567890",
      name: "my-key",
    });
    expect(r.key).toBe("sk_abcdef1234567890");
  });

  it("allows null name", () => {
    const r = CreateApiKeyResponseSchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      key: "sk_abc",
      name: null,
    });
    expect(r.name).toBeNull();
  });
});
