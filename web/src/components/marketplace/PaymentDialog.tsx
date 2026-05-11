"use client";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { useQueryClient } from "@tanstack/react-query";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";
import { paymentInfo, callHubWorkflow, type PaymentRequirements } from "@/lib/api/hub";
import { ApiError } from "@/lib/api/client";
import { buildPaymentTx } from "@/lib/solana/x402";
import { formatAddress, formatLamports, solscanAccount, solscanTx } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";

type Phase =
  | "checking"
  | "no_payment"
  | "needs_wallet"
  | "ready"
  | "signing"
  | "confirming"
  | "calling"
  | "done"
  | "error";

export interface PaymentDialogProps {
  workflowId: string;
  workflowName: string;
  onClose: () => void;
}

export function PaymentDialog({ workflowId, workflowName, onClose }: PaymentDialogProps) {
  const router = useRouter();
  const { publicKey, sendTransaction, connected } = useWallet();
  const { setVisible } = useWalletModal();
  const { connection } = useConnection();
  const qc = useQueryClient();

  const [phase, setPhase] = useState<Phase>("checking");
  const [reqs, setReqs] = useState<PaymentRequirements | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [txSig, setTxSig] = useState<string | null>(null);

  // 1. Fetch payment info on mount.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const r = await paymentInfo(workflowId);
        if (cancelled) return;
        setReqs(r);
        if (r.amount_lamports === 0) {
          // Free workflow — call directly without payment.
          setPhase("calling");
          try {
            const c = await callHubWorkflow(workflowId);
            qc.invalidateQueries({ queryKey: ["runs"] });
            router.push(`/runs/${c.run_id}`);
          } catch (e) {
            setError(e instanceof Error ? e.message : "Call failed");
            setPhase("error");
          }
        } else {
          setPhase(connected ? "ready" : "needs_wallet");
        }
      } catch (e) {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : "Failed to load payment info");
        setPhase("error");
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workflowId]);

  // 2. When wallet connects after we've fetched reqs, move to "ready".
  useEffect(() => {
    if (phase === "needs_wallet" && connected) setPhase("ready");
  }, [connected, phase]);

  async function pay() {
    if (!reqs || !publicKey) return;
    setError(null);
    setPhase("signing");
    try {
      const tx = await buildPaymentTx(connection, publicKey, reqs);
      const sig = await sendTransaction(tx, connection);
      setTxSig(sig);
      setPhase("confirming");
      await connection.confirmTransaction(sig, "confirmed");

      setPhase("calling");
      const c = await callHubWorkflow(workflowId, { paymentSignature: sig });
      qc.invalidateQueries({ queryKey: ["runs"] });
      setPhase("done");
      router.push(`/runs/${c.run_id}`);
    } catch (e) {
      if (e instanceof ApiError && e.status === 402) {
        setError("Payment verification failed — try again.");
      } else {
        setError(e instanceof Error ? e.message : "Payment failed");
      }
      setPhase("error");
    }
  }

  const busy = phase === "signing" || phase === "confirming" || phase === "calling";

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[480px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        <div className="flex items-start justify-between">
          <div className="min-w-0">
            <h2 className="text-[16px] font-semibold tracking-tight">Run &quot;{workflowName}&quot;</h2>
            <p className="text-[12px] text-ink-500 mt-1">
              {reqs?.amount_lamports === 0
                ? "This workflow is free."
                : reqs
                ? "This workflow requires payment via x402."
                : "Loading payment requirements…"}
            </p>
          </div>
          <button onClick={onClose} className="text-ink-500 hover:text-ink-900">
            <Icon name="x" className="w-4 h-4" />
          </button>
        </div>

        {phase === "checking" && (
          <div className="text-[12px] text-ink-500">Checking…</div>
        )}

        {reqs && reqs.amount_lamports > 0 && (
          <div className="rounded-lg border border-ink-200 bg-ink-50 p-3 space-y-1.5">
            <Row label="Network" value={<Pill tone="amber">{reqs.network}</Pill>} />
            <Row
              label="Amount"
              value={
                <span className="font-mono">{formatLamports(BigInt(reqs.amount_lamports))}</span>
              }
            />
            <Row
              label="To"
              value={
                <a
                  href={solscanAccount(reqs.recipient, NETWORK)}
                  target="_blank"
                  rel="noreferrer"
                  className="font-mono text-violet-700 underline"
                >
                  {formatAddress(reqs.recipient, 6, 4)}
                </a>
              }
            />
            <Row
              label="Memo"
              value={<span className="font-mono text-[11px] truncate">{reqs.memo}</span>}
            />
          </div>
        )}

        {txSig && (
          <div className="text-[12px] text-ink-600">
            Sent:{" "}
            <a
              className="font-mono text-violet-700 underline"
              target="_blank"
              rel="noreferrer"
              href={solscanTx(txSig, NETWORK)}
            >
              {formatAddress(txSig, 8, 8)}
            </a>
          </div>
        )}

        {error && <p className="text-[12px] text-rose-600">{error}</p>}

        <div className="flex justify-end gap-2">
          <Btn variant="ghost" onClick={onClose} disabled={busy}>
            Cancel
          </Btn>
          {phase === "needs_wallet" && (
            <Btn variant="primary" onClick={() => setVisible(true)}>
              Connect wallet
            </Btn>
          )}
          {(phase === "ready" || phase === "error") &&
            reqs &&
            reqs.amount_lamports > 0 &&
            connected && (
              <Btn variant="primary" onClick={pay}>
                Pay {formatLamports(BigInt(reqs.amount_lamports))}
              </Btn>
            )}
          {busy && (
            <Btn variant="primary" disabled>
              {phase === "signing"
                ? "Sign in wallet…"
                : phase === "confirming"
                ? "Confirming…"
                : "Starting run…"}
            </Btn>
          )}
        </div>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between text-[12px]">
      <span className="uppercase tracking-wider font-mono text-ink-500">{label}</span>
      <span>{value}</span>
    </div>
  );
}
