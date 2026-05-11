"use client";
import { Handle, Position, type NodeProps } from "reactflow";
import { Pill } from "@/components/primitives/Pill";

export interface StepNodeData {
  /** Display label, typically `${plugin}.${action}` */
  label: string;
  plugin: string;
  action: string;
}

export function StepNode({ data, selected }: NodeProps<StepNodeData>) {
  return (
    <div className={
      "rounded-lg border bg-white shadow-card px-3 py-2 min-w-[180px] " +
      (selected ? "border-violet-500 ring-2 ring-violet-500/30" : "border-ink-200")
    }>
      <Handle type="target" position={Position.Left} className="!bg-ink-400 !w-2 !h-2" />
      <div className="text-[12px] font-medium text-ink-900 truncate">{data.label}</div>
      <div className="flex items-center gap-1 mt-1">
        <Pill tone="violet">{data.plugin}</Pill>
        <Pill tone="ink">{data.action}</Pill>
      </div>
      <Handle type="source" position={Position.Right} className="!bg-ink-400 !w-2 !h-2" />
    </div>
  );
}
