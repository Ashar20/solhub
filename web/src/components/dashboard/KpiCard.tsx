import { cn } from "@/lib/utils/cn";

export function KpiCard({ label, value, sub, className }: {
  label: string; value: React.ReactNode; sub?: React.ReactNode; className?: string;
}) {
  return (
    <div className={cn("rounded-xl border border-ink-200 bg-white shadow-card p-4", className)}>
      <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500">{label}</div>
      <div className="mt-1 text-[22px] font-semibold tracking-tight text-ink-900">{value}</div>
      {sub && <div className="mt-1 text-[11px] text-ink-500">{sub}</div>}
    </div>
  );
}
