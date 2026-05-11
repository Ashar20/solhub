import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { WorkflowPicker } from "./WorkflowPicker";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.setItem("solhub.bearer", "k");
});

function wrap(node: ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{node}</QueryClientProvider>;
}

function jsonRes(body: unknown) {
  return { ok: true, status: 200, statusText: "OK", text: async () => JSON.stringify(body) } as Response;
}

const wfShape = (id: string, name: string, active = true) => ({
  id,
  org_id: "22222222-2222-2222-2222-222222222222",
  name,
  trigger_type: "manual",
  trigger_config: {},
  steps: [],
  is_active: active,
  is_public: false,
  onchain_pda: null,
  fee_per_exec_usdc: null,
  execution_count: 0,
  created_at: "2026-05-11T00:00:00Z",
  updated_at: "2026-05-11T00:00:00Z",
});

describe("WorkflowPicker", () => {
  it("shows placeholder when no value", async () => {
    F.mockResolvedValueOnce(jsonRes([]));
    render(wrap(<WorkflowPicker value="" onChange={() => {}} />));
    // Wait for the loading state to resolve before asserting the placeholder
    await waitFor(() => expect(screen.getByText(/select workflow/i)).toBeInTheDocument());
  });

  it("shows selected workflow's name when value matches", async () => {
    const id = "11111111-1111-1111-1111-111111111111";
    F.mockResolvedValueOnce(jsonRes([wfShape(id, "the picked one")]));
    render(wrap(<WorkflowPicker value={id} onChange={() => {}} />));
    // Wait for query to resolve
    await screen.findByText("the picked one");
  });

  it("excludes currentWorkflowId from list", async () => {
    const selfId = "11111111-1111-1111-1111-111111111111";
    const otherId = "22222222-2222-2222-2222-222222222222";
    F.mockResolvedValueOnce(jsonRes([wfShape(selfId, "myself"), wfShape(otherId, "another")]));
    const onChange = vi.fn();
    render(wrap(<WorkflowPicker value="" onChange={onChange} excludeId={selfId} />));
    fireEvent.click(screen.getByRole("button"));
    await screen.findByText("another");
    expect(screen.queryByText("myself")).not.toBeInTheDocument();
  });

  it("fires onChange when an item is clicked", async () => {
    const otherId = "22222222-2222-2222-2222-222222222222";
    F.mockResolvedValueOnce(jsonRes([wfShape(otherId, "pickable")]));
    const onChange = vi.fn();
    render(wrap(<WorkflowPicker value="" onChange={onChange} />));
    fireEvent.click(screen.getByRole("button"));
    await screen.findByText("pickable");
    fireEvent.click(screen.getByText("pickable"));
    expect(onChange).toHaveBeenCalledWith(otherId);
  });
});
