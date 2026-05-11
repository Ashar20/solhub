import { beforeEach, describe, it, expect, vi } from "vitest";
import { orgs, workflows, runs, hub, analytics } from ".";
import { setToken, clearToken } from "./client";

// ---------------------------------------------------------------------------
// Shared mock setup
// ---------------------------------------------------------------------------

const F = vi.fn();

beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  clearToken();
  setToken("test-key");
});

function ok(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    text: async () => JSON.stringify(body),
  } as Response;
}

function deleted(): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    text: async () => JSON.stringify({ status: "deleted" }),
  } as Response;
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const ORG = {
  id: "11111111-1111-1111-1111-111111111111",
  name: "Acme",
  wallet_address: null,
  credits_usdc: 500,
  created_at: "2026-05-11T00:00:00Z",
};

const API_KEY = {
  id: "22222222-2222-2222-2222-222222222222",
  org_id: "11111111-1111-1111-1111-111111111111",
  name: "ci-key",
  last_used_at: null,
  created_at: "2026-05-11T00:00:00Z",
  revoked_at: null,
};

const CREATE_KEY_RESP = {
  id: "22222222-2222-2222-2222-222222222222",
  key: "sk_abc123",
  name: "ci-key",
};

const WORKFLOW = {
  id: "33333333-3333-3333-3333-333333333333",
  org_id: "11111111-1111-1111-1111-111111111111",
  name: "My Workflow",
  trigger_type: "manual",
  trigger_config: { type: "manual" },
  steps: [],
  is_active: true,
  is_public: false,
  onchain_pda: null,
  fee_per_exec_usdc: null,
  execution_count: 0,
  created_at: "2026-05-11T00:00:00Z",
};

const CREATE_WF_RESP = {
  workflow_id: "33333333-3333-3333-3333-333333333333",
  status: "created",
  next_run: null,
  onchain_pda: null,
};

const TRIGGER_RESP = {
  run_id: "44444444-4444-4444-4444-444444444444",
  status: "Pending",
};

const RUN = {
  run_id: "44444444-4444-4444-4444-444444444444",
  workflow_id: "33333333-3333-3333-3333-333333333333",
  org_id: "11111111-1111-1111-1111-111111111111",
  status: "Pending",
  triggered_by: "manual",
  steps_log: [],
  slot: null,
  signature: null,
  fee_lamports: null,
  jito_tip_lamports: null,
  error_message: null,
  started_at: "2026-05-11T00:00:00Z",
  completed_at: null,
};

const ANALYTICS = {
  total_executions: 10,
  successful: 8,
  failed: 2,
  total_fee_lamports: 50000,
};

// ---------------------------------------------------------------------------
// orgs
// ---------------------------------------------------------------------------

describe("orgs.getMe", () => {
  it("GETs /v1/orgs/me with Bearer", async () => {
    F.mockResolvedValueOnce(ok(ORG));
    const result = await orgs.getMe();
    expect(result.name).toBe("Acme");
    expect(result.credits_usdc).toBe(500);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/orgs/me");
    expect((init as RequestInit).method ?? "GET").toBe("GET");
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: "Bearer test-key",
    });
  });
});

describe("orgs.listApiKeys", () => {
  it("GETs /v1/orgs/me/api_keys with Bearer", async () => {
    F.mockResolvedValueOnce(ok([API_KEY]));
    const keys = await orgs.listApiKeys();
    expect(keys).toHaveLength(1);
    expect(keys[0]!.name).toBe("ci-key");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/orgs/me/api_keys");
    expect((init as RequestInit).method ?? "GET").toBe("GET");
  });
});

describe("orgs.createApiKey", () => {
  it("POSTs /v1/orgs/me/api_keys with name in body", async () => {
    F.mockResolvedValueOnce(ok(CREATE_KEY_RESP));
    const result = await orgs.createApiKey("ci-key");
    expect(result.key).toBe("sk_abc123");
    expect(result.id).toBe("22222222-2222-2222-2222-222222222222");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/orgs/me/api_keys");
    expect((init as RequestInit).method).toBe("POST");
    expect(JSON.parse((init as RequestInit).body as string)).toMatchObject({
      name: "ci-key",
    });
  });

  it("POSTs without name when called with no args", async () => {
    F.mockResolvedValueOnce(ok(CREATE_KEY_RESP));
    await orgs.createApiKey();
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).method).toBe("POST");
  });
});

