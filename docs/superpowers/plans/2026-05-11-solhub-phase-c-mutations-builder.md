# Solhub Frontend — Phase C: Mutations + Workflow Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Workflows become editable. Add Create/Update/Delete/Trigger mutations on the list, then build the React Flow canvas with a tool palette, Zod-driven inspector, auto-save, publish, and test-run.

**Architecture:** TanStack Query mutations with cache invalidation. React Flow with custom node types styled in Tailwind to match the prototype. Plugin registry is a static TS object mirroring `IDEA.md` §7.3 — each plugin/action carries a Zod schema used by the inspector to render typed forms.

**Tech Stack:** Adds `reactflow`, `react-hook-form`, `@hookform/resolvers`.

**Pre-requisite:** Phases A + B complete.

**Reference:** spec §6 (Builder row), §10. IDEA.md §4.2 (workflow types), §7 (plugin matrix).

**Commit policy:** `git add web/` only.

---

## Task 1: Install React Flow + form deps

- [ ] **Step 1**

```bash
cd web
pnpm add reactflow@11 react-hook-form @hookform/resolvers
```

- [ ] **Step 2: Commit**

```bash
git add web/package.json web/pnpm-lock.yaml
git commit -m "chore(web): add reactflow, react-hook-form"
```

---

## Task 2: Plugin registry

**Files:**
- Create: `web/src/lib/plugins/registry.ts`
- Create: `web/src/lib/plugins/types.ts`

Reference: `IDEA.md` §7.3 (plugin matrix).

- [ ] **Step 1: `types.ts`**

```ts
import { z } from "zod";

export type ActionType = "read" | "transaction" | "notification" | "logic";

export interface PluginAction {
  id: string;            // e.g. "swap"
  name: string;
  description: string;
  type: ActionType;
  /** Zod schema for the `params` object. */
  schema: z.ZodTypeAny;
  /** Default values used by the inspector form. */
  defaults: Record<string, unknown>;
}

export interface PluginDef {
  id: string;            // e.g. "jupiter"
  name: string;
  category: "swap" | "lend" | "stake" | "perps" | "oracle" | "lp" | "nft" | "notify" | "logic";
  actions: PluginAction[];
}
```

- [ ] **Step 2: `registry.ts`**

```ts
import { z } from "zod";
import type { PluginDef } from "./types";

export const REGISTRY: PluginDef[] = [
  {
    id: "jupiter", name: "Jupiter", category: "swap",
    actions: [{
      id: "swap", name: "Swap", description: "Best-route token swap.", type: "transaction",
      schema: z.object({
        input_mint: z.string().min(32),
        output_mint: z.string().min(32),
        amount: z.coerce.bigint().positive(),
        slippage_bps: z.coerce.number().int().min(0).max(10_000).default(50),
      }),
      defaults: { input_mint: "", output_mint: "", amount: "1000000", slippage_bps: 50 },
    }],
  },
  {
    id: "kamino", name: "Kamino", category: "lend",
    actions: [
      { id: "deposit", name: "Deposit", description: "Deposit collateral.", type: "transaction",
        schema: z.object({ market: z.string(), reserve: z.string(), amount: z.coerce.bigint().positive() }),
        defaults: { market: "", reserve: "", amount: "1000000" } },
      { id: "claim_rewards", name: "Claim rewards", description: "Harvest accrued rewards.", type: "transaction",
        schema: z.object({ market: z.string() }), defaults: { market: "" } },
      { id: "check_ltv", name: "Check LTV", description: "Read current loan-to-value.", type: "read",
        schema: z.object({ obligation: z.string() }), defaults: { obligation: "" } },
    ],
  },
  {
    id: "marinade", name: "Marinade", category: "stake",
    actions: [
      { id: "liquid_stake", name: "Liquid stake", description: "Stake SOL for mSOL.", type: "transaction",
        schema: z.object({ amount_lamports: z.coerce.bigint().positive() }),
        defaults: { amount_lamports: "1000000000" } },
      { id: "unstake", name: "Unstake", description: "Convert mSOL back to SOL.", type: "transaction",
        schema: z.object({ msol_amount: z.coerce.bigint().positive() }),
        defaults: { msol_amount: "1000000000" } },
    ],
  },
  {
    id: "drift", name: "Drift", category: "perps",
    actions: [
      { id: "open_position", name: "Open position", description: "Open a perp position.", type: "transaction",
        schema: z.object({
          market: z.string(),
          side: z.enum(["long", "short"]),
          base_amount: z.coerce.bigint().positive(),
        }),
        defaults: { market: "", side: "long", base_amount: "100000000" } },
      { id: "check_margin", name: "Check margin", description: "Read margin health.", type: "read",
        schema: z.object({ user_account: z.string() }), defaults: { user_account: "" } },
    ],
  },
  {
    id: "pyth", name: "Pyth", category: "oracle",
    actions: [{
      id: "read_price", name: "Read price", description: "Fetch a Pyth price feed.", type: "read",
      schema: z.object({ feed: z.string() }), defaults: { feed: "" },
    }],
  },
  {
    id: "notify.telegram", name: "Telegram", category: "notify",
    actions: [{
      id: "send_message", name: "Send message", description: "Send a Telegram message.", type: "notification",
      schema: z.object({ chat_id: z.string(), text: z.string().min(1) }),
      defaults: { chat_id: "", text: "" },
    }],
  },
  {
    id: "notify.discord", name: "Discord", category: "notify",
    actions: [{
      id: "send_message", name: "Send message", description: "Send a Discord message.", type: "notification",
      schema: z.object({ webhook_url: z.string().url(), content: z.string().min(1) }),
      defaults: { webhook_url: "", content: "" },
    }],
  },
  {
    id: "system", name: "System", category: "logic",
    actions: [
      { id: "condition", name: "Condition", description: "Branch on an expression.", type: "logic",
        schema: z.object({ expression: z.string().min(1) }), defaults: { expression: "" } },
      { id: "http_request", name: "HTTP request", description: "Outbound HTTP call.", type: "logic",
        schema: z.object({
          url: z.string().url(),
          method: z.enum(["GET", "POST", "PUT", "DELETE"]).default("GET"),
          body: z.string().optional(),
        }), defaults: { url: "", method: "GET", body: "" } },
    ],
  },
];

export function findAction(pluginId: string, actionId: string) {
  const p = REGISTRY.find((x) => x.id === pluginId);
  if (!p) return null;
  const a = p.actions.find((x) => x.id === actionId);
  return a ? { plugin: p, action: a } : null;
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): static plugin registry mirroring IDEA.md §7.3"
```

