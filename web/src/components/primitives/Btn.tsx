import * as React from "react";
import { cn } from "@/lib/utils/cn";

const SIZES = {
  sm: "h-7 px-2.5 text-[12px]",
  md: "h-8 px-3 text-[13px]",
  lg: "h-10 px-4 text-[14px]",
} as const;

const VARIANTS = {
  default: "bg-white hover:bg-ink-50 border-ink-200 text-ink-900",
  primary: "bg-ink-950 hover:bg-ink-900 border-ink-950 text-white",
  accent: "bg-violet-600 hover:bg-violet-700 border-violet-600 text-white",
  success: "bg-emerald-600 hover:bg-emerald-700 border-emerald-600 text-white",
  ghost: "bg-transparent hover:bg-ink-100 border-transparent text-ink-700",
  danger: "bg-white hover:bg-rose-50 border-rose-200 text-rose-700",
} as const;

export interface BtnProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof VARIANTS;
  size?: keyof typeof SIZES;
  icon?: React.ReactNode;
}

export function Btn({ children, variant = "default", size = "md", icon, className, ...rest }: BtnProps) {
  return (
    <button
      {...rest}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md border font-medium transition-colors",
        SIZES[size],
        VARIANTS[variant],
        className,
      )}
    >
      {icon}
      {children}
    </button>
  );
}
