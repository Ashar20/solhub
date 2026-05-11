import { Pill } from "@/components/primitives/Pill";
import { formatAddress, solscanAccount } from "@/lib/utils/format";
import type { SolanaNetwork } from "@/lib/utils/format";

export interface BalanceCardProps {
  title: string;
  subtitle?: string;
  address: string | null;
  lines: { label: string; value: React.ReactNode }[];
  network?: SolanaNetwork;
  footer?: React.ReactNode;
}

export function BalanceCard({
  title, subtitle, address, lines, network = "devnet", footer,
}: BalanceCardProps) {
  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 flex flex-col">
      <div className="flex items-center justify-between mb-2 gap-2">
        <div className="min-w-0">
          <div className="text-[14px] font-semibold tracking-tight">{title}</div>
          {subtitle && <div className="text-[11px] text-ink-500">{subtitle}</div>}
        </div>
        {address && (
          <a
            href={solscanAccount(address, network)}
            target="_blank"
            rel="noreferrer"
            className="shrink-0"
          >
            <Pill tone="ink">{formatAddress(address)}</Pill>
          </a>
        )}
      </div>
      <div className="grid grid-cols-2 gap-2 mt-2">
        {lines.map((l) => (
          <div key={l.label}>
            <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500">{l.label}</div>
            <div className="text-[15px] font-medium font-mono mt-0.5">{l.value}</div>
          </div>
        ))}
      </div>
      {footer && <div className="mt-3 pt-3 border-t border-ink-100">{footer}</div>}
    </div>
  );
}
