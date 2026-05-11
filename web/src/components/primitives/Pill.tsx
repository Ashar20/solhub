import * as React from "react";
import { cn } from "@/lib/utils/cn";

const TONES = {
  ink: "bg-ink-100 text-ink-700 border-ink-200",
  violet: "bg-violet-50 text-violet-700 border-violet-200",
  emerald: "bg-emerald-50 text-emerald-700 border-emerald-200",
  amber: "bg-amber-50 text-amber-700 border-amber-200",
  rose: "bg-rose-50 text-rose-700 border-rose-200",
  cyan: "bg-cyan-50 text-cyan-700 border-cyan-200",
  sol: "bg-gradient-to-r from-sol-purple/10 to-sol-green/10 text-ink-900 border-ink-200",
} as const;

export type PillTone = keyof typeof TONES;

export function Pill({
  children, tone = "ink", className,
}: { children: React.ReactNode; tone?: PillTone; className?: string }) {
  return (
    <span className={cn(
      "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-[11px] font-medium font-mono",
      TONES[tone], className,
    )}>{children}</span>
  );
}
