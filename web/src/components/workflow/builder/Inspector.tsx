"use client";
import type { Node } from "reactflow";
import type { StepNodeData } from "./StepNode";
import { findAction } from "@/lib/plugins/registry";
import { Pill } from "@/components/primitives/Pill";
import { ZodForm } from "./ZodForm";
import { WorkflowPicker } from "./WorkflowPicker";
import type { z } from "zod";

export interface InspectorProps {
  node: Node<StepNodeData> | null;
  params: Record<string, unknown>;
  onParamsChange: (v: Record<string, unknown>) => void;
  onDelete: () => void;
  /** ID of the workflow currently being edited (excluded from sub-workflow picker). */
  currentWorkflowId?: string;
}

export function Inspector({
  node, params, onParamsChange, onDelete, currentWorkflowId,
}: InspectorProps) {
  if (!node) {
    return <div className="p-4 text-[12px] text-ink-500">Select a step to inspect.</div>;
  }
  const found = findAction(node.data.plugin, node.data.action);
  if (!found) {
    return (
      <div className="p-4 text-[12px] text-rose-600">
        Unknown plugin/action: {node.data.plugin}.{node.data.action}
      </div>
    );
  }

  const isSubWorkflow = node.data.plugin === "solhub" && node.data.action === "run_workflow";

  return (
    <div className="p-4 space-y-4">
      <div>
        <div className="flex items-center gap-1.5 mb-2 flex-wrap">
          <Pill tone="violet">{found.plugin.name}</Pill>
          <Pill tone="ink">{found.action.name}</Pill>
          {found.plugin.status === "stub" && <Pill tone="amber">stub</Pill>}
          {isSubWorkflow && <Pill tone="sol">sub-workflow</Pill>}
        </div>
        <h2 className="text-[15px] font-semibold tracking-tight">{found.action.name}</h2>
        <p className="text-[12px] text-ink-500">{found.action.description}</p>
      </div>

      {isSubWorkflow ? (
        <SubWorkflowForm
          params={params}
          onChange={onParamsChange}
          excludeId={currentWorkflowId}
        />
      ) : (
        <ZodForm
          schema={found.action.schema as z.ZodObject<z.ZodRawShape>}
          value={params}
          onChange={onParamsChange}
        />
      )}

      <button
        onClick={onDelete}
        className="text-[12px] text-rose-600 hover:underline"
      >
        Delete step
      </button>
    </div>
  );
}

function SubWorkflowForm({
  params, onChange, excludeId,
}: {
  params: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
  excludeId?: string;
}) {
  const workflowId = typeof params.workflow_id === "string" ? params.workflow_id : "";
  const timeoutSecs = typeof params.timeout_secs === "number" || typeof params.timeout_secs === "string"
    ? String(params.timeout_secs)
    : "60";

  return (
    <div className="space-y-3">
      <div>
        <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider block mb-1">
          workflow
        </span>
        <WorkflowPicker
          value={workflowId}
          onChange={(id) => onChange({ ...params, workflow_id: id })}
          excludeId={excludeId}
        />
      </div>
      <label className="block">
        <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider">
          timeout_secs
        </span>
        <input
          type="text"
          inputMode="numeric"
          value={timeoutSecs}
          onChange={(e) => onChange({ ...params, timeout_secs: e.target.value })}
          className="mt-1 w-full h-8 px-2 rounded-md border border-ink-200 text-[13px] font-mono"
        />
      </label>
    </div>
  );
}
