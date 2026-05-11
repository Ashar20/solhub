import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useRunStream } from "./use-run-stream";

class MockEventSource {
  static instances: MockEventSource[] = [];
  url: string;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  closed = false;

  constructor(url: string) {
    this.url = url;
    MockEventSource.instances.push(this);
  }

  close() {
    this.closed = true;
  }

  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent);
  }

  fail() {
    this.onerror?.(new Event("error"));
  }
}

beforeEach(() => {
  MockEventSource.instances = [];
  vi.stubGlobal("EventSource", MockEventSource);
  window.localStorage.setItem("solhub.bearer", "test-token");
});

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

describe("useRunStream", () => {
  it("starts in streaming state when given a run_id", async () => {
    const { result } = renderHook(() => useRunStream("run-1"), { wrapper });
    await waitFor(() => expect(result.current.state).toBe("streaming"));
    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0]!.url).toContain("/api/runs/run-1/logs");
    expect(MockEventSource.instances[0]!.url).toContain("token=test-token");
  });

  it("collects step_log events", async () => {
    const { result } = renderHook(() => useRunStream("run-2"), { wrapper });
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    act(() =>
      MockEventSource.instances[0]!.emit({
        event: "step_log",
        data: { msg: "hi" },
      }),
    );
    await waitFor(() => expect(result.current.events).toHaveLength(1));
    expect(result.current.events[0]!.event).toBe("step_log");
  });

  it("closes on run_complete event", async () => {
    const { result } = renderHook(() => useRunStream("run-3"), { wrapper });
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    const es = MockEventSource.instances[0]!;
    act(() =>
      es.emit({
        event: "run_complete",
        data: {},
      }),
    );
    await waitFor(() => expect(result.current.state).toBe("closed"));
    expect(es.closed).toBe(true);
  });

  it("falls back to polling on error", async () => {
    const { result } = renderHook(() => useRunStream("run-4"), { wrapper });
    await waitFor(() => expect(MockEventSource.instances).toHaveLength(1));
    act(() => MockEventSource.instances[0]!.fail());
    await waitFor(() => expect(result.current.state).toBe("polling"));
  });
});
