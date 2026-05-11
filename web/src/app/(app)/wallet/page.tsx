"use client";
import { useWallet } from "@solana/wallet-adapter-react";
import { Topbar } from "@/components/shell/Topbar";
import { BalanceCard } from "@/components/wallet/BalanceCard";
import { useMe } from "@/lib/hooks/use-org";
import { useSolBalance, useUsdcBalance } from "@/lib/hooks/use-balances";
import { formatLamports, formatUsdc } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";

export default function WalletPage() {
  const me = useMe();
  const { publicKey, connected } = useWallet();
  const sol = useSolBalance();
  const usdc = useUsdcBalance();

  return (
    <>
      <Topbar crumbs={["Account", "Wallet & Permissions"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="grid grid-cols-2 gap-4">
          <BalanceCard
            title="Personal wallet"
            subtitle={connected ? "Connected" : "Not connected — use Connect in the topbar"}
            address={connected ? publicKey?.toBase58() ?? null : null}
            network={NETWORK}
            lines={[
              { label: "SOL", value: sol.data != null ? formatLamports(sol.data) : "—" },
              { label: "USDC", value: usdc.data != null ? formatUsdc(usdc.data) : "—" },
            ]}
          />
          <BalanceCard
            title="Org signing wallet"
            subtitle="Read-only · Turnkey-managed"
            address={me.data?.wallet_address ?? null}
            network={NETWORK}
            lines={[
              {
                label: "Credits",
                value: me.data ? formatUsdc(BigInt(me.data.credits_usdc)) : "—",
              },
            ]}
            footer={
              <div className="text-[11px] text-ink-500">
                Deposit / withdraw will appear here once the execution-vault Anchor program is deployed and its IDL is available.
              </div>
            }
          />
        </div>
      </main>
    </>
  );
}
