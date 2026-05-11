import * as React from "react";

export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex items-center justify-center min-w-[18px] h-[18px] px-1 rounded border border-ink-200 bg-white text-[10px] font-mono text-ink-600 shadow-sm">
      {children}
    </kbd>
  );
}
