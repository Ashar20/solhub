"use client";
import Link from "next/link";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";
import { StatusPill } from "./StatusPill";
import { formatRelativeTime } from "@/lib/utils/format";
import {
  useUpdateWorkflow,
  useDeleteWorkflow,
  useTriggerWorkflow,
} from "@/lib/hooks/use-workflow-mutations";

export interface WorkflowRowProps {
  w: {
    id: string;
    name: string;
    trigger_type: string;
    is_active: boolean;
    execution_count?: number;
    created_at: string;
    updated_at?: string;
  };
}

export function WorkflowRow({ w }: WorkflowRowProps) {
  const upd = useUpdateWorkflow(w.id);
  const del = useDeleteWorkflow();
  const trig = useTriggerWorkflow();
  const router = useRouter();
  const [busy, setBusy] = useState(false);

  async function onTrigger(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setBusy(true);
    try {
      const r = await trig.mutateAsync({ id: w.id });
      router.push(`/runs/${r.run_id}`);
    } finally { setBusy(false); }
  }

  async function onToggle(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setBusy(true);
    try { await upd.mutateAsync({ is_active: !w.is_active }); }
    finally { setBusy(false); }
  }

  async function onDelete(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!confirm(`Delete workflow "${w.name}"?`)) return;
    setBusy(true);
    try { await del.mutateAsync(w.id); }
    finally { setBusy(false); }
  }

  return (
    <div className="grid grid-cols-[1fr_140px_120px_140px_140px] items-center px-4 h-12 border-b border-ink-100 hover:bg-ink-50 text-[13px]">
      <Link href={`/workflows/${w.id}`} className="flex items-center gap-2 min-w-0 hover:underline">
        <div className="font-medium text-ink-900 truncate">{w.name}</div>
        <Pill tone="ink">{w.trigger_type}</Pill>
      </Link>
      <StatusPill active={w.is_active} />
      <div className="text-ink-500 font-mono">{w.execution_count ?? 0} runs</div>
      <div className="text-[12px] text-ink-500">{formatRelativeTime(w.updated_at ?? w.created_at)}</div>
      <div className="flex justify-end gap-1">
        <button
          onClick={onTrigger}
          disabled={busy}
          title="Trigger now"
          aria-label="Trigger"
          className="w-7 h-7 rounded hover:bg-ink-100 inline-flex items-center justify-center disabled:opacity-50"
        >
          <Icon name="play" className="w-3.5 h-3.5 text-emerald-600" />
        </button>
        <button
          onClick={onToggle}
          disabled={busy}
          title={w.is_active ? "Pause" : "Resume"}
          aria-label={w.is_active ? "Pause" : "Resume"}
          className="w-7 h-7 rounded hover:bg-ink-100 inline-flex items-center justify-center disabled:opacity-50"
        >
          <Icon name={w.is_active ? "pause" : "play"} className="w-3.5 h-3.5 text-ink-700" />
        </button>
        <button
          onClick={onDelete}
          disabled={busy}
          title="Delete"
          aria-label="Delete"
          className="w-7 h-7 rounded hover:bg-rose-50 inline-flex items-center justify-center disabled:opacity-50"
        >
          <Icon name="trash" className="w-3.5 h-3.5 text-rose-600" />
        </button>
      </div>
    </div>
  );
}
