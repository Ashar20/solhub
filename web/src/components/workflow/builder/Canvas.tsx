"use client";
import { useCallback, useEffect, useMemo, useRef } from "react";
import {
  ReactFlow, ReactFlowProvider, Controls, MiniMap, Background, BackgroundVariant,
  useReactFlow,
  type Edge, type Node, type NodeTypes,
  type OnNodesChange, type OnEdgesChange, type OnConnect,
  type OnSelectionChangeParams,
} from "reactflow";
import { StepNode, type StepNodeData } from "./StepNode";

export interface CanvasProps {
  nodes: Node<StepNodeData>[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: OnConnect;
  onSelect: (nodeId: string | null) => void;
}

function CanvasInner({
  nodes, edges, onNodesChange, onEdgesChange, onConnect, onSelect,
}: CanvasProps) {
  const nodeTypes: NodeTypes = useMemo(() => ({ step: StepNode }), []);
  const rf = useReactFlow();
  const fittedFor = useRef<number>(-1);

  // When nodes first arrive (or grow from 0), refit the viewport.
  // Without this, fitView only runs on the initial empty mount and the
  // populated nodes are never brought into view.
  useEffect(() => {
    if (nodes.length === 0) return;
    if (fittedFor.current === nodes.length) return;
    fittedFor.current = nodes.length;
    // Defer so React Flow has measured the new nodes before we fit.
    const id = window.requestAnimationFrame(() => {
      rf.fitView({ padding: 0.2, duration: 200 });
    });
    return () => window.cancelAnimationFrame(id);
  }, [nodes.length, rf]);

  const handleSelection = useCallback(
    (p: OnSelectionChangeParams) => onSelect(p.nodes[0]?.id ?? null),
    [onSelect],
  );

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onSelectionChange={handleSelection}
      defaultEdgeOptions={{ animated: true, style: { stroke: "#71717a", strokeWidth: 1.5 } }}
      proOptions={{ hideAttribution: true }}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="rgba(24,24,27,0.18)" />
      <MiniMap pannable zoomable className="!bg-white !border !border-ink-200" />
      <Controls className="!shadow-card !border !border-ink-200" />
    </ReactFlow>
  );
}

export function Canvas(props: CanvasProps) {
  return (
    <div className="h-full w-full">
      <ReactFlowProvider>
        <CanvasInner {...props} />
      </ReactFlowProvider>
    </div>
  );
}
