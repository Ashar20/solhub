"use client";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  addEdge, applyEdgeChanges, applyNodeChanges,
  type Connection, type Edge, type EdgeChange, type Node, type NodeChange,
} from "reactflow";
import { useWorkflow, useWorkflows } from "@/lib/hooks/use-workflows";
import { useDraft } from "@/lib/hooks/use-draft";
import { findAction } from "@/lib/plugins/registry";
import { EXAMPLES, exampleToGraph, type WorkflowExample } from "@/lib/plugins/examples";
import {
  useCreateWorkflow,
  useUpdateWorkflow,
  useTriggerWorkflow,
} from "@/lib/hooks/use-workflow-mutations";
import { ApiError } from "@/lib/api/client";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { Canvas } from "./Canvas";
import { Inspector } from "./Inspector";
import { ToolPalette } from "./ToolPalette";
import type { StepNodeData } from "./StepNode";

export interface BuilderShellProps { id: string }

// Topologically order nodes by edge graph; preserve insertion order as fallback.
function graphToSteps(
  nodes: Node<StepNodeData>[],
  edges: Edge[],
  params: Record<string, Record<string, unknown>>,
): Array<{ id: string; plugin: string; action: string; params: Record<string, unknown>; condition: null; on_error: { kind: "abort" } }> {
  const indeg = new Map<string, number>();
  for (const n of nodes) indeg.set(n.id, 0);
  for (const e of edges) indeg.set(e.target, (indeg.get(e.target) ?? 0) + 1);

  const order: string[] = [];
  const queue: string[] = nodes.filter((n) => indeg.get(n.id) === 0).map((n) => n.id);
  while (queue.length) {
    const id = queue.shift()!;
    order.push(id);
    for (const e of edges.filter((x) => x.source === id)) {
      const nd = (indeg.get(e.target) ?? 1) - 1;
      indeg.set(e.target, nd);
      if (nd === 0) queue.push(e.target);
    }
  }
  // Append any disconnected/cycle nodes preserving insertion order.
  for (const n of nodes) if (!order.includes(n.id)) order.push(n.id);

  return order.map((id) => {
    const n = nodes.find((x) => x.id === id)!;
    return {
      id,
      plugin: n.data.plugin,
      action: n.data.action,
      params: params[id] ?? {},
      condition: null,
      on_error: { kind: "abort" as const },
    };
  });
}

// Convert backend WorkflowStep[] → React Flow nodes + edges (linear left-to-right).
// Read schemas.ts for the actual WorkflowStep shape; minimum fields used: id, plugin, action.
type ApiStep = { id: string; plugin: string; action: string; params?: unknown };

function stepsToGraph(steps: ApiStep[]): { nodes: Node<StepNodeData>[]; edges: Edge[] } {
  const nodes: Node<StepNodeData>[] = steps.map((s, i) => ({
    id: s.id,
    type: "step",
    position: { x: 80 + i * 240, y: 160 },
    data: { label: `${s.plugin}.${s.action}`, plugin: s.plugin, action: s.action },
  }));
  const edges: Edge[] = [];
  for (let i = 1; i < steps.length; i++) {
    edges.push({
      id: `${steps[i - 1]!.id}->${steps[i]!.id}`,
      source: steps[i - 1]!.id,
      target: steps[i]!.id,
      animated: true,
    });
  }
  return { nodes, edges };
}

