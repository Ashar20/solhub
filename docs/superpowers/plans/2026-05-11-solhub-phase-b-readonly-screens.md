# Solhub Frontend — Phase B: Read-Only Screens Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the first six screens end-to-end against the Rust API: Login, Dashboard, Workflows list, Runs list, Run detail (incl. live log stream), Marketplace list. After Phase B the app is demo-able.

**Architecture:** TanStack Query hooks per resource. Server-state lives in the cache; UI components are dumb renderers. SSE handled by `useRunStream` with a polling fallback so the run-detail page works even before Backend Gap #1 ships.

**Tech Stack:** Same as Phase A. No new deps.

**Pre-requisite:** Phase A complete (`docs/superpowers/plans/2026-05-11-solhub-phase-a-foundation.md`).

**Reference:** spec §6 (screens table), §7 (SSE), §10 (Solana-specific UI bits).

**Commit policy:** `git add web/` only. Plan/spec docs stay uncommitted.

---

## Task 1: TanStack Query hooks for the API surface

**Files:**
- Create: `web/src/lib/hooks/use-workflows.ts`
- Create: `web/src/lib/hooks/use-runs.ts`
- Create: `web/src/lib/hooks/use-hub.ts`
- Create: `web/src/lib/hooks/use-org.ts`
- Create: `web/src/lib/hooks/use-analytics.ts`

- [ ] **Step 1: `use-org.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { orgs } from "@/lib/api";

export const useMe = () => useQuery({
  queryKey: ["org", "me"] as const,
  queryFn: orgs.getMe,
});
```

- [ ] **Step 2: `use-workflows.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { workflows } from "@/lib/api";
import type { ListWorkflowsParams } from "@/lib/api/workflows";

export const useWorkflows = (params: ListWorkflowsParams = {}) => useQuery({
  queryKey: ["workflows", params] as const,
  queryFn: () => workflows.listWorkflows(params),
});

export const useWorkflow = (id: string | undefined) => useQuery({
  queryKey: ["workflow", id] as const,
  queryFn: () => workflows.getWorkflow(id!),
  enabled: !!id && id !== "new",
});
```

- [ ] **Step 3: `use-runs.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { runs } from "@/lib/api";
import type { ListRunsParams } from "@/lib/api/runs";

export const useRuns = (params: ListRunsParams = {}) => useQuery({
  queryKey: ["runs", params] as const,
  queryFn: () => runs.listRuns(params),
  refetchInterval: typeof document !== "undefined" && document.visibilityState === "visible" ? 5000 : false,
});

export const useRun = (run_id: string | undefined, pollMs?: number) => useQuery({
  queryKey: ["run", run_id] as const,
  queryFn: () => runs.getRun(run_id!),
  enabled: !!run_id,
  refetchInterval: pollMs,
});
```

- [ ] **Step 4: `use-hub.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { hub } from "@/lib/api";

export const useHub = () => useQuery({
  queryKey: ["hub"] as const,
  queryFn: hub.listHub,
});

export const useHubWorkflow = (id: string | undefined) => useQuery({
  queryKey: ["hub", id] as const,
  queryFn: () => hub.getHubWorkflow(id!),
  enabled: !!id,
});
```

- [ ] **Step 5: `use-analytics.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { analytics } from "@/lib/api";

export const useAnalytics = (range: "1d" | "7d" | "30d" = "7d") => useQuery({
  queryKey: ["analytics", range] as const,
  queryFn: () => analytics.getAnalytics(range),
});
```

- [ ] **Step 6: Typecheck + commit**

```bash
cd web && pnpm typecheck
git add web/
git commit -m "feat(web): TanStack Query hooks for API resources"
```

---

## Task 2: Login screen

**Files:**
- Replace: `web/src/app/(auth)/login/page.tsx`

- [ ] **Step 1: Implement**

