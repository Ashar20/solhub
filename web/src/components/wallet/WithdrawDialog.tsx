"use client";
import { useEffect, useState } from "react";
import {
  useWallet,
  useConnection,
  useAnchorWallet,
} from "@solana/wallet-adapter-react";
import { useQueryClient } from "@tanstack/react-query";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { executionVaultProgram } from "@/lib/anchor/programs";
import { buildWithdrawTx, fetchCreatorBalance } from "@/lib/anchor/deposit";
import { formatUsdc, solscanTx } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";

export interface WithdrawDialogProps {
  onClose: () => void;
}

export function WithdrawDialog({ onClose }: WithdrawDialogProps) {
  const [available, setAvailable] = useState<bigint | null>(null);
  const [amount, setAmount] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [txSig, setTxSig] = useState<string | null>(null);
  const { publicKey, sendTransaction } = useWallet();
  const wallet = useAnchorWallet();
  const { connection } = useConnection();
  const qc = useQueryClient();

  useEffect(() => {
    if (!publicKey || !wallet) {
      setAvailable(0n);
      return;
    }
    let cancelled = false;
    (async () => {
      const program = executionVaultProgram(connection, wallet);
      const bal = await fetchCreatorBalance(program, publicKey);
      if (!cancelled) setAvailable(bal);
    })();
    return () => {
      cancelled = true;
    };
  }, [publicKey, wallet, connection]);

  async function submit() {
    if (!publicKey || !wallet) {
      setErr("Connect a wallet first");
      return;
    }
    const parsed = parseFloat(amount || "0");
    if (!Number.isFinite(parsed) || parsed <= 0) {
      setErr("Enter an amount");
      return;
    }
    const micro = BigInt(Math.floor(parsed * 1_000_000));
    if (available != null && micro > available) {
      setErr(`Exceeds available balance (${formatUsdc(available)})`);
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const program = executionVaultProgram(connection, wallet);
      const tx = await buildWithdrawTx(program, publicKey, micro);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      const sig = await sendTransaction(tx, connection);
      setTxSig(sig);
      await connection.confirmTransaction(sig, "confirmed");
      qc.invalidateQueries({ queryKey: ["balance"] });
      onClose();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Transaction failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[420px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        <div className="flex items-start justify-between">
          <div>
            <h2 className="text-[16px] font-semibold tracking-tight">
              Withdraw earnings
            </h2>
            <p className="text-[12px] text-ink-500">
              Available: {available != null ? formatUsdc(available) : "—"}
            </p>
          </div>
          <button
            onClick={onClose}
            className="text-ink-500 hover:text-ink-900"
          >
            <Icon name="x" className="w-4 h-4" />
          </button>
        </div>
        <label className="block">
          <span className="text-[12px] font-medium">Amount (USDC)</span>
          <input
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            placeholder="10.00"
            disabled={busy}
            className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px] font-mono"
          />
        </label>
        {txSig && (
          <div className="text-[12px] text-ink-600">
            Sent:{" "}
            <a
              className="font-mono text-violet-700 underline"
              target="_blank"
              rel="noreferrer"
              href={solscanTx(txSig, NETWORK)}
            >
              {txSig.slice(0, 10)}…{txSig.slice(-6)}
            </a>
          </div>
        )}
        {err && <p className="text-[12px] text-rose-600">{err}</p>}
        <div className="flex justify-end gap-2">
          <Btn variant="ghost" onClick={onClose} disabled={busy}>
            Cancel
          </Btn>
          <Btn
            variant="primary"
            onClick={submit}
            disabled={
              busy || !amount || available == null || available === 0n
            }
          >
            {busy ? "Submitting…" : "Withdraw"}
          </Btn>
        </div>
      </div>
    </div>
  );
}
