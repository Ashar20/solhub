"use client";
import { useEffect, useState } from "react";
import type { Node, Edge } from "reactflow";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { Canvas } from "./Canvas";
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

  useEffect(() => { if (data?.name) setName(data.name); }, [data?.name]);

  useEffect(() => {
    if (!data?.steps) return;
    const steps = data.steps as ApiStep[];
    const g = stepsToGraph(steps);
    setNodes(g.nodes);
    setEdges(g.edges);
  }, [data?.steps]);

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
        <aside className="border-r border-ink-200 bg-white flex items-center justify-center text-[11px] text-ink-400 font-mono">
          tool palette (Task 7)
        </aside>
        <main className="bg-ink-50">
          <Canvas
            key={data?.id ?? "new"}
            initialNodes={nodes}
            initialEdges={edges}
            onSelect={setSelectedId}
          />
          {/* setNodes/setEdges unused for now — Task 7+ will use them when adding nodes from palette */}
          {null /* setNodes, setEdges reserved for Task 7+ */}
        </main>
        <aside className="border-l border-ink-200 bg-white overflow-y-auto">
          <div className="p-3 text-[11px] font-mono text-ink-500">
            selected: {selectedId ?? "none"}
          </div>
        </aside>
      </div>
    </div>
  );
}