```tsx
"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth/use-auth";
import { orgs, ApiError, setToken } from "@/lib/api";
import { Btn } from "@/components/primitives/Btn";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";

export default function LoginPage() {
  const router = useRouter();
  const { signIn } = useAuth();
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    setToken(value); // probe with the proposed token
    try {
      await orgs.getMe();
      signIn(value);
      router.replace("/dashboard");
    } catch (err) {
      const msg = err instanceof ApiError ? `Invalid key (${err.status})` : "Network error";
      setError(msg);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="w-[400px] rounded-xl border border-ink-200 bg-white shadow-card p-8 space-y-4">
      <SolhubLogo />
      <div>
        <h1 className="text-[20px] font-semibold tracking-tight">Sign in</h1>
        <p className="text-[12px] text-ink-500 mt-1">
          Paste your API key from the SolHub backend. Stored locally in your browser.
        </p>
      </div>
      <label className="block">
        <span className="text-[12px] font-medium text-ink-700">API key</span>
        <input
          type="password"
          autoFocus
          autoComplete="off"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px] font-mono focus:outline-none focus:ring-2 focus:ring-violet-500/30"
          placeholder="sk_live_…"
        />
      </label>
      {error && <p className="text-[12px] text-rose-600">{error}</p>}
      <Btn type="submit" variant="primary" size="lg" disabled={busy || !value} className="w-full justify-center">
        {busy ? "Verifying…" : "Sign in"}
      </Btn>
    </form>
  );
}
```

- [ ] **Step 2: Smoke**

```bash
pnpm dev
```

- With backend running: paste valid key → lands on dashboard.
- With backend running, wrong key → "Invalid key (401)" stays on /login.
- Backend off: shows "Network error".

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): login screen with API-key probe"
```

---

## Task 3: Dashboard

**Files:**
- Replace: `web/src/app/(app)/dashboard/page.tsx`
- Create: `web/src/components/dashboard/KpiCard.tsx`
- Create: `web/src/components/dashboard/RecentList.tsx`

- [ ] **Step 1: `KpiCard.tsx`**

```tsx
import { cn } from "@/lib/utils/cn";

export function KpiCard({ label, value, sub, className }: {
  label: string; value: React.ReactNode; sub?: React.ReactNode; className?: string;
}) {
  return (
    <div className={cn("rounded-xl border border-ink-200 bg-white shadow-card p-4", className)}>
      <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500">{label}</div>
      <div className="mt-1 text-[22px] font-semibold tracking-tight text-ink-900">{value}</div>
      {sub && <div className="mt-1 text-[11px] text-ink-500">{sub}</div>}
    </div>
  );
}
```

- [ ] **Step 2: `RecentList.tsx`**

```tsx
import Link from "next/link";