describe("orgs.revokeApiKey", () => {
  it("DELETEs /v1/orgs/me/api_keys/:id", async () => {
    F.mockResolvedValueOnce(ok({ status: "revoked" }));
    await orgs.revokeApiKey("22222222-2222-2222-2222-222222222222");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/orgs/me/api_keys/22222222-2222-2222-2222-222222222222",
    );
    expect((init as RequestInit).method).toBe("DELETE");
  });
});

// ---------------------------------------------------------------------------
// workflows
// ---------------------------------------------------------------------------

describe("workflows.listWorkflows", () => {
  it("GETs /v1/workflows with no params by default", async () => {
    F.mockResolvedValueOnce(ok([WORKFLOW]));
    const result = await workflows.listWorkflows();
    expect(result).toHaveLength(1);
    const [url] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/workflows");
    expect(String(url)).not.toContain("active_only");
  });

  it("encodes active_only param when provided", async () => {
    F.mockResolvedValueOnce(ok([WORKFLOW]));
    await workflows.listWorkflows({ active_only: true });
    const [url] = F.mock.calls[0]!;
    expect(String(url)).toContain("active_only=true");
  });
});

describe("workflows.getWorkflow", () => {
  it("GETs /v1/workflows/:id with Bearer", async () => {
    F.mockResolvedValueOnce(ok(WORKFLOW));
    const wf = await workflows.getWorkflow(
      "33333333-3333-3333-3333-333333333333",
    );
    expect(wf.name).toBe("My Workflow");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/workflows/33333333-3333-3333-3333-333333333333",
    );
    expect((init as RequestInit).method ?? "GET").toBe("GET");
  });
});

describe("workflows.createWorkflow", () => {
  it("POSTs /v1/workflows with nested trigger object", async () => {
    F.mockResolvedValueOnce(ok(CREATE_WF_RESP));
    const result = await workflows.createWorkflow({
      name: "My Workflow",
      trigger: { type: "manual" },
      steps: [],
    });
    expect(result.workflow_id).toBe("33333333-3333-3333-3333-333333333333");
    expect(result.status).toBe("created");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/workflows");
    expect((init as RequestInit).method).toBe("POST");
    const body = JSON.parse((init as RequestInit).body as string);
    expect(body.trigger).toMatchObject({ type: "manual" });
    expect(body.name).toBe("My Workflow");
  });
});

describe("workflows.updateWorkflow", () => {
  it("PATCHes /v1/workflows/:id with partial body", async () => {
    F.mockResolvedValueOnce(ok({ ...WORKFLOW, is_active: false }));
    const result = await workflows.updateWorkflow(
      "33333333-3333-3333-3333-333333333333",
      { is_active: false },
    );
    expect(result.is_active).toBe(false);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/workflows/33333333-3333-3333-3333-333333333333",
    );
    expect((init as RequestInit).method).toBe("PATCH");
  });
});

describe("workflows.deleteWorkflow", () => {
  it("DELETEs /v1/workflows/:id", async () => {
    F.mockResolvedValueOnce(deleted());
    await workflows.deleteWorkflow("33333333-3333-3333-3333-333333333333");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/workflows/33333333-3333-3333-3333-333333333333",
    );
    expect((init as RequestInit).method).toBe("DELETE");
  });
});

describe("workflows.triggerWorkflow", () => {
  it("POSTs /v1/workflows/:id/trigger", async () => {
    F.mockResolvedValueOnce(ok(TRIGGER_RESP));
    const result = await workflows.triggerWorkflow(
      "33333333-3333-3333-3333-333333333333",
    );
    expect(result.run_id).toBe("44444444-4444-4444-4444-444444444444");
    expect(result.status).toBe("Pending");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/workflows/33333333-3333-3333-3333-333333333333/trigger",
    );
    expect((init as RequestInit).method).toBe("POST");
  });

  it("sends param_overrides when provided", async () => {
    F.mockResolvedValueOnce(ok(TRIGGER_RESP));
    await workflows.triggerWorkflow("33333333-3333-3333-3333-333333333333", {
      amount: 100,
    });
    const [, init] = F.mock.calls[0]!;
    const body = JSON.parse((init as RequestInit).body as string);
    expect(body.param_overrides).toMatchObject({ amount: 100 });
  });
});

