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

/** Plugin/action/field combos that should render a WorkflowPicker instead of a text input. */
const WORKFLOW_PICKER_FIELDS: Array<{ plugin: string; action: string; field: string }> = [
  { plugin: "solhub", action: "run_workflow", field: "workflow_id" },
  { plugin: "solhub", action: "emit_webhook", field: "target_workflow_id" },
];

function pickerFieldFor(plugin: string, action: string): string | null {
  return (
    WORKFLOW_PICKER_FIELDS.find((x) => x.plugin === plugin && x.action === action)?.field ?? null
  );
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

  const pickerField = pickerFieldFor(node.data.plugin, node.data.action);
  const isSubWorkflow = node.data.plugin === "solhub" && node.data.action === "run_workflow";
  const isEmitWebhook = node.data.plugin === "solhub" && node.data.action === "emit_webhook";

  return (
    <div className="p-4 space-y-4">
      <div>
        <div className="flex items-center gap-1.5 mb-2 flex-wrap">
          <Pill tone="violet">{found.plugin.name}</Pill>
          <Pill tone="ink">{found.action.name}</Pill>
          {found.plugin.status === "stub" && <Pill tone="amber">stub</Pill>}
          {isSubWorkflow && <Pill tone="sol">sub-workflow</Pill>}
          {isEmitWebhook && <Pill tone="sol">emit-webhook</Pill>}
        </div>
        <h2 className="text-[15px] font-semibold tracking-tight">{found.action.name}</h2>
        <p className="text-[12px] text-ink-500">{found.action.description}</p>
      </div>

      {pickerField ? (
        <WorkflowPickerForm
          action={found.action as unknown as { schema: z.ZodObject<z.ZodRawShape> }}
          params={params}
          onChange={onParamsChange}
          pickerField={pickerField}
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

function WorkflowPickerForm({
  action, params, onChange, pickerField, excludeId,
}: {
  action: { schema: z.ZodObject<z.ZodRawShape> };
  params: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
  pickerField: string;
  excludeId?: string;
}) {
  const pickerValue = typeof params[pickerField] === "string" ? (params[pickerField] as string) : "";

  return (
    <div className="space-y-3">
      <div>
        <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider block mb-1">
          {pickerField}
        </span>
        <WorkflowPicker
          value={pickerValue}
          onChange={(id) => onChange({ ...params, [pickerField]: id })}
          excludeId={excludeId}
        />
      </div>
      <ZodForm
        schema={action.schema}
        value={params}
        onChange={onChange}
        skip={[pickerField]}
      />
    </div>
  );
}