export function RecentList({ title, items, emptyText }: {
  title: string;
  items: { id: string; primary: React.ReactNode; secondary?: React.ReactNode; href: string }[];
  emptyText: string;
}) {
  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card">
      <div className="px-4 h-10 border-b border-ink-200 flex items-center text-[12px] font-medium text-ink-700">{title}</div>
      <ul>
        {items.length === 0 && (
          <li className="px-4 py-6 text-[12px] text-ink-500">{emptyText}</li>
        )}
        {items.map((it) => (
          <li key={it.id} className="border-b border-ink-100 last:border-b-0">
            <Link href={it.href} className="block px-4 py-2.5 hover:bg-ink-50 text-[13px]">
              <div className="text-ink-900 font-medium truncate">{it.primary}</div>
              {it.secondary && <div className="text-[11px] text-ink-500 mt-0.5">{it.secondary}</div>}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 3: Dashboard page**

```tsx
"use client";
import { Topbar } from "@/components/shell/Topbar";
import { KpiCard } from "@/components/dashboard/KpiCard";
import { RecentList } from "@/components/dashboard/RecentList";
import { useAnalytics } from "@/lib/hooks/use-analytics";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { useRuns } from "@/lib/hooks/use-runs";
import { formatLamports, formatUsdc, formatRelativeTime } from "@/lib/utils/format";

export default function Dashboard() {
  const analytics = useAnalytics("7d");
  const workflows = useWorkflows({ limit: 5 });
  const runs = useRuns({ limit: 10 });

  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "Dashboard"]} />
      <main className="flex-1 p-6 grid-bg overflow-y-auto">
        <div className="grid grid-cols-4 gap-3 mb-4">
          <KpiCard
            label="Executions · 7d"
            value={analytics.data?.executions ?? "—"}
          />
          <KpiCard
            label="Success rate"
            value={analytics.data ? `${(analytics.data.success_rate * 100).toFixed(1)}%` : "—"}
          />
          <KpiCard
            label="Fee spend · 7d"
            value={analytics.data ? formatLamports(analytics.data.fee_spend_lamports) : "—"}
          />
          <KpiCard
            label="Credits"
            value={analytics.data ? formatUsdc(analytics.data.credits_remaining) : "—"}
          />
        </div>
        <div className="grid grid-cols-2 gap-4">
          <RecentList
            title="Recent workflows"
            emptyText={workflows.isLoading ? "Loading…" : "No workflows yet."}
            items={(workflows.data ?? []).map((w) => ({
              id: w.id,
              primary: w.name,
              secondary: `${w.trigger.type} · ${w.is_active ? "active" : "paused"}`,
              href: `/workflows/${w.id}`,
            }))}
          />
          <RecentList
            title="Recent runs"
            emptyText={runs.isLoading ? "Loading…" : "No runs yet."}
            items={(runs.data ?? []).map((r) => ({
              id: r.run_id,
              primary: r.status,
              secondary: formatRelativeTime(r.started_at),
              href: `/runs/${r.run_id}`,
            }))}
          />
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 4: Smoke + commit**

```bash
pnpm dev
# visit /dashboard with valid token
git add web/
git commit -m "feat(web): dashboard with KPIs + recent workflows/runs"
```

---

## Task 4: Workflows list (read-only)

**Files:**
- Create: `web/src/app/(app)/workflows/page.tsx`
- Create: `web/src/components/workflow/WorkflowRow.tsx`
- Create: `web/src/components/workflow/StatusPill.tsx`

Note: Mutations (trigger, enable/disable, delete) ship in Phase C. This task only renders the list with filters and search.

- [ ] **Step 1: `StatusPill.tsx`**

```tsx
import { Pill } from "@/components/primitives/Pill";

export function StatusPill({ active }: { active: boolean }) {
  return active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">paused</Pill>;
}
```

- [ ] **Step 2: `WorkflowRow.tsx`**

```tsx
import Link from "next/link";
import type { Workflow } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";
import { StatusPill } from "./StatusPill";

export function WorkflowRow({ w }: { w: Workflow }) {
  return (
    <Link
      href={`/workflows/${w.id}`}
      className="grid grid-cols-[1fr_140px_120px_120px_80px] items-center px-4 h-12 border-b border-ink-100 hover:bg-ink-50 text-[13px]"
    >
      <div className="flex items-center gap-2 min-w-0">
        <div className="font-medium text-ink-900 truncate">{w.name}</div>
        <Pill tone="ink">{w.trigger.type}</Pill>
      </div>
      <StatusPill active={w.is_active} />
      <div className="text-ink-500 font-mono">{w.execution_count.toString()} runs</div>
      <div className="text-ink-500">—</div>
      <div className="text-ink-500 text-right">v—</div>
    </Link>
  );
}
```

- [ ] **Step 3: Page**

```tsx
"use client";
import { useState } from "react";
import { Topbar } from "@/components/shell/Topbar";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { WorkflowRow } from "@/components/workflow/WorkflowRow";
import Link from "next/link";

type Status = "all" | "active" | "inactive";

export default function WorkflowsList() {
  const [status, setStatus] = useState<Status>("all");
  const [q, setQ] = useState("");
  const { data, isLoading } = useWorkflows({ status: status === "all" ? undefined : status });

  const filtered = (data ?? []).filter((w) =>
    q.trim() === "" ? true : w.name.toLowerCase().includes(q.toLowerCase()),
  );

  const tabClass = (s: Status) =>
    "h-8 px-3 text-[12px] font-medium rounded-md " +
    (status === s ? "bg-ink-900 text-white" : "text-ink-600 hover:bg-ink-100");

  return (
    <>
      <Topbar
        crumbs={["Workspace", "solhub-prod", "Workflows"]}
        right={
          <Link href="/workflows/new">
            <Btn variant="primary" icon={<Icon name="plus" className="w-3.5 h-3.5" />}>
              New workflow
            </Btn>
          </Link>
        }
      />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-1 p-0.5 rounded-md bg-ink-100">
            <button onClick={() => setStatus("all")} className={tabClass("all")}>All</button>
            <button onClick={() => setStatus("active")} className={tabClass("active")}>Live</button>
            <button onClick={() => setStatus("inactive")} className={tabClass("inactive")}>Paused</button>
          </div>
          <input
            placeholder="Filter…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="h-8 px-3 rounded-md border border-ink-200 text-[12px] w-64 focus:outline-none"
          />
        </div>
        <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          <div className="grid grid-cols-[1fr_140px_120px_120px_80px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
            <div>Name</div><div>Status</div><div>Runs</div><div>Last</div><div className="text-right">Ver</div>
          </div>
          {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
          {!isLoading && filtered.length === 0 && (
            <div className="p-6 text-[12px] text-ink-500">No workflows match.</div>
          )}
          {filtered.map((w) => <WorkflowRow key={w.id} w={w} />)}
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 4: Smoke + commit**

```bash
pnpm dev
git add web/
git commit -m "feat(web): workflows list (read-only) with filters"
```

---

## Task 5: `useRunStream` hook (SSE + polling fallback)

**Files:**
- Create: `web/src/lib/hooks/use-run-stream.ts`
- Test: `web/src/lib/hooks/use-run-stream.test.ts`

- [ ] **Step 1: Implement**

```ts
"use client";
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { runStreamUrl, getRun } from "@/lib/api/runs";
import { RunLogEventSchema, type RunLogEvent } from "@/lib/api/schemas";

export interface UseRunStreamResult {
  events: RunLogEvent[];
  state: "idle" | "streaming" | "polling" | "closed" | "error";
  /** Reset and reopen the stream. */
  reset: () => void;
}

export function useRunStream(run_id: string | undefined): UseRunStreamResult {
  const [events, setEvents] = useState<RunLogEvent[]>([]);
  const [state, setState] = useState<UseRunStreamResult["state"]>("idle");
  const esRef = useRef<EventSource | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const qc = useQueryClient();

  const reset = () => {
    esRef.current?.close();
    if (pollRef.current) clearInterval(pollRef.current);
    setEvents([]);
    setState("idle");
  };

  useEffect(() => {
    if (!run_id) return;
    setEvents([]);
    setState("streaming");

    const url = runStreamUrl(run_id);
    const es = new EventSource(url);
    esRef.current = es;

    es.onmessage = (e) => {
      try {
        const evt = RunLogEventSchema.parse(JSON.parse(e.data));
        setEvents((prev) => [...prev, evt]);
        if (evt.event === "run_complete" || evt.event === "error") {
          es.close();
          setState("closed");
          qc.invalidateQueries({ queryKey: ["run", run_id] });
        }
      } catch { /* ignore malformed event */ }
    };

    es.onerror = () => {
      es.close();
      // Fallback: poll the run endpoint every 1s until terminal status
      setState("polling");
      pollRef.current = setInterval(async () => {
        try {
          const r = await getRun(run_id);
          if (["confirmed", "failed", "skipped"].includes(r.status)) {
            if (pollRef.current) clearInterval(pollRef.current);
            setState("closed");
            qc.invalidateQueries({ queryKey: ["run", run_id] });
          }
        } catch {
          setState("error");
          if (pollRef.current) clearInterval(pollRef.current);
        }
      }, 1000);
    };

    return () => {
      es.close();
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [run_id, qc]);

  return { events, state, reset };
}
```

- [ ] **Step 2: Unit test (EventSource mock)**

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useRunStream } from "./use-run-stream";

class MockEventSource {
  static instances: MockEventSource[] = [];
  url: string;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onerror: ((e: Event) => void) | null = null;
  closed = false;
  constructor(url: string) { this.url = url; MockEventSource.instances.push(this); }
  close() { this.closed = true; }
}

beforeEach(() => {
  MockEventSource.instances = [];
  vi.stubGlobal("EventSource", MockEventSource);
});

const wrapper = ({ children }: { children: React.ReactNode }) => {
  const qc = new QueryClient();
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
};

describe("useRunStream", () => {
  it("collects events", async () => {
    const { result } = renderHook(() => useRunStream("run-1"), { wrapper });
    const es = MockEventSource.instances[0]!;
    es.onmessage?.({ data: JSON.stringify({ event: "step_start", step_id: "s1", data: {}, timestamp: "2026-05-11T00:00:00Z" }) } as MessageEvent);
    await waitFor(() => expect(result.current.events).toHaveLength(1));
  });

  it("closes on run_complete", async () => {
    const { result } = renderHook(() => useRunStream("run-1"), { wrapper });
    const es = MockEventSource.instances[0]!;
    es.onmessage?.({ data: JSON.stringify({ event: "run_complete", data: {}, timestamp: "2026-05-11T00:00:00Z" }) } as MessageEvent);
    await waitFor(() => expect(result.current.state).toBe("closed"));
    expect(es.closed).toBe(true);
  });
});
```

- [ ] **Step 3: Run + commit**

```bash
pnpm test src/lib/hooks/use-run-stream.test.ts
git add web/
git commit -m "feat(web): useRunStream SSE hook with polling fallback"
```

---

## Task 6: Runs list

**Files:**
- Create: `web/src/app/(app)/runs/page.tsx`
- Create: `web/src/components/runs/RunRow.tsx`
- Create: `web/src/components/runs/RunStatusPill.tsx`

- [ ] **Step 1: `RunStatusPill.tsx`**

```tsx
import { Pill } from "@/components/primitives/Pill";
import type { RunStatus } from "@/lib/api/schemas";

const TONE: Record<RunStatus, "emerald" | "amber" | "rose" | "ink" | "violet"> = {
  pending: "ink",
  triggered: "ink",
  simulating: "ink",
  bundling: "violet",
  submitted: "violet",
  confirmed: "emerald",
  retrying: "amber",
  failed: "rose",
  skipped: "ink",
};

export function RunStatusPill({ status }: { status: RunStatus }) {
  return <Pill tone={TONE[status]}>{status}</Pill>;
}
```

- [ ] **Step 2: `RunRow.tsx`**

```tsx
import Link from "next/link";
import type { WorkflowRun } from "@/lib/api/schemas";
import { RunStatusPill } from "./RunStatusPill";
import { formatLamports, formatRelativeTime, formatSlot } from "@/lib/utils/format";

export function RunRow({ r }: { r: WorkflowRun }) {
  return (
    <Link
      href={`/runs/${r.run_id}`}
      className="grid grid-cols-[80px_1fr_120px_140px_140px_120px] items-center px-4 h-11 border-b border-ink-100 hover:bg-ink-50 text-[12px] font-mono"
    >
      <RunStatusPill status={r.status} />
      <div className="text-ink-900 truncate">{r.workflow_id}</div>
      <div className="text-ink-500">{formatRelativeTime(r.started_at)}</div>
      <div className="text-ink-500">{r.slot ? formatSlot(r.slot) : "—"}</div>
      <div className="text-ink-500">{r.jito_tip_lamports ? formatLamports(r.jito_tip_lamports) : "—"}</div>
      <div className="text-ink-400 text-right truncate">{r.signature ?? "—"}</div>
    </Link>
  );
}
```

- [ ] **Step 3: Page**

```tsx
"use client";
import { Topbar } from "@/components/shell/Topbar";
import { useRuns } from "@/lib/hooks/use-runs";
import { RunRow } from "@/components/runs/RunRow";

export default function RunsPage() {
  const { data, isLoading } = useRuns({ limit: 50 });
  return (
    <>
      <Topbar crumbs={["Workspace", "Operate", "Runs & Logs"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          <div className="grid grid-cols-[80px_1fr_120px_140px_140px_120px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
            <div>Status</div><div>Workflow</div><div>Started</div><div>Slot</div><div>Jito tip</div><div className="text-right">Signature</div>
          </div>
          {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
          {!isLoading && (data ?? []).length === 0 && (
            <div className="p-6 text-[12px] text-ink-500">No runs yet.</div>
          )}
          {(data ?? []).map((r) => <RunRow key={r.run_id} r={r} />)}
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): runs list with status, slot, tip"
```

---

## Task 7: Run detail page

**Files:**
- Create: `web/src/app/(app)/runs/[run_id]/page.tsx`
- Create: `web/src/components/runs/StepTimeline.tsx`
- Create: `web/src/components/runs/LiveLogStream.tsx`

- [ ] **Step 1: `StepTimeline.tsx`**

```tsx
import type { StepLog } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";

const TONE = { pending: "ink", running: "violet", success: "emerald", failed: "rose", skipped: "ink" } as const;

export function StepTimeline({ steps }: { steps: StepLog[] }) {
  if (steps.length === 0) {
    return <div className="text-[12px] text-ink-500 p-4">No steps recorded yet.</div>;
  }
  return (
    <ol className="space-y-2">
      {steps.map((s, i) => (
        <li key={s.step_id} className="flex items-start gap-3 rounded-lg border border-ink-200 bg-white p-3">
          <div className="w-5 h-5 rounded-full bg-ink-100 text-[10px] font-mono flex items-center justify-center text-ink-700">
            {i + 1}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <div className="text-[13px] font-medium">{s.step_id}</div>
              <Pill tone={TONE[s.status]}>{s.status}</Pill>
              <span className="text-[11px] text-ink-500 font-mono">{s.duration_ms} ms</span>
            </div>
            {s.error && (
              <pre className="mt-1 text-[11px] font-mono text-rose-700 whitespace-pre-wrap">{s.error}</pre>
            )}
          </div>
        </li>
      ))}
    </ol>
  );
}
```

- [ ] **Step 2: `LiveLogStream.tsx`**

```tsx
"use client";
import { useEffect, useRef } from "react";
import type { RunLogEvent } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";

export function LiveLogStream({ events, state }: {
  events: RunLogEvent[];
  state: "idle" | "streaming" | "polling" | "closed" | "error";
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => { ref.current?.scrollTo({ top: ref.current.scrollHeight }); }, [events.length]);

  return (
    <div className="rounded-xl border border-ink-200 bg-ink-950 text-ink-100 h-full flex flex-col">
      <div className="h-9 px-3 border-b border-ink-800 flex items-center justify-between">
        <div className="text-[11px] font-mono uppercase tracking-wider text-ink-400">Live log</div>
        <Pill tone={state === "streaming" ? "emerald" : state === "polling" ? "amber" : state === "closed" ? "ink" : "rose"}>
          {state}
        </Pill>
      </div>
      <div ref={ref} className="flex-1 overflow-y-auto p-3 font-mono text-[11px] leading-relaxed scrollbar-thin">
        {events.length === 0 && <div className="text-ink-500">Waiting for first event…</div>}
        {events.map((e, i) => (
          <div key={i} className="whitespace-pre-wrap">
            <span className="text-ink-500">{e.timestamp}</span>{"  "}
            <span className={
              e.event === "error" ? "text-rose-400"
              : e.event === "run_complete" ? "text-emerald-400"
              : "text-violet-300"
            }>{e.event}</span>
            {e.step_id && <span className="text-ink-300"> [{e.step_id}]</span>}
            {" "}
            <span className="text-ink-200">{JSON.stringify(e.data)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Page**

```tsx
"use client";
import { use } from "react";
import { Topbar } from "@/components/shell/Topbar";
import { useRun } from "@/lib/hooks/use-runs";
import { useRunStream } from "@/lib/hooks/use-run-stream";
import { StepTimeline } from "@/components/runs/StepTimeline";
import { LiveLogStream } from "@/components/runs/LiveLogStream";
import { RunStatusPill } from "@/components/runs/RunStatusPill";
import { formatLamports, formatSlot, solscanTx } from "@/lib/utils/format";

export default function RunDetail({ params }: { params: Promise<{ run_id: string }> }) {
  const { run_id } = use(params);
  const run = useRun(run_id);
  const stream = useRunStream(run_id);

  const network = (process.env.NEXT_PUBLIC_SOLANA_NETWORK as "mainnet" | "devnet") ?? "devnet";

  return (
    <>
      <Topbar crumbs={["Workspace", "Operate", "Run " + run_id.slice(0, 8)]} />
      <main className="flex-1 grid grid-cols-[1fr_480px] gap-4 p-6 overflow-hidden">
        <section className="overflow-y-auto pr-2">
          <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 mb-4">
            <div className="flex items-center gap-3 mb-2">
              <RunStatusPill status={run.data?.status ?? "pending"} />
              {run.data?.signature && (
                <a href={solscanTx(run.data.signature, network)} target="_blank" rel="noreferrer" className="text-[12px] font-mono text-violet-700 underline">
                  {run.data.signature.slice(0, 12)}…
                </a>
              )}
              {run.data?.slot && (
                <span className="text-[12px] font-mono text-ink-500">slot {formatSlot(run.data.slot)}</span>
              )}
              {run.data?.jito_tip_lamports && (
                <span className="text-[12px] font-mono text-ink-500">tip {formatLamports(run.data.jito_tip_lamports)}</span>
              )}
            </div>
            {run.data?.error && (
              <pre className="text-[11px] font-mono text-rose-700 whitespace-pre-wrap mt-2">{run.data.error}</pre>
            )}
          </div>
          <StepTimeline steps={run.data?.steps ?? []} />
        </section>
        <aside className="overflow-hidden">
          <LiveLogStream events={stream.events} state={stream.state} />
        </aside>
      </main>
    </>
  );
}
```

- [ ] **Step 4: Smoke + commit**

```bash
pnpm dev
git add web/
git commit -m "feat(web): run detail with step timeline + live log stream"
```

---

## Task 8: Marketplace list

**Files:**
- Create: `web/src/app/(app)/marketplace/page.tsx`
- Create: `web/src/components/marketplace/PlaybookCard.tsx`
- Create: `web/src/components/marketplace/ProtocolBadge.tsx`

- [ ] **Step 1: `ProtocolBadge.tsx`**

```tsx
import { Pill } from "@/components/primitives/Pill";

export function ProtocolBadge({ name }: { name: string }) {
  return <Pill tone="violet">{name}</Pill>;
}
```

- [ ] **Step 2: `PlaybookCard.tsx`**

```tsx
import Link from "next/link";
import type { HubWorkflow } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";
import { ProtocolBadge } from "./ProtocolBadge";
import { Icon } from "@/components/primitives/Icon";

export function PlaybookCard({ p }: { p: HubWorkflow }) {
  return (
    <Link
      href={`/marketplace/${p.id}`}
      className="rounded-xl border border-ink-200 bg-white shadow-card p-4 hover:shadow-pop transition-shadow flex flex-col"
    >
      <div className="flex items-start justify-between mb-2">
        <div>
          <div className="text-[14px] font-semibold tracking-tight">{p.name}</div>
          <div className="text-[11px] font-mono text-ink-500">{p.author}</div>
        </div>
        <div className="flex gap-1">
          {p.verified && <Pill tone="emerald"><Icon name="check" className="w-3 h-3" />verified</Pill>}
          {p.audited && <Pill tone="cyan"><Icon name="shield" className="w-3 h-3" />audited</Pill>}
        </div>
      </div>
      <p className="text-[12px] text-ink-600 line-clamp-2 mb-3">{p.description}</p>
      <div className="flex flex-wrap gap-1 mb-3">
        {p.protocols.map((proto) => <ProtocolBadge key={proto} name={proto} />)}
      </div>
      <div className="mt-auto grid grid-cols-3 text-[11px] font-mono">
        <div><div className="text-ink-400">Runs</div><div className="text-ink-900">{p.runs.toLocaleString()}</div></div>
        <div><div className="text-ink-400">Success</div><div className="text-ink-900">{p.success_rate}</div></div>
        <div><div className="text-ink-400">Fee</div><div className="text-ink-900">{p.fee_usdc}</div></div>
      </div>
    </Link>
  );
}
```

- [ ] **Step 3: Page**

```tsx
"use client";
import { Topbar } from "@/components/shell/Topbar";
import { useHub } from "@/lib/hooks/use-hub";
import { PlaybookCard } from "@/components/marketplace/PlaybookCard";

export default function MarketplacePage() {
  const { data, isLoading } = useHub();
  return (
    <>
      <Topbar crumbs={["Hub", "Marketplace"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        {isLoading && <div className="text-[12px] text-ink-500">Loading…</div>}
        {!isLoading && (data ?? []).length === 0 && (
          <div className="text-[12px] text-ink-500">No published workflows yet.</div>
        )}
        <div className="grid grid-cols-3 gap-3">
          {(data ?? []).map((p) => <PlaybookCard key={p.id} p={p} />)}
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): marketplace list with playbook cards"
```

---

## Task 9: Acceptance smoke pass

- [ ] **Step 1: Boot dev server with backend**

Backend must be running at `localhost:8080` with at least one org, API key, and ideally a workflow + run.

- [ ] **Step 2: Smoke checklist**

| Step | Expected |
|---|---|
| Visit `/`, no token | redirects to `/login` |
| Paste valid key, submit | lands on `/dashboard`; KPIs render |
| Click `Workflows` nav | list renders (empty state OK if backend has none) |
| Filter to `Live` tab | request includes `?status=active` (verify in DevTools Network) |
| Click `Runs & Logs` | list renders; polling visible every 5s |
| Open a run detail | step timeline + live log appear; if SSE fails, "polling" pill shows |
| Click `Marketplace` | playbook grid renders |

- [ ] **Step 3: Final commit if any small fixes**

```bash
pnpm typecheck && pnpm test && pnpm build
git add web/
git commit -m "fix(web): phase B smoke pass adjustments"  # only if needed
```

---

## Self-review checklist

- [ ] Spec §6 — Login, Dashboard, Workflows, Runs, Run detail, Marketplace all wired? Yes.
- [ ] Spec §7 — SSE with polling fallback? Yes (Task 5).
- [ ] Spec §10 — Solscan links + slot/lamport formatting on run detail? Yes.
- [ ] No `git add .` anywhere. ✓
- [ ] Mutations (trigger, enable/disable, delete) NOT implemented — deferred to Phase C, per spec phasing. ✓

---

## End-of-phase acceptance

Phase B is **done** when:
- All six screens render real data when endpoints exist.
- All six screens render graceful loading/empty states when endpoints return 404 or empty arrays.
- `pnpm typecheck && pnpm test && pnpm build` pass.
- The smoke checklist above passes manually.
