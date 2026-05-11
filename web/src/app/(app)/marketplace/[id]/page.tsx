"use client";
import { useState } from "react";
import Link from "next/link";
import { Topbar } from "@/components/shell/Topbar";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";
import { useHub } from "@/lib/hooks/use-hub";
import { PaymentDialog } from "@/components/marketplace/PaymentDialog";
import { formatAddress, formatLamports, formatRelativeTime, solscanAccount } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";
import { findAction } from "@/lib/plugins/registry";
import type { WorkflowStep } from "@/lib/api/schemas";

export default function MarketplaceDetail({ params }: { params: { id: string } }) {
  const { id } = params;
  const { data, isLoading } = useHub();
  const wf = (data ?? []).find((w) => w.id === id);
  const [open, setOpen] = useState(false);

  const feeLamports = wf?.fee_per_exec_usdc ?? 0;
  const isPaid = feeLamports > 0;

  // steps is typed as array | record; normalize to WorkflowStep[]
  const steps: WorkflowStep[] = Array.isArray(wf?.steps)
    ? (wf.steps as WorkflowStep[])
    : [];

  return (
    <>
      <Topbar crumbs={["Hub", "Marketplace", wf?.name ?? id.slice(0, 8)]} />
      <main className="flex-1 p-6 overflow-y-auto">
        {isLoading && <div className="text-[12px] text-ink-500">Loading…</div>}
        {!isLoading && !wf && (
          <div className="rounded-xl border border-ink-200 bg-white shadow-card p-6">
            <p className="text-[13px] text-ink-700 mb-1">Workflow not in the public hub.</p>
            <p className="text-[12px] text-ink-500">It may be private or no longer listed.</p>
            <Link
              href="/marketplace"
              className="text-[12px] text-violet-700 hover:underline mt-3 inline-block"
            >
              ← Back to marketplace
            </Link>
          </div>
        )}
        {wf && (
          <div className="grid grid-cols-[1fr_320px] gap-6">
            <div>
              <div className="flex items-center gap-2 mb-2 flex-wrap">
                <h1 className="text-[22px] font-semibold tracking-tight">{wf.name}</h1>
                <Pill tone={wf.is_active ? "emerald" : "amber"}>
                  {wf.is_active ? "live" : "paused"}
                </Pill>
                {isPaid && <Pill tone="sol">x402 paid</Pill>}
              </div>
              <div className="text-[12px] font-mono text-ink-500 mb-4">
                <a
                  href={solscanAccount(wf.org_id, NETWORK)}
                  target="_blank"
                  rel="noreferrer"
                  className="hover:underline"
                >
                  {formatAddress(wf.org_id, 6, 4)}
                </a>
                {" · "}
                <Pill tone="violet">{wf.trigger_type}</Pill>
              </div>
              <section className="rounded-xl border border-ink-200 bg-white shadow-card p-4">
                <h2 className="text-[12px] uppercase tracking-wider font-mono text-ink-500 mb-3">
                  Steps
                </h2>
                {steps.length === 0 && (
                  <p className="text-[12px] text-ink-500">No steps configured.</p>
                )}
                <ol className="space-y-2">
                  {steps.map((s, i) => {
                    const found = findAction(s.plugin, s.action);
                    return (
                      <li
                        key={s.id}
                        className="flex items-start gap-3 rounded-lg border border-ink-100 p-3"
                      >
                        <div className="w-5 h-5 rounded-full bg-ink-100 text-[10px] font-mono flex items-center justify-center text-ink-700 shrink-0 mt-0.5">
                          {i + 1}
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 flex-wrap">
                            <Pill tone="violet">{s.plugin}</Pill>
                            <Pill tone="ink">{s.action}</Pill>
                            {!found && <Pill tone="rose">unknown</Pill>}
                          </div>
                          {found?.action.description && (
                            <p className="text-[11px] text-ink-500 mt-1">
                              {found.action.description}
                            </p>
                          )}
                        </div>
                      </li>
                    );
                  })}
                </ol>
              </section>
            </div>
            <aside className="rounded-xl border border-ink-200 bg-white shadow-card p-4 space-y-3 h-fit">
              <div className="grid grid-cols-2 gap-3 text-[12px] font-mono">
                <div>
                  <div className="text-ink-400">Runs</div>
                  <div className="text-ink-900">{Number(wf.execution_count).toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-ink-400">Fee / exec</div>
                  <div className="text-ink-900">
                    {isPaid ? formatLamports(BigInt(feeLamports)) : "free"}
                  </div>
                </div>
                <div>
                  <div className="text-ink-400">Created</div>
                  <div className="text-ink-900">{formatRelativeTime(wf.created_at)}</div>
                </div>
                <div>
                  <div className="text-ink-400">Steps</div>
                  <div className="text-ink-900">{steps.length}</div>
                </div>
              </div>
              <Btn
                variant="primary"
                size="lg"
                className="w-full justify-center"
                icon={<Icon name="play" className="w-3.5 h-3.5" />}
                onClick={() => setOpen(true)}
              >
                Use this workflow
              </Btn>
              <Link
                href="/marketplace"
                className="text-[12px] text-ink-500 hover:text-ink-900 inline-flex items-center gap-1"
              >
                <Icon name="chevron" className="w-3 h-3 rotate-180" />
                Back to marketplace
              </Link>
            </aside>
          </div>
        )}
        {open && wf && (
          <PaymentDialog
            workflowId={wf.id}
            workflowName={wf.name}
            onClose={() => setOpen(false)}
          />
        )}
      </main>
    </>
  );
}
