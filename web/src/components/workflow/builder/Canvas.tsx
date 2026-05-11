"use client";
import { useCallback, useMemo } from "react";
import {
  ReactFlow, Controls, MiniMap, Background, BackgroundVariant,
  useNodesState, useEdgesState, addEdge,
  type Connection, type Edge, type Node, type NodeTypes,
  type OnSelectionChangeParams,
} from "reactflow";
import { StepNode, type StepNodeData } from "./StepNode";

export interface CanvasProps {
  initialNodes: Node<StepNodeData>[];
  initialEdges: Edge[];
  onSelect: (nodeId: string | null) => void;
}

export function Canvas({ initialNodes, initialEdges, onSelect }: CanvasProps) {
  const [nodes, , onNodesChange] = useNodesState<StepNodeData>(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const nodeTypes: NodeTypes = useMemo(() => ({ step: StepNode }), []);

  const onConnect = useCallback(
    (c: Connection) => setEdges((eds) => addEdge({ ...c, animated: true }, eds)),
    [setEdges],
  );

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
      fitView
      defaultEdgeOptions={{ animated: true, style: { stroke: "#71717a", strokeWidth: 1.5 } }}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="rgba(24,24,27,0.18)" />
      <MiniMap pannable zoomable className="!bg-white !border !border-ink-200" />
      <Controls className="!shadow-card !border !border-ink-200" />
    </ReactFlow>
  );
}
