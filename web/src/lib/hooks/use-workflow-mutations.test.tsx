import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import {
  useCreateWorkflow,
  useUpdateWorkflow,
  useDeleteWorkflow,
  useTriggerWorkflow,
} from "./use-workflow-mutations";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.setItem("solhub.bearer", "k");
});

function wrap({ children }: { children: ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

function ok(body: unknown, status = 200) {
  return {
    ok: status < 400,
    status,
    statusText: "OK",
    text: async () => JSON.stringify(body),
  } as Response;
}

describe("useCreateWorkflow", () => {
  it("POSTs to /v1/workflows", async () => {
    F.mockResolvedValueOnce(
      ok({
        workflow_id: "11111111-1111-1111-1111-111111111111",
        status: "created",
      }),
    );
    const { result } = renderHook(() => useCreateWorkflow(), { wrapper: wrap });
    await act(async () => {
      await result.current.mutateAsync({
        name: "x",
        trigger: { type: "cron", schedule: "0 * * * *" },
        steps: [],
      });
    });
    expect(F.mock.calls.length).toBeGreaterThanOrEqual(1);
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/workflows");
    expect((init as RequestInit).method).toBe("POST");
  });
});

describe("useUpdateWorkflow", () => {
  it("PATCHes /v1/workflows/:id", async () => {
    F.mockResolvedValueOnce(
      ok({
        id: "11111111-1111-1111-1111-111111111111",
        org_id: "22222222-2222-2222-2222-222222222222",
        name: "x",
        trigger_type: "cron",
        trigger_config: { schedule: "0 * * * *" },
        steps: [],
        is_active: false,
        is_public: false,
        execution_count: 0,
        created_at: "2026-05-11T00:00:00Z",
        updated_at: "2026-05-11T00:00:00Z",
      }),
    );
    const { result } = renderHook(
      () => useUpdateWorkflow("11111111-1111-1111-1111-111111111111"),
      { wrapper: wrap },
    );
    await act(async () => {
      await result.current.mutateAsync({ is_active: false });
    });
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain(
      "/v1/workflows/11111111-1111-1111-1111-111111111111",
    );
    expect((init as RequestInit).method).toBe("PATCH");
  });
});

describe("useDeleteWorkflow", () => {
  it("DELETEs /v1/workflows/:id", async () => {
    F.mockResolvedValueOnce(ok({ status: "deleted" }));
    const { result } = renderHook(() => useDeleteWorkflow(), {
      wrapper: wrap,
    });
    await act(async () => {
      await result.current.mutateAsync("abc");
    });
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).method).toBe("DELETE");
  });
});

describe("useTriggerWorkflow", () => {
  it("POSTs /v1/workflows/:id/trigger", async () => {
    F.mockResolvedValueOnce(
      ok({
        run_id: "11111111-1111-1111-1111-111111111111",
        status: "Pending",
      }),
    );
    const { result } = renderHook(() => useTriggerWorkflow(), {
      wrapper: wrap,
    });
    await act(async () => {
      await result.current.mutateAsync({ id: "wf-1" });
    });
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/workflows/wf-1/trigger");
    expect((init as RequestInit).method).toBe("POST");
  });
});
