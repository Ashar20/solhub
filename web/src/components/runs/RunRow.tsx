import Link from "next/link";
import type { listRuns } from "@/lib/api/runs";
import { RunStatusPill } from "./RunStatusPill";
import { formatLamports, formatRelativeTime, formatSlot, formatAddress } from "@/lib/utils/format";

export interface RunRowProps {
  r: Awaited<ReturnType<typeof listRuns>>[number];
}

export function RunRow({ r }: RunRowProps) {
  return (
    <Link
      href={`/runs/${r.run_id}`}
      className="grid grid-cols-[100px_1fr_120px_140px_140px_120px] items-center px-4 h-11 border-b border-ink-100 hover:bg-ink-50 text-[12px] font-mono"
    >
      <RunStatusPill status={r.status} />
      <div className="text-ink-900 truncate">{formatAddress(r.workflow_id, 8, 4)}</div>
      <div className="text-ink-500">{formatRelativeTime(r.started_at)}</div>
      <div className="text-ink-500">{r.slot != null ? formatSlot(r.slot) : "—"}</div>
      <div className="text-ink-500">{r.jito_tip_lamports != null ? formatLamports(BigInt(r.jito_tip_lamports)) : "—"}</div>
      <div className="text-ink-400 text-right truncate">{r.signature ? formatAddress(r.signature, 6, 4) : "—"}</div>
    </Link>
  );
}
