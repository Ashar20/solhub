"use client";
import { Handle, Position, type NodeProps } from "reactflow";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";

export interface StepNodeData {
  /** Display label, typically `${plugin}.${action}` */
  label: string;
  plugin: string;
  action: string;
  /** When plugin/action === solhub/run_workflow, name of the picked sub-workflow. */
  subWorkflowName?: string;
}

export function StepNode({ data, selected }: NodeProps<StepNodeData>) {
  const isSubWorkflow = data.plugin === "solhub" && data.action === "run_workflow";

  return (
    <div className={
      "rounded-lg border bg-white shadow-card px-3 py-2 min-w-[180px] " +
      (selected
        ? "border-violet-500 ring-2 ring-violet-500/30"
        : isSubWorkflow
        ? "border-sol-purple/40"
        : "border-ink-200")
    }>
      <Handle type="target" position={Position.Left} className="!bg-ink-400 !w-2 !h-2" />
      <div className="flex items-center gap-1 text-[12px] font-medium text-ink-900 truncate">
        {isSubWorkflow && <Icon name="arrow" className="w-3 h-3 text-sol-purple" />}
        {isSubWorkflow ? (data.subWorkflowName ?? "Select sub-workflow…") : data.label}
      </div>
      <div className="flex items-center gap-1 mt-1 flex-wrap">
        <Pill tone="violet">{data.plugin}</Pill>
        <Pill tone="ink">{data.action}</Pill>
      </div>
      <Handle type="source" position={Position.Right} className="!bg-ink-400 !w-2 !h-2" />
    </div>
  );
}
