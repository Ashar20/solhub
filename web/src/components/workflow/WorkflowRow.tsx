import Link from "next/link";
import { Pill } from "@/components/primitives/Pill";
import { StatusPill } from "./StatusPill";
import { formatRelativeTime } from "@/lib/utils/format";

interface WorkflowRowProps {
  id: string;
  name: string;
  trigger_type: string;
  is_active: boolean;
  execution_count?: number;
  created_at: string;
  updated_at?: string;
}

export function WorkflowRow({ w }: { w: WorkflowRowProps }) {
  return (
    <Link
      href={`/workflows/${w.id}`}
      className="grid grid-cols-[1fr_140px_120px_140px] items-center px-4 h-12 border-b border-ink-100 hover:bg-ink-50 text-[13px]"
    >
      <div className="flex items-center gap-2 min-w-0">
        <div className="font-medium text-ink-900 truncate">{w.name}</div>
        <Pill tone="ink">{w.trigger_type}</Pill>
      </div>
      <StatusPill active={w.is_active} />
      <div className="text-ink-500 font-mono">{w.execution_count ?? 0} runs</div>
      <div className="text-[12px] text-ink-500 text-right">{formatRelativeTime(w.updated_at ?? w.created_at)}</div>
    </Link>
  );
}