export function BuilderShell({ id }: BuilderShellProps) {
  const isNew = id === "new";
  const { data } = useWorkflow(isNew ? undefined : id);
  const [name, setName] = useState("Untitled workflow");
  const [nodes, setNodes] = useState<Node<StepNodeData>[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [params, setParams] = useState<Record<string, Record<string, unknown>>>({});

  const { draft, save: saveDraft, clear: clearDraft } = useDraft(id);
  const [hydratedFromDraft, setHydratedFromDraft] = useState(false);

  const router = useRouter();
  const create = useCreateWorkflow();
  const update = useUpdateWorkflow(isNew ? "" : id);
  const trigger = useTriggerWorkflow();

  const { data: allWorkflows } = useWorkflows();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => setNodes((ns) => applyNodeChanges(changes, ns)),
    [],
  );
  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => setEdges((es) => applyEdgeChanges(changes, es)),
    [],
  );
  const onConnect = useCallback(
    (c: Connection) => setEdges((es) => addEdge({ ...c, animated: true }, es)),
    [],
  );

  function buildTriggerForRequest(): { type: "cron" | "webhook" | "manual" | "price_alert" | "on_chain"; [k: string]: unknown } {
    if (!isNew && data?.trigger_type) {
      const t = data.trigger_type as "cron" | "webhook" | "manual" | "price_alert" | "on_chain";
      return { type: t, ...(data.trigger_config as object ?? {}) };
    }
    return { type: "manual" };
  }

  async function onSave() {
    setBusy(true); setError(null);
    try {
      const steps = graphToSteps(nodes, edges, params);
      if (isNew) {
        const r = await create.mutateAsync({
          name,
          trigger: buildTriggerForRequest(),
          steps,
        });
        clearDraft();
        router.replace(`/workflows/${r.workflow_id}`);
      } else {
        await update.mutateAsync({
          trigger: buildTriggerForRequest(),
          steps,
        });
        clearDraft();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  }

  async function onPublish() {
    setBusy(true); setError(null);
    try {
      const steps = graphToSteps(nodes, edges, params);
      if (isNew) {
        // Create, then navigate — activate in the next render after redirect.
        const r = await create.mutateAsync({
          name,
          trigger: buildTriggerForRequest(),
          steps,
          is_public: false,
        });
        clearDraft();
        router.replace(`/workflows/${r.workflow_id}`);
        return;
      }
      await update.mutateAsync({
        trigger: buildTriggerForRequest(),
        steps,
        is_active: true,
      });
      clearDraft();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Publish failed");
    } finally {
      setBusy(false);
    }
  }

  async function onTestRun() {
    if (nodes.length === 0) {
      setError("Add at least one step before running");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const steps = graphToSteps(nodes, edges, params);
      let workflowId = id;
      if (isNew) {
        const created = await create.mutateAsync({
          name,
          trigger: buildTriggerForRequest(),
          steps,
        });
        workflowId = created.workflow_id;
        clearDraft();
        router.replace(`/workflows/${workflowId}`);
      }
      const r = await trigger.mutateAsync({ id: workflowId });
      router.push(`/runs/${r.run_id}`);
    } catch (e) {
      if (e instanceof ApiError && e.status === 402) {
        setError("Not enough credits — top up credits for your org, then try again.");
      } else {
        setError(e instanceof Error ? e.message : "Test run failed");
      }
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => { if (data?.name) setName(data.name); }, [data?.name]);

  useEffect(() => {
    if (!data?.steps) return;
    const steps = data.steps as ApiStep[];
    const g = stepsToGraph(steps);
    setNodes(g.nodes);
    setEdges(g.edges);
  }, [data?.steps]);

  // Hydrate from draft only if:
  //   - we have a draft AND the draft actually has nodes (skip empty drafts —
  //     they were written by the old broken auto-save and would race-override
  //     the server data with [])
  //   - server data hasn't populated nodes yet
  //   - we haven't hydrated already (one-shot)
  // For id === "new" there's no server data, so a non-empty draft wins on mount.
  useLayoutEffect(() => {
    if (hydratedFromDraft || !draft) return;
    const draftNodes = (draft.nodes as Node<StepNodeData>[] | undefined) ?? [];
    if (draftNodes.length === 0) return; // empty draft is junk; ignore
    if (data?.steps && (data.steps as ApiStep[]).length > 0) return; // server wins
    if (nodes.length > 0) return; // user already started editing
    setName(draft.name);
    setNodes(draftNodes);
    setEdges(draft.edges as Edge[]);
    setParams(draft.params);
    setHydratedFromDraft(true);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft, data?.steps]);

  // Debounced auto-save: write a draft 500ms after the last change.
  // Skip empty graphs — auto-saving [] would clobber the server data on
  // future mounts before it hydrates.
  useEffect(() => {
    if (nodes.length === 0) return;
    const t = setTimeout(() => {
      saveDraft({
        name,
        nodes: nodes as unknown[],
        edges: edges as unknown[],
        params,
      });
    }, 500);
    return () => clearTimeout(t);
  }, [name, nodes, edges, params, saveDraft]);

  // Keep sub-workflow / emit_webhook node labels in sync with picked workflow id → name.
  // Runs only when allWorkflows or params change; uses functional setNodes to avoid
  // adding `nodes` to the dependency array (which would create an update loop).
  useEffect(() => {
    if (!allWorkflows) return;
    setNodes((prev) => {
      let changed = false;
      const next = prev.map((n) => {
        const isPicker =
          n.data.plugin === "solhub" &&
          (n.data.action === "run_workflow" || n.data.action === "emit_webhook");
        if (!isPicker) return n;
        const pickedField =
          n.data.action === "run_workflow" ? "workflow_id" : "target_workflow_id";
        const pickedId = params[n.id]?.[pickedField];
        if (typeof pickedId !== "string" || !pickedId) {
          if (n.data.subWorkflowName != null) {
            changed = true;
            return { ...n, data: { ...n.data, subWorkflowName: undefined } };
          }
          return n;
        }
        const picked = allWorkflows.find((w) => w.id === pickedId);
        const newName = picked?.name;
        if (n.data.subWorkflowName !== newName) {
          changed = true;
          return { ...n, data: { ...n.data, subWorkflowName: newName } };
        }
        return n;
      });
      return changed ? next : prev;
    });
  }, [allWorkflows, params]);

  function addStep(plugin: string, action: string) {
    const id = `step_${Math.random().toString(36).slice(2, 8)}`;
    const found = findAction(plugin, action);
    setNodes((prev) => [
      ...prev,
      {
        id,
        type: "step",
        position: { x: 80 + prev.length * 240, y: 160 },
        data: { label: `${plugin}.${action}`, plugin, action },
      },
    ]);
    setParams((p) => ({ ...p, [id]: { ...(found?.action.defaults ?? {}) } }));
  }

  const selectedNode = nodes.find((n) => n.id === selectedId) ?? null;
  const selectedFallbackParams =
    selectedNode
      ? (findAction(selectedNode.data.plugin, selectedNode.data.action)?.action.defaults ?? {})
      : {};
  const selectedParams: Record<string, unknown> = selectedId
    ? (params[selectedId] ?? selectedFallbackParams)
    : {};

  function setSelectedParams(v: Record<string, unknown>) {
    if (!selectedId) return;
    setParams((p) => ({ ...p, [selectedId]: v }));
  }

  function deleteSelected() {
    if (!selectedId) return;
    setNodes((n) => n.filter((x) => x.id !== selectedId));
    setEdges((e) => e.filter((x) => x.source !== selectedId && x.target !== selectedId));
    setParams((p) => {
      const next = { ...p };
      delete next[selectedId];
      return next;
    });
    setSelectedId(null);
  }

  return (
    <div className="h-screen flex flex-col">
      <header className="h-12 border-b border-ink-200 bg-white flex items-center justify-between px-4">
        <div className="flex items-center gap-3 min-w-0">
          <SolhubLogo />
          <span className="text-ink-300">/</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Untitled workflow"
            className="text-[13px] font-medium bg-transparent focus:outline-none px-2 py-1 hover:bg-ink-50 rounded min-w-[200px]"
          />
          {!isNew && (data?.is_active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">draft</Pill>)}
          {isNew && <Pill tone="ink">new</Pill>}
          {isNew && nodes.length === 0 && (
            <ExamplePicker
              onLoad={(ex) => {
                const g = exampleToGraph(ex);
                setName(g.name);
                setNodes(g.nodes);
                setEdges(g.edges);
                setParams(g.params);
              }}
            />
          )}
        </div>
        <div className="flex items-center gap-2">
          {error && <span className="text-[11px] text-rose-600 truncate max-w-[180px]">{error}</span>}
          <Btn variant="default" size="sm" onClick={onTestRun} disabled={busy || nodes.length === 0}>
            {busy ? "Running…" : "Test run"}
          </Btn>
          <Btn variant="primary" size="sm" onClick={onSave} disabled={busy || nodes.length === 0}>
            {busy ? "Saving…" : "Save"}
          </Btn>
          <Btn variant="success" size="sm" onClick={onPublish} disabled={busy || nodes.length === 0}>
            Publish
          </Btn>
        </div>
      </header>
      <div className="flex-1 min-h-0 grid grid-cols-[240px_1fr_320px] grid-rows-[1fr] overflow-hidden">
        <aside className="border-r border-ink-200 bg-white">
          <ToolPalette onAdd={addStep} />
        </aside>
        <main className="bg-ink-50 h-full w-full overflow-hidden">
          <Canvas
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onSelect={setSelectedId}
          />
        </main>
        <aside className="border-l border-ink-200 bg-white overflow-y-auto">
          <Inspector
            node={selectedNode}
            params={selectedParams}
            onParamsChange={setSelectedParams}
            onDelete={deleteSelected}
            currentWorkflowId={isNew ? undefined : id}
          />
        </aside>
      </div>
    </div>
  );
}

function ExamplePicker({ onLoad }: { onLoad: (ex: WorkflowExample) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDoc(e: MouseEvent) {
      if (!ref.current?.contains(e.target as HTMLElement)) setOpen(false);
    }
    if (open) document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div ref={ref} className="relative">
      <Btn
        size="sm"
        variant="ghost"
        icon={<Icon name="spark" className="w-3 h-3" />}
        onClick={() => setOpen(!open)}
      >
        Load example
      </Btn>
      {open && (
        <div className="absolute top-full mt-1 w-[320px] rounded-md border border-ink-200 bg-white shadow-pop z-50">
          {EXAMPLES.map((ex) => (
            <button
              key={ex.id}
              onClick={() => { onLoad(ex); setOpen(false); }}
              className="block w-full text-left px-3 py-2 hover:bg-ink-50 border-b border-ink-100 last:border-b-0"
            >
              <div className="text-[13px] font-medium">{ex.name}</div>
              <div className="text-[11px] text-ink-500 line-clamp-2">{ex.description}</div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
