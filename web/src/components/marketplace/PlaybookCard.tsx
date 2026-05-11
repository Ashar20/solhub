import Link from "next/link";
import { Pill } from "@/components/primitives/Pill";
import { formatUsdc, formatAddress } from "@/lib/utils/format";
import type { useHub } from "@/lib/hooks/use-hub";

type HubItem = NonNullable<ReturnType<typeof useHub>["data"]>[number];

export interface PlaybookCardProps {
  /** A Workflow from the hub listing — same shape as a regular workflow. */
  w: HubItem;
}

export function PlaybookCard({ w }: PlaybookCardProps) {
  return (
    <Link
      href={`/marketplace/${w.id}`}
      className="rounded-xl border border-ink-200 bg-white shadow-card p-4 hover:shadow-pop transition-shadow flex flex-col"
    >
      <div className="flex items-start justify-between mb-2 gap-2">
        <div className="min-w-0">
          <div className="text-[14px] font-semibold tracking-tight truncate">{w.name}</div>
          <div className="text-[11px] font-mono text-ink-500 truncate">{formatAddress(w.org_id, 6, 4)}</div>
        </div>
        <Pill tone={w.is_active ? "emerald" : "amber"}>{w.is_active ? "live" : "paused"}</Pill>
      </div>
      <div className="flex flex-wrap gap-1 mb-3">
        <Pill tone="violet">{w.trigger_type}</Pill>
      </div>
      <div className="mt-auto grid grid-cols-2 text-[11px] font-mono">
        <div>
          <div className="text-ink-400">Runs</div>
          <div className="text-ink-900">{Number(w.execution_count).toLocaleString()}</div>
        </div>
        <div>
          <div className="text-ink-400">Fee / exec</div>
          <div className="text-ink-900">{w.fee_per_exec_usdc != null ? formatUsdc(BigInt(w.fee_per_exec_usdc)) : "—"}</div>
        </div>
      </div>
    </Link>
  );
}
