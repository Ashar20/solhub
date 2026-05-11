"use client";
import { useState } from "react";
import Link from "next/link";
import { Pill } from "@/components/primitives/Pill";
import { Btn } from "@/components/primitives/Btn";
import { formatAddress, formatLamports } from "@/lib/utils/format";
import { PaymentDialog } from "./PaymentDialog";
import type { useHub } from "@/lib/hooks/use-hub";

type HubItem = NonNullable<ReturnType<typeof useHub>["data"]>[number];

export interface PlaybookCardProps {
  w: HubItem;
}

export function PlaybookCard({ w }: PlaybookCardProps) {
  const [open, setOpen] = useState(false);
  const feeLamports = w.fee_per_exec_usdc ?? 0;
  const isPaid = feeLamports > 0;

  return (
    <>
      <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 hover:shadow-pop transition-shadow flex flex-col">
        <Link href={`/marketplace/${w.id}`} className="min-w-0">
          <div className="flex items-start justify-between mb-2 gap-2">
            <div className="min-w-0">
              <div className="text-[14px] font-semibold tracking-tight truncate">{w.name}</div>
              <div className="text-[11px] font-mono text-ink-500 truncate">
                {formatAddress(w.org_id, 6, 4)}
              </div>
            </div>
            <Pill tone={w.is_active ? "emerald" : "amber"}>
              {w.is_active ? "live" : "paused"}
            </Pill>
          </div>
          <div className="flex flex-wrap gap-1 mb-3">
            <Pill tone="violet">{w.trigger_type}</Pill>
            {isPaid && <Pill tone="sol">x402 paid</Pill>}
          </div>
        </Link>
        <div className="grid grid-cols-2 text-[11px] font-mono mt-auto">
          <div>
            <div className="text-ink-400">Runs</div>
            <div className="text-ink-900">{Number(w.execution_count).toLocaleString()}</div>
          </div>
          <div>
            <div className="text-ink-400">Fee / exec</div>
            <div className="text-ink-900">
              {isPaid ? formatLamports(BigInt(feeLamports)) : "free"}
            </div>
          </div>
        </div>
        <div className="mt-3 pt-3 border-t border-ink-100 flex justify-end">
          <Btn variant="primary" size="sm" onClick={() => setOpen(true)}>
            Use
          </Btn>
        </div>
      </div>
      {open && (
        <PaymentDialog
          workflowId={w.id}
          workflowName={w.name}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}