---

## Task 3: Workflow mutations

**Files:**
- Create: `web/src/lib/hooks/use-workflow-mutations.ts`
- Test: `web/src/lib/hooks/use-workflow-mutations.test.tsx`

- [ ] **Step 1: Implement**

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { workflows } from "@/lib/api";
import type { CreateWorkflowBody } from "@/lib/api/workflows";

export function useCreateWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateWorkflowBody) => workflows.createWorkflow(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
}

export function useUpdateWorkflow(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: Partial<CreateWorkflowBody> & { is_active?: boolean }) =>
      workflows.updateWorkflow(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["workflows"] });
      qc.invalidateQueries({ queryKey: ["workflow", id] });
    },
  });
}

export function useDeleteWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => workflows.deleteWorkflow(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
}

export function useTriggerWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; overrides?: Record<string, unknown> }) =>
      workflows.triggerWorkflow(vars.id, vars.overrides),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["runs"] }),
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/
git commit -m "feat(web): workflow mutation hooks"
```

---

## Task 4: Add row-level actions to Workflows list

**Files:**
- Modify: `web/src/components/workflow/WorkflowRow.tsx`
- Modify: `web/src/app/(app)/workflows/page.tsx`

- [ ] **Step 1: Replace `WorkflowRow.tsx` with action menu**

```tsx
"use client";
import Link from "next/link";
import { useState } from "react";
import type { Workflow } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";
import { StatusPill } from "./StatusPill";
import { Icon } from "@/components/primitives/Icon";
import { useUpdateWorkflow, useDeleteWorkflow, useTriggerWorkflow } from "@/lib/hooks/use-workflow-mutations";
import { useRouter } from "next/navigation";

