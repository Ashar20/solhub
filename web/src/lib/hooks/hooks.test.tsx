import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactNode } from "react";
import { useMe } from "./use-org";
import { useWorkflows, useWorkflow } from "./use-workflows";
import { useRuns, useRun } from "./use-runs";
import { useHub } from "./use-hub";
import { useAnalytics } from "./use-analytics";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.setItem("solhub.bearer", "test-key");
});

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

function jsonRes(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    text: async () => JSON.stringify(body),
  } as Response;
}

describe("useMe", () => {
  it("fetches /v1/orgs/me", async () => {
    F.mockResolvedValueOnce(
      jsonRes({
        id: "11111111-1111-1111-1111-111111111111",
        name: "test",
        wallet_address: null,
        credits_usdc: 0,
        created_at: "2026-05-11T00:00:00Z",
      }),
    );
    const { result } = renderHook(() => useMe(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.name).toBe("test");
  });
});

describe("useWorkflow", () => {
  it("is disabled when id is 'new'", async () => {
    const { result } = renderHook(() => useWorkflow("new"), { wrapper });
    // No fetch should fire
    await new Promise((r) => setTimeout(r, 50));
    expect(F).not.toHaveBeenCalled();
    expect(result.current.fetchStatus).toBe("idle");
  });
});

describe("useWorkflows", () => {
  it("queries with key including params", async () => {
    F.mockResolvedValueOnce(jsonRes([]));
    const { result } = renderHook(() => useWorkflows({ active_only: true }), {
      wrapper,
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(String(F.mock.calls[0]![0])).toContain("active_only=true");
  });
});

describe("useRuns", () => {
  it("queries /v1/runs", async () => {
    F.mockResolvedValueOnce(jsonRes([]));
    const { result } = renderHook(() => useRuns(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});

describe("useRun", () => {
  it("is disabled when run_id is undefined", async () => {
    renderHook(() => useRun(undefined), { wrapper });
    await new Promise((r) => setTimeout(r, 50));
    expect(F).not.toHaveBeenCalled();
  });
});

describe("useHub", () => {
  it("queries /v1/hub anonymously", async () => {
    F.mockResolvedValueOnce(jsonRes([]));
    const { result } = renderHook(() => useHub(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).headers).not.toHaveProperty("Authorization");
  });
});

describe("useAnalytics", () => {
  it("queries /v1/analytics", async () => {
    F.mockResolvedValueOnce(
      jsonRes({
        total_executions: 10,
        successful: 9,
        failed: 1,
        total_fee_lamports: 1000,
      }),
    );
    const { result } = renderHook(() => useAnalytics(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.total_executions).toBe(10);
  });
});
