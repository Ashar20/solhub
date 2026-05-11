import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { WorkflowRow } from "./WorkflowRow";

const mockPush = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush, replace: vi.fn() }),
}));

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  mockPush.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.setItem("solhub.bearer", "k");
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

function wrap(node: ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return <QueryClientProvider client={qc}>{node}</QueryClientProvider>;
}

const sample = {
  id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  name: "test wf",
  trigger_type: "cron",
  is_active: true,
  execution_count: 5,
  created_at: "2026-05-11T00:00:00Z",
  updated_at: "2026-05-11T00:00:00Z",
};

function ok(body: unknown) {
  return { ok: true, status: 200, statusText: "OK", text: async () => JSON.stringify(body) } as Response;
}

describe("WorkflowRow", () => {
  it("renders name and trigger pill", () => {
    render(wrap(<WorkflowRow w={sample} />));
    expect(screen.getByText("test wf")).toBeInTheDocument();
    expect(screen.getByText("cron")).toBeInTheDocument();
  });

  it("trigger button POSTs and navigates", async () => {
    F.mockResolvedValueOnce(ok({ run_id: "11111111-1111-1111-1111-111111111111", status: "Pending" }));
    render(wrap(<WorkflowRow w={sample} />));
    fireEvent.click(screen.getByLabelText("Trigger"));
    await waitFor(() => expect(mockPush).toHaveBeenCalledWith("/runs/11111111-1111-1111-1111-111111111111"));
  });

  it("pause button PATCHes with is_active false when active", async () => {
    F.mockResolvedValueOnce(ok({ id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", name: "test wf", trigger_type: "cron", is_active: false, execution_count: 5, created_at: "2026-05-11T00:00:00Z", updated_at: "2026-05-11T00:00:00Z", trigger_config: {}, org_id: "22222222-2222-2222-2222-222222222222", is_public: false, steps: [] }));
    render(wrap(<WorkflowRow w={sample} />));
    fireEvent.click(screen.getByLabelText("Pause"));
    await waitFor(() => expect(F).toHaveBeenCalled());
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).method).toBe("PATCH");
  });

  it("delete button DELETEs after confirm", async () => {
    F.mockResolvedValueOnce(ok({ status: "deleted" }));
    render(wrap(<WorkflowRow w={sample} />));
    fireEvent.click(screen.getByLabelText("Delete"));
    await waitFor(() => expect(F).toHaveBeenCalled());
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).method).toBe("DELETE");
  });
});