export function WorkflowRow({ w }: { w: Workflow }) {
  const upd = useUpdateWorkflow(w.id);
  const del = useDeleteWorkflow();
  const trig = useTriggerWorkflow();
  const router = useRouter();
  const [busy, setBusy] = useState(false);

  async function onTrigger(e: React.MouseEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      const r = await trig.mutateAsync({ id: w.id });
      router.push(`/runs/${r.run_id}`);
    } finally { setBusy(false); }
  }

  async function onToggle(e: React.MouseEvent) {
    e.preventDefault();
    await upd.mutateAsync({ is_active: !w.is_active });
  }

  async function onDelete(e: React.MouseEvent) {
    e.preventDefault();
    if (!confirm(`Delete workflow "${w.name}"?`)) return;
    await del.mutateAsync(w.id);
  }

  return (
    <div className="grid grid-cols-[1fr_140px_120px_120px_160px] items-center px-4 h-12 border-b border-ink-100 hover:bg-ink-50 text-[13px]">
      <Link href={`/workflows/${w.id}`} className="flex items-center gap-2 min-w-0">
        <div className="font-medium text-ink-900 truncate">{w.name}</div>
        <Pill tone="ink">{w.trigger.type}</Pill>
      </Link>
      <StatusPill active={w.is_active} />
      <div className="text-ink-500 font-mono">{w.execution_count.toString()} runs</div>
      <div className="text-ink-500">—</div>
      <div className="flex justify-end gap-1">
        <button onClick={onTrigger} disabled={busy} title="Trigger" className="w-7 h-7 rounded hover:bg-ink-100 inline-flex items-center justify-center">
          <Icon name="play" className="w-3.5 h-3.5 text-emerald-600" />
        </button>
        <button onClick={onToggle} title={w.is_active ? "Pause" : "Resume"} className="w-7 h-7 rounded hover:bg-ink-100 inline-flex items-center justify-center">
          <Icon name={w.is_active ? "pause" : "play"} className="w-3.5 h-3.5 text-ink-700" />
        </button>
        <button onClick={onDelete} title="Delete" className="w-7 h-7 rounded hover:bg-rose-50 inline-flex items-center justify-center">
          <Icon name="trash" className="w-3.5 h-3.5 text-rose-600" />
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Update the grid header in `workflows/page.tsx`** to match the new last column width (`120px → 160px`).

Find:
```tsx
<div className="grid grid-cols-[1fr_140px_120px_120px_80px] ...">
  <div>Name</div><div>Status</div><div>Runs</div><div>Last</div><div className="text-right">Ver</div>
```

Replace with:
```tsx
<div className="grid grid-cols-[1fr_140px_120px_120px_160px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
  <div>Name</div><div>Status</div><div>Runs</div><div>Last</div><div className="text-right">Actions</div>
```

- [ ] **Step 3: Smoke + commit**

```bash
pnpm dev
git add web/
git commit -m "feat(web): row actions on workflows list (trigger, toggle, delete)"
```

---

## Task 5: Builder route shell

**Files:**
- Create: `web/src/app/(app)/workflows/[id]/page.tsx`
- Create: `web/src/components/workflow/builder/BuilderShell.tsx`

The builder route uses a custom header (no Topbar) per the prototype's `app.jsx` `route !== "builder"` check. Implement that here.

- [ ] **Step 1: Page**

```tsx
"use client";
import { use } from "react";
import { BuilderShell } from "@/components/workflow/builder/BuilderShell";

export default function BuilderPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <BuilderShell id={id} />;
}
```

- [ ] **Step 2: `BuilderShell.tsx` — minimal layout (canvas added next task)**

```tsx
"use client";
import { useState } from "react";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";

export function BuilderShell({ id }: { id: string }) {
  const isNew = id === "new";
  const { data } = useWorkflow(isNew ? undefined : id);
  const [name, setName] = useState(data?.name ?? "Untitled workflow");

  return (
    <div className="h-screen flex flex-col">
      <header className="h-12 border-b border-ink-200 bg-white flex items-center justify-between px-4">
        <div className="flex items-center gap-3">
          <SolhubLogo />
          <span className="text-ink-300">/</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="text-[13px] font-medium bg-transparent focus:outline-none px-2 py-1 hover:bg-ink-50 rounded"
          />
          {data?.is_active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">draft</Pill>}
        </div>
        <div className="flex items-center gap-2">
          <Btn variant="default" size="sm">Test run</Btn>
          <Btn variant="primary" size="sm">Save</Btn>
          <Btn variant="success" size="sm">Publish</Btn>
        </div>
      </header>
      <div className="flex-1 grid grid-cols-[240px_1fr_320px]">
        <aside className="border-r border-ink-200 bg-white">{/* ToolPalette in Task 7 */}</aside>
        <main className="bg-ink-50">{/* Canvas in Task 6 */}</main>
        <aside className="border-l border-ink-200 bg-white">{/* Inspector in Task 8 */}</aside>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): builder route shell (header + three-column layout)"
```

---

## Task 6: React Flow canvas

**Files:**
- Modify: `web/src/components/workflow/builder/BuilderShell.tsx`
- Create: `web/src/components/workflow/builder/Canvas.tsx`
- Create: `web/src/components/workflow/builder/StepNode.tsx`
- Modify: `web/src/app/globals.css` (React Flow base styles)

- [ ] **Step 1: Import RF styles**

Add to top of `web/src/app/globals.css` (before `@tailwind` directives):
```css
@import "reactflow/dist/style.css";
```

- [ ] **Step 2: `StepNode.tsx`**

```tsx
import { Handle, Position, type NodeProps } from "reactflow";
import { Pill } from "@/components/primitives/Pill";

export interface StepNodeData {
  label: string;
  plugin: string;
  action: string;
  selected?: boolean;
}

export function StepNode({ data, selected }: NodeProps<StepNodeData>) {
  return (
    <div className={
      "rounded-lg border bg-white shadow-card px-3 py-2 min-w-[180px] " +
      (selected ? "border-violet-500 ring-2 ring-violet-500/30" : "border-ink-200")
    }>
      <Handle type="target" position={Position.Left} className="!bg-ink-400 !w-2 !h-2" />
      <div className="flex items-center gap-2">
        <div className="text-[12px] font-medium">{data.label}</div>
      </div>
      <div className="flex items-center gap-1 mt-1">
        <Pill tone="violet">{data.plugin}</Pill>
        <Pill tone="ink">{data.action}</Pill>
      </div>
      <Handle type="source" position={Position.Right} className="!bg-ink-400 !w-2 !h-2" />
    </div>
  );
}
```

- [ ] **Step 3: `Canvas.tsx`**

```tsx
"use client";
import {
  ReactFlow, Controls, MiniMap, Background, BackgroundVariant,
  useNodesState, useEdgesState, addEdge,
  type Connection, type Edge, type Node, type NodeTypes,
} from "reactflow";
import { useCallback, useMemo } from "react";
import { StepNode, type StepNodeData } from "./StepNode";

export interface CanvasProps {
  initialNodes: Node<StepNodeData>[];
  initialEdges: Edge[];
  onChange: (nodes: Node<StepNodeData>[], edges: Edge[]) => void;
  onSelect: (nodeId: string | null) => void;
}

export function Canvas({ initialNodes, initialEdges, onChange, onSelect }: CanvasProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState<StepNodeData>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const nodeTypes: NodeTypes = useMemo(() => ({ step: StepNode }), []);

  const onConnect = useCallback(
    (c: Connection) => setEdges((eds) => addEdge({ ...c, animated: true }, eds)),
    [setEdges],
  );

  function emit() { onChange(nodes, edges); }

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={(c) => { onNodesChange(c); emit(); }}
      onEdgesChange={(c) => { onEdgesChange(c); emit(); }}
      onConnect={onConnect}
      onSelectionChange={(p) => onSelect(p.nodes[0]?.id ?? null)}
      fitView
      defaultEdgeOptions={{ animated: true, style: { stroke: "#71717a", strokeWidth: 1.5 } }}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="rgba(24,24,27,0.18)" />
      <MiniMap pannable zoomable className="!bg-white !border !border-ink-200" />
      <Controls className="!shadow-card !border !border-ink-200" />
    </ReactFlow>
  );
}
```

- [ ] **Step 4: Wire into `BuilderShell.tsx`**

Replace the `<main>` placeholder with the canvas. Add state for nodes/edges/selected:

```tsx
"use client";
import { useEffect, useState } from "react";
import type { Node, Edge } from "reactflow";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { Canvas } from "./Canvas";
import type { StepNodeData } from "./StepNode";
import type { WorkflowStep } from "@/lib/api/schemas";

function stepsToGraph(steps: WorkflowStep[]): { nodes: Node<StepNodeData>[]; edges: Edge[] } {
  const nodes: Node<StepNodeData>[] = steps.map((s, i) => ({
    id: s.id,
    type: "step",
    position: { x: 80 + i * 240, y: 160 },
    data: { label: `${s.plugin}.${s.action}`, plugin: s.plugin, action: s.action },
  }));
  const edges: Edge[] = steps.slice(1).map((s, i) => ({
    id: `${steps[i]!.id}->${s.id}`,
    source: steps[i]!.id,
    target: s.id,
    animated: true,
  }));
  return { nodes, edges };
}

export function BuilderShell({ id }: { id: string }) {
  const isNew = id === "new";
  const { data } = useWorkflow(isNew ? undefined : id);
  const [name, setName] = useState("Untitled workflow");
  const [nodes, setNodes] = useState<Node<StepNodeData>[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    if (data) {
      setName(data.name);
      const g = stepsToGraph(data.steps);
      setNodes(g.nodes);
      setEdges(g.edges);
    }
  }, [data]);

  return (
    <div className="h-screen flex flex-col">
      <header className="h-12 border-b border-ink-200 bg-white flex items-center justify-between px-4">
        <div className="flex items-center gap-3">
          <SolhubLogo />
          <span className="text-ink-300">/</span>
          <input value={name} onChange={(e) => setName(e.target.value)} className="text-[13px] font-medium bg-transparent focus:outline-none px-2 py-1 hover:bg-ink-50 rounded"/>
          {data?.is_active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">draft</Pill>}
        </div>
        <div className="flex items-center gap-2">
          <Btn variant="default" size="sm">Test run</Btn>
          <Btn variant="primary" size="sm">Save</Btn>
          <Btn variant="success" size="sm">Publish</Btn>
        </div>
      </header>
      <div className="flex-1 grid grid-cols-[240px_1fr_320px]">
        <aside className="border-r border-ink-200 bg-white">{/* ToolPalette next task */}</aside>
        <main className="bg-ink-50">
          <Canvas
            initialNodes={nodes}
            initialEdges={edges}
            onChange={(n, e) => { setNodes(n); setEdges(e); }}
            onSelect={setSelectedId}
          />
        </main>
        <aside className="border-l border-ink-200 bg-white">
          {/* Inspector next task; show selectedId for now */}
          <div className="p-3 text-[11px] font-mono text-ink-500">selected: {selectedId ?? "none"}</div>
        </aside>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Smoke + commit**

```bash
pnpm dev
# /workflows/<some-id-with-steps> shows nodes + edges
git add web/
git commit -m "feat(web): React Flow canvas with custom step nodes"
```

---

## Task 7: Tool palette

**Files:**
- Create: `web/src/components/workflow/builder/ToolPalette.tsx`
- Modify: `web/src/components/workflow/builder/BuilderShell.tsx`

- [ ] **Step 1: `ToolPalette.tsx`**

```tsx
"use client";
import { useState } from "react";
import { REGISTRY } from "@/lib/plugins/registry";
import { Icon } from "@/components/primitives/Icon";

export function ToolPalette({ onAdd }: { onAdd: (plugin: string, action: string) => void }) {
  const [q, setQ] = useState("");
  const filtered = REGISTRY
    .map((p) => ({ ...p, actions: p.actions.filter((a) =>
      `${p.name} ${p.id} ${a.name} ${a.id}`.toLowerCase().includes(q.toLowerCase()),
    )}))
    .filter((p) => p.actions.length > 0);

  return (
    <div className="h-full flex flex-col">
      <div className="h-10 px-3 border-b border-ink-200 flex items-center gap-2">
        <Icon name="search" className="w-3.5 h-3.5 text-ink-400" />
        <input
          placeholder="Search tools…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          className="flex-1 text-[12px] bg-transparent focus:outline-none"
        />
      </div>
      <div className="flex-1 overflow-y-auto scrollbar-thin p-2 space-y-3">
        {filtered.map((p) => (
          <div key={p.id}>
            <div className="text-[10px] uppercase tracking-wider font-mono text-ink-500 px-2 mb-1">{p.name}</div>
            <ul>
              {p.actions.map((a) => (
                <li key={a.id}>
                  <button
                    onClick={() => onAdd(p.id, a.id)}
                    className="w-full text-left px-2 py-1.5 rounded hover:bg-ink-50 text-[12px]"
                  >
                    <div className="font-medium text-ink-900">{a.name}</div>
                    <div className="text-[11px] text-ink-500 line-clamp-1">{a.description}</div>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Wire `onAdd` in `BuilderShell.tsx`**

Add helper inside `BuilderShell` and pass to ToolPalette:

```tsx
function addStep(plugin: string, action: string) {
  const id = `step_${Math.random().toString(36).slice(2, 8)}`;
  setNodes((prev) => [
    ...prev,
    {
      id, type: "step",
      position: { x: 80 + prev.length * 240, y: 160 },
      data: { label: `${plugin}.${action}`, plugin, action },
    },
  ]);
}
```

Replace `<aside className="border-r ...">{/* ToolPalette */}</aside>` with:
```tsx
<aside className="border-r border-ink-200 bg-white">
  <ToolPalette onAdd={addStep} />
</aside>
```

(Add `import { ToolPalette } from "./ToolPalette";` at top.)

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): tool palette with searchable plugin actions"
```

---

## Task 8: Inspector (Zod-driven form per node)

**Files:**
- Create: `web/src/components/workflow/builder/Inspector.tsx`
- Create: `web/src/components/workflow/builder/ZodForm.tsx`
- Modify: `web/src/components/workflow/builder/BuilderShell.tsx`

- [ ] **Step 1: `ZodForm.tsx` — renders inputs from a ZodObject**

```tsx
"use client";
import { z, type ZodTypeAny } from "zod";

function fieldType(schema: ZodTypeAny): "string" | "number" | "bigint" | "boolean" | "enum" {
  if (schema instanceof z.ZodEnum) return "enum";
  if (schema instanceof z.ZodNumber) return "number";
  if (schema instanceof z.ZodBigInt) return "bigint";
  if (schema instanceof z.ZodBoolean) return "boolean";
  return "string";
}

function unwrap(schema: ZodTypeAny): ZodTypeAny {
  if (schema instanceof z.ZodDefault) return unwrap(schema._def.innerType);
  if (schema instanceof z.ZodOptional) return unwrap(schema._def.innerType);
  if (schema instanceof z.ZodEffects) return unwrap(schema._def.schema);
  return schema;
}

export function ZodForm({
  schema, value, onChange,
}: {
  schema: z.ZodObject<z.ZodRawShape>;
  value: Record<string, unknown>;
  onChange: (v: Record<string, unknown>) => void;
}) {
  const shape = schema.shape;
  return (
    <div className="space-y-3">
      {Object.entries(shape).map(([key, raw]) => {
        const inner = unwrap(raw as ZodTypeAny);
        const t = fieldType(inner);
        const current = value[key] as unknown;

        if (t === "enum") {
          const options = (inner as z.ZodEnum<[string, ...string[]]>).options;
          return (
            <label key={key} className="block">
              <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider">{key}</span>
              <select
                value={(current as string) ?? options[0]}
                onChange={(e) => onChange({ ...value, [key]: e.target.value })}
                className="mt-1 w-full h-8 px-2 rounded-md border border-ink-200 text-[13px]"
              >
                {options.map((opt: string) => <option key={opt} value={opt}>{opt}</option>)}
              </select>
            </label>
          );
        }
        if (t === "boolean") {
          return (
            <label key={key} className="flex items-center gap-2">
              <input type="checkbox" checked={!!current} onChange={(e) => onChange({ ...value, [key]: e.target.checked })} />
              <span className="text-[12px]">{key}</span>
            </label>
          );
        }
        const inputType = t === "number" || t === "bigint" ? "text" : "text";
        return (
          <label key={key} className="block">
            <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider">{key}</span>
            <input
              type={inputType}
              value={current == null ? "" : String(current)}
              onChange={(e) => onChange({ ...value, [key]: e.target.value })}
              className="mt-1 w-full h-8 px-2 rounded-md border border-ink-200 text-[13px] font-mono"
            />
          </label>
        );
      })}
    </div>
  );
}
```

Note: We deliberately keep numeric/bigint inputs as text and let the Zod `coerce` handle conversion on submit. This sidesteps a class of HTML number-input bugs.

- [ ] **Step 2: `Inspector.tsx`**

```tsx
"use client";
import type { Node } from "reactflow";
import type { StepNodeData } from "./StepNode";
import { findAction } from "@/lib/plugins/registry";
import { Pill } from "@/components/primitives/Pill";
import { ZodForm } from "./ZodForm";

export function Inspector({
  node, params, onParamsChange, onDelete,
}: {
  node: Node<StepNodeData> | null;
  params: Record<string, unknown>;
  onParamsChange: (v: Record<string, unknown>) => void;
  onDelete: () => void;
}) {
  if (!node) {
    return <div className="p-4 text-[12px] text-ink-500">Select a step to inspect.</div>;
  }
  const found = findAction(node.data.plugin, node.data.action);
  if (!found) {
    return <div className="p-4 text-[12px] text-rose-600">Unknown plugin/action: {node.data.plugin}.{node.data.action}</div>;
  }
  return (
    <div className="p-4 space-y-4">
      <div>
        <Pill tone="violet">{found.plugin.name}</Pill>{" "}
        <Pill tone="ink">{found.action.name}</Pill>
        <h2 className="text-[15px] font-semibold tracking-tight mt-2">{found.action.name}</h2>
        <p className="text-[12px] text-ink-500">{found.action.description}</p>
      </div>
      <ZodForm
        schema={found.action.schema as Parameters<typeof ZodForm>[0]["schema"]}
        value={params}
        onChange={onParamsChange}
      />
      <button onClick={onDelete} className="text-[12px] text-rose-600 hover:underline">Delete step</button>
    </div>
  );
}
```

- [ ] **Step 3: Wire into `BuilderShell.tsx`**

Track per-step params in state and pass through:

Add inside `BuilderShell`:
```tsx
const [params, setParams] = useState<Record<string, Record<string, unknown>>>({});
const selectedNode = nodes.find((n) => n.id === selectedId) ?? null;

function setSelectedParams(v: Record<string, unknown>) {
  if (!selectedId) return;
  setParams((p) => ({ ...p, [selectedId]: v }));
}
function deleteSelected() {
  if (!selectedId) return;
  setNodes((n) => n.filter((x) => x.id !== selectedId));
  setEdges((e) => e.filter((x) => x.source !== selectedId && x.target !== selectedId));
  setParams((p) => { const { [selectedId]: _, ...rest } = p; return rest; });
  setSelectedId(null);
}
```

Replace the right `<aside>` placeholder with:
```tsx
<aside className="border-l border-ink-200 bg-white overflow-y-auto">
  <Inspector
    node={selectedNode}
    params={params[selectedId ?? ""] ?? (selectedNode ? (findAction(selectedNode.data.plugin, selectedNode.data.action)?.action.defaults ?? {}) : {})}
    onParamsChange={setSelectedParams}
    onDelete={deleteSelected}
  />
</aside>
```

Add `import { Inspector } from "./Inspector";` and `import { findAction } from "@/lib/plugins/registry";`.

When adding a step (in `addStep`), seed its params:
```tsx
function addStep(plugin: string, action: string) {
  const id = `step_${Math.random().toString(36).slice(2, 8)}`;
  const found = findAction(plugin, action);
  setNodes((prev) => [
    ...prev,
    { id, type: "step", position: { x: 80 + prev.length * 240, y: 160 },
      data: { label: `${plugin}.${action}`, plugin, action } },
  ]);
  setParams((p) => ({ ...p, [id]: { ...(found?.action.defaults ?? {}) } }));
}
```

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): Zod-driven inspector for selected step"
```

---

## Task 9: Auto-save draft to localStorage

**Files:**
- Create: `web/src/lib/hooks/use-draft.ts`
- Modify: `web/src/components/workflow/builder/BuilderShell.tsx`

- [ ] **Step 1: Hook**

```ts
"use client";
import { useEffect, useState } from "react";

export interface DraftState {
  name: string;
  nodes: unknown[];
  edges: unknown[];
  params: Record<string, Record<string, unknown>>;
  updatedAt: string;
}

export function useDraft(id: string) {
  const key = `solhub.draft.${id}`;
  const [draft, setDraft] = useState<DraftState | null>(null);

  useEffect(() => {
    const raw = window.localStorage.getItem(key);
    if (raw) {
      try { setDraft(JSON.parse(raw)); } catch { /* ignore */ }
    }
  }, [key]);

  function save(d: Omit<DraftState, "updatedAt">) {
    const next = { ...d, updatedAt: new Date().toISOString() };
    window.localStorage.setItem(key, JSON.stringify(next));
    setDraft(next);
  }
  function clear() {
    window.localStorage.removeItem(key);
    setDraft(null);
  }
  return { draft, save, clear };
}
```

- [ ] **Step 2: Wire into `BuilderShell.tsx`**

Add:
```tsx
const { draft, save: saveDraft, clear: clearDraft } = useDraft(id);
useEffect(() => {
  if (!id) return;
  const t = setTimeout(() => saveDraft({ name, nodes, edges, params }), 500);
  return () => clearTimeout(t);
}, [id, name, nodes, edges, params, saveDraft]);

useEffect(() => {
  if (draft && nodes.length === 0 && !data) {
    setName(draft.name);
    setNodes(draft.nodes as Node<StepNodeData>[]);
    setEdges(draft.edges as Edge[]);
    setParams(draft.params);
  }
  // eslint-disable-next-line react-hooks/exhaustive-deps
}, [draft, data]);
```

(Hydrate from draft only when there's no server-side data and the canvas is empty.)

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): builder auto-saves draft to localStorage"
```

---

## Task 10: Save / Publish / Test-run actions

**Files:**
- Modify: `web/src/components/workflow/builder/BuilderShell.tsx`

- [ ] **Step 1: Convert canvas state → API body**

Add helpers at top of `BuilderShell.tsx`:

```tsx
function graphToSteps(
  nodes: Node<StepNodeData>[],
  edges: Edge[],
  params: Record<string, Record<string, unknown>>,
): WorkflowStep[] {
  // Topologically order nodes by edge graph; fall back to insertion order.
  const indeg = new Map(nodes.map((n) => [n.id, 0]));
  for (const e of edges) indeg.set(e.target, (indeg.get(e.target) ?? 0) + 1);
  const order: string[] = [];
  const queue = nodes.filter((n) => indeg.get(n.id) === 0).map((n) => n.id);
  while (queue.length) {
    const id = queue.shift()!;
    order.push(id);
    for (const e of edges.filter((x) => x.source === id)) {
      const nd = (indeg.get(e.target) ?? 1) - 1;
      indeg.set(e.target, nd);
      if (nd === 0) queue.push(e.target);
    }
  }
  // Any leftover (cycle or disconnected) appended in order.
  for (const n of nodes) if (!order.includes(n.id)) order.push(n.id);

  return order.map((id) => {
    const n = nodes.find((x) => x.id === id)!;
    return {
      id,
      plugin: n.data.plugin,
      action: n.data.action,
      params: params[id] ?? {},
      condition: null,
      on_error: { kind: "abort" },
    };
  });
}
```

(Add `import type { WorkflowStep } from "@/lib/api/schemas";` if not already present.)

- [ ] **Step 2: Mutations**

```tsx
import { useCreateWorkflow, useUpdateWorkflow, useTriggerWorkflow } from "@/lib/hooks/use-workflow-mutations";

// inside BuilderShell:
const create = useCreateWorkflow();
const update = useUpdateWorkflow(isNew ? "" : id);
const trigger = useTriggerWorkflow();
const router = useRouter();

async function save() {
  const body = {
    name,
    trigger: data?.trigger ?? { type: "cron" as const, schedule: "0 * * * *" },
    steps: graphToSteps(nodes, edges, params),
  };
  if (isNew) {
    const r = await create.mutateAsync(body);
    clearDraft();
    router.replace(`/workflows/${r.workflow_id}`);
  } else {
    await update.mutateAsync(body);
    clearDraft();
  }
}
async function publish() {
  if (isNew) await save();
  await update.mutateAsync({ is_active: true });
}
async function testRun() {
  if (isNew) return;
  const r = await trigger.mutateAsync({ id });
  router.push(`/runs/${r.run_id}`);
}
```

(Add `import { useRouter } from "next/navigation";`)

- [ ] **Step 3: Wire buttons in the header**

Replace the three header buttons with:
```tsx
<Btn variant="default" size="sm" onClick={testRun} disabled={isNew}>Test run</Btn>
<Btn variant="primary" size="sm" onClick={save}>Save</Btn>
<Btn variant="success" size="sm" onClick={publish}>Publish</Btn>
```

- [ ] **Step 4: Smoke + commit**

```bash
pnpm dev
# /workflows/new → palette → add step → save → URL updates to new id
# /workflows/<id> → publish → row in list shows live
git add web/
git commit -m "feat(web): builder save/publish/test-run actions"
```

---

## Task 11: Acceptance smoke pass

- [ ] **Step 1: Manual checklist**

| Step | Expected |
|---|---|
| `/workflows` → click trigger icon | navigates to `/runs/<new>` |
| `/workflows` → click pause icon | row flips to "paused" |
| `/workflows` → click trash icon | confirm dialog; on confirm row disappears |
| `/workflows/new` | empty canvas, palette + inspector empty state |
| Add Jupiter.swap from palette | node appears at right of last node |
| Select node | inspector shows form with `input_mint`, `output_mint`, `amount`, `slippage_bps` |
| Connect two nodes | edge animates |
| Click Save | URL updates to `/workflows/<uuid>`, draft cleared |
| Reload | canvas hydrates from server |
| Click Publish | list shows row as live |
| Click Test run on saved workflow | lands on `/runs/<run_id>` |

- [ ] **Step 2: typecheck + test + build**

```bash
pnpm typecheck && pnpm test && pnpm build
```

- [ ] **Step 3: Final commit if needed**

```bash
git add web/
git commit -m "fix(web): phase C smoke pass adjustments"  # only if needed
```

---

## Self-review checklist

- [ ] Spec §6 — Builder row covered: GET workflow, POST create, PATCH update, POST trigger. ✓
- [ ] Plugin registry mirrors IDEA.md §7.3 P0 plugins (Jupiter, Kamino, Marinade, Drift, Pyth, Telegram, Discord, system). Orca/Raydium/Sendgrid deferred to Phase E. ✓
- [ ] Auto-save uses localStorage keyed by id; cleared on save. ✓
- [ ] No `git add .` anywhere. ✓
- [ ] Types: `WorkflowStep.condition` and `on_error` match the Zod schema in Phase A — `condition: null`, `on_error: { kind: "abort" }`. ✓

---

## End-of-phase acceptance

Phase C is **done** when the smoke checklist passes and `pnpm typecheck && pnpm test && pnpm build` are clean.
