"use client";
import { useState } from "react";
import { Topbar } from "@/components/shell/Topbar";
import { useMe } from "@/lib/hooks/use-org";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { ApiKeyList } from "@/components/settings/ApiKeyList";
import { CreateApiKeyDialog } from "@/components/settings/CreateApiKeyDialog";
import { formatAddress, formatUsdc, formatRelativeTime, solscanAccount } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";

export default function SettingsPage() {
  const me = useMe();
  const [open, setOpen] = useState(false);

  return (
    <>
      <Topbar crumbs={["Account", "Settings"]} />
      <main className="flex-1 p-6 overflow-y-auto space-y-6">
        <section>
          <h2 className="text-[14px] font-semibold tracking-tight mb-2">Organisation</h2>
          <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 grid grid-cols-4 gap-3 text-[13px]">
            <Field label="Name" value={me.data?.name ?? "—"} />
            <Field
              label="Signing wallet"
              value={
                me.data?.wallet_address ? (
                  <a
                    className="font-mono text-violet-700 hover:underline"
                    href={solscanAccount(me.data.wallet_address, NETWORK)}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {formatAddress(me.data.wallet_address)}
                  </a>
                ) : (
                  "—"
                )
              }
            />
            <Field
              label="Credits"
              value={
                me.data ? (
                  <span className="font-mono">{formatUsdc(BigInt(me.data.credits_usdc))}</span>
                ) : (
                  "—"
                )
              }
            />
            <Field
              label="Member since"
              value={me.data ? formatRelativeTime(me.data.created_at) : "—"}
            />
          </div>
        </section>
        <section>
          <div className="flex items-center justify-between mb-2">
            <h2 className="text-[14px] font-semibold tracking-tight">API keys</h2>
            <Btn
              variant="primary"
              size="sm"
              icon={<Icon name="plus" className="w-3.5 h-3.5" />}
              onClick={() => setOpen(true)}
            >
              New key
            </Btn>
          </div>
          <ApiKeyList />
        </section>
        {open && <CreateApiKeyDialog onClose={() => setOpen(false)} />}
      </main>
    </>
  );
}

function Field({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div>
      <div className="text-[11px] uppercase font-mono text-ink-500">{label}</div>
      <div className="mt-0.5">{value}</div>
    </div>
  );
}
