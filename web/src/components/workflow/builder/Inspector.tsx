"use client";
import type { Node } from "reactflow";
import type { StepNodeData } from "./StepNode";
import { findAction } from "@/lib/plugins/registry";
import { Pill } from "@/components/primitives/Pill";
import { ZodForm } from "./ZodForm";
import type { z } from "zod";

export interface InspectorProps {
  node: Node<StepNodeData> | null;
  params: Record<string, unknown>;
  onParamsChange: (v: Record<string, unknown>) => void;
  onDelete: () => void;
}

export function Inspector({ node, params, onParamsChange, onDelete }: InspectorProps) {
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
  return (
    <div className="p-4 space-y-4">
      <div>
        <div className="flex items-center gap-1.5 mb-2">
          <Pill tone="violet">{found.plugin.name}</Pill>
          <Pill tone="ink">{found.action.name}</Pill>
          {found.plugin.status === "stub" && <Pill tone="amber">stub</Pill>}
        </div>
        <h2 className="text-[15px] font-semibold tracking-tight">{found.action.name}</h2>
        <p className="text-[12px] text-ink-500">{found.action.description}</p>
      </div>
      <ZodForm
        schema={found.action.schema as z.ZodObject<z.ZodRawShape>}
        value={params}
        onChange={onParamsChange}
      />
      <button
        onClick={onDelete}
        className="text-[12px] text-rose-600 hover:underline"
      >
        Delete step
      </button>
    </div>
  );
}