// ---------------------------------------------------------------------------
// runs
// ---------------------------------------------------------------------------

describe("runs.listRuns", () => {
  it("GETs /v1/runs with no params by default", async () => {
    F.mockResolvedValueOnce(ok([RUN]));
    const result = await runs.listRuns();
    expect(result).toHaveLength(1);
    const [url] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/runs");
  });

  it("encodes workflow_id, status and limit params", async () => {
    F.mockResolvedValueOnce(ok([RUN]));
    await runs.listRuns({
      workflow_id: "33333333-3333-3333-3333-333333333333",
      status: "Pending",
      limit: 25,
    });
    const [url] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "workflow_id=33333333-3333-3333-3333-333333333333",
    );
    expect(String(url)).toContain("status=Pending");
    expect(String(url)).toContain("limit=25");
  });
});

describe("runs.getRun", () => {
  it("GETs /v1/runs/:run_id with Bearer", async () => {
    F.mockResolvedValueOnce(ok(RUN));
    const result = await runs.getRun("44444444-4444-4444-4444-444444444444");
    expect(result.run_id).toBe("44444444-4444-4444-4444-444444444444");
    expect(result.status).toBe("Pending");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/runs/44444444-4444-4444-4444-444444444444",
    );
    expect((init as RequestInit).method ?? "GET").toBe("GET");
  });
});

describe("runs.runStreamUrl", () => {
  it("returns the SSE URL for the given run_id", () => {
    const url = runs.runStreamUrl("44444444-4444-4444-4444-444444444444");
    expect(url).toContain(
      "/v1/runs/44444444-4444-4444-4444-444444444444/logs",
    );
  });
});

// ---------------------------------------------------------------------------
// hub
// ---------------------------------------------------------------------------

describe("hub.listHub", () => {
  it("GETs /v1/hub WITHOUT Bearer (anonymous)", async () => {
    F.mockResolvedValueOnce(ok([{ ...WORKFLOW, is_public: true }]));
    const result = await hub.listHub();
    expect(result).toHaveLength(1);
    expect(result[0]!.is_public).toBe(true);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/hub");
    expect(String(url)).not.toContain("/v1/hub/");
    expect((init as RequestInit).headers).not.toHaveProperty("Authorization");
  });
});

describe("hub.publishToHub", () => {
  it("POSTs /v1/hub/publish with required body fields", async () => {
    F.mockResolvedValueOnce(ok({ ...WORKFLOW, is_public: true }));
    const result = await hub.publishToHub({
      workflow_id: "33333333-3333-3333-3333-333333333333",
      fee_per_execution_usdc: 0.01,
    });
    expect(result.is_public).toBe(true);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/hub/publish");
    expect((init as RequestInit).method).toBe("POST");
    const body = JSON.parse((init as RequestInit).body as string);
    expect(body.workflow_id).toBe("33333333-3333-3333-3333-333333333333");
    expect(body.fee_per_execution_usdc).toBe(0.01);
  });
});

describe("hub.callHubWorkflow", () => {
  it("POSTs /v1/hub/:id/call with Bearer", async () => {
    F.mockResolvedValueOnce(
      ok({ run_id: "44444444-4444-4444-4444-444444444444", status: "Pending" }),
    );
    const result = await hub.callHubWorkflow(
      "33333333-3333-3333-3333-333333333333",
    );
    expect(result.run_id).toBe("44444444-4444-4444-4444-444444444444");
    expect(result.status).toBe("Pending");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/hub/33333333-3333-3333-3333-333333333333/call",
    );
    expect((init as RequestInit).method).toBe("POST");
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: "Bearer test-key",
    });
  });
});

// ---------------------------------------------------------------------------
// analytics
// ---------------------------------------------------------------------------

describe("analytics.getAnalytics", () => {
  it("GETs /v1/analytics with Bearer", async () => {
    F.mockResolvedValueOnce(ok(ANALYTICS));
    const result = await analytics.getAnalytics();
    expect(result.total_executions).toBe(10);
    expect(result.successful).toBe(8);
    expect(result.failed).toBe(2);
    expect(result.total_fee_lamports).toBe(50000);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/analytics");
    expect((init as RequestInit).method ?? "GET").toBe("GET");
    expect((init as RequestInit).headers).toMatchObject({
      Authorization: "Bearer test-key",
    });
  });
});
