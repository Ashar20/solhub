import { cn } from "@/lib/utils/cn";

export function SolhubLogo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div className="w-7 h-7 rounded-lg bg-ink-950 flex items-center justify-center relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-sol-purple/60 to-sol-green/40" />
        <span className="relative font-mono text-white text-[13px] font-semibold tracking-tight">sh</span>
      </div>
      <div className="leading-none">
        <div className="text-[15px] font-semibold tracking-tight">solhub</div>
        <div className="text-[9px] font-mono text-ink-500 uppercase tracking-[0.18em]">workflow os</div>
      </div>
    </div>
  );
}
