"use client";
import { use } from "react";
import Link from "next/link";
import { Topbar } from "@/components/shell/Topbar";
import { useRun } from "@/lib/hooks/use-runs";
import { useRunStream } from "@/lib/hooks/use-run-stream";
import { StepTimeline } from "@/components/runs/StepTimeline";
import { LiveLogStream } from "@/components/runs/LiveLogStream";
import { RunStatusPill } from "@/components/runs/RunStatusPill";
import { formatLamports, formatSlot, solscanTx, formatAddress } from "@/lib/utils/format";

export default function RunDetail({ params }: { params: Promise<{ run_id: string }> }) {
  const { run_id } = use(params);
  const run = useRun(run_id);
  const stream = useRunStream(run_id);

  const network = (process.env.NEXT_PUBLIC_SOLANA_NETWORK as "mainnet" | "devnet") ?? "devnet";

  return (
    <>
      <Topbar crumbs={["Workspace", "Operate", `Run ${run_id.slice(0, 8)}`]} />
      <main className="flex-1 grid grid-cols-[1fr_480px] gap-4 p-6 overflow-hidden">
        <section className="overflow-y-auto pr-2">
          <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 mb-4">
            <div className="flex items-center gap-3 mb-2 flex-wrap">
              {run.data ? <RunStatusPill status={run.data.status} /> : null}
              {run.data?.signature && (
                <Link
                  href={solscanTx(run.data.signature, network)}
                  target="_blank"
                  rel="noreferrer"
                  className="text-[12px] font-mono text-violet-700 underline"
                >
                  {formatAddress(run.data.signature, 6, 6)}
                </Link>
              )}
              {run.data?.slot != null && (
                <span className="text-[12px] font-mono text-ink-500">slot {formatSlot(BigInt(run.data.slot))}</span>
              )}
              {run.data?.jito_tip_lamports != null && (
                <span className="text-[12px] font-mono text-ink-500">
                  tip {formatLamports(BigInt(run.data.jito_tip_lamports))}
                </span>
              )}
              {run.data?.fee_lamports != null && (
                <span className="text-[12px] font-mono text-ink-500">
                  fee {formatLamports(BigInt(run.data.fee_lamports))}
                </span>
              )}
            </div>
            {run.data?.error_message && (
              <pre className="text-[11px] font-mono text-rose-700 whitespace-pre-wrap mt-2">{run.data.error_message}</pre>
            )}
          </div>
          <StepTimeline steps={run.data?.steps_log ?? []} />
        </section>
        <aside className="overflow-hidden">
          <LiveLogStream events={stream.events} state={stream.state} />
        </aside>
      </main>
    </>
  );
}
