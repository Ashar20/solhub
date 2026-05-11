"use client";
import { useEffect, useState } from "react";
import type { Node, Edge } from "reactflow";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { useDraft } from "@/lib/hooks/use-draft";
import { findAction } from "@/lib/plugins/registry";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { Canvas } from "./Canvas";
import { Inspector } from "./Inspector";
import { ToolPalette } from "./ToolPalette";
import type { StepNodeData } from "./StepNode";

export interface BuilderShellProps { id: string }

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

  // Suppress unused variable warning — clearDraft will be called by Task 10 (Save/Publish).
  void clearDraft;

  useEffect(() => { if (data?.name) setName(data.name); }, [data?.name]);

  useEffect(() => {
    if (!data?.steps) return;
    const steps = data.steps as ApiStep[];
    const g = stepsToGraph(steps);
    setNodes(g.nodes);
    setEdges(g.edges);
  }, [data?.steps]);

  // Hydrate from draft only if:
  //   - we have a draft
  //   - server data hasn't populated nodes yet (i.e. canvas is empty)
  //   - we haven't hydrated already (one-shot)
  // For id === "new" there's no server data, so the draft always wins on mount.
  useEffect(() => {
    if (hydratedFromDraft || !draft) return;
    if (data?.steps && (data.steps as ApiStep[]).length > 0) return; // server wins
    if (nodes.length > 0) return; // user already started editing
    setName(draft.name);
    setNodes(draft.nodes as Node<StepNodeData>[]);
    setEdges(draft.edges as Edge[]);
    setParams(draft.params);
    setHydratedFromDraft(true);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft, data?.steps]);

  // Debounced auto-save: write a draft 500ms after the last change.
  useEffect(() => {
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
        </div>
        <div className="flex items-center gap-2">
          <Btn variant="default" size="sm" disabled>Test run</Btn>
          <Btn variant="primary" size="sm" disabled>Save</Btn>
          <Btn variant="success" size="sm" disabled>Publish</Btn>
        </div>
      </header>
      <div className="flex-1 grid grid-cols-[240px_1fr_320px] overflow-hidden">
        <aside className="border-r border-ink-200 bg-white">
          <ToolPalette onAdd={addStep} />
        </aside>
        <main className="bg-ink-50">
          <Canvas
            key={data?.id ?? "new"}
            initialNodes={nodes}
            initialEdges={edges}
            onSelect={setSelectedId}
          />
        </main>
        <aside className="border-l border-ink-200 bg-white overflow-y-auto">
          <Inspector
            node={selectedNode}
            params={selectedParams}
            onParamsChange={setSelectedParams}
            onDelete={deleteSelected}
          />
        </aside>
      </div>
    </div>
  );
}
