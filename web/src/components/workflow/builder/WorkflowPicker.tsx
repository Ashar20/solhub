"use client";
import { useMemo, useState, useRef, useEffect } from "react";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { Icon } from "@/components/primitives/Icon";
import { Pill } from "@/components/primitives/Pill";
import { cn } from "@/lib/utils/cn";

export interface WorkflowPickerProps {
  value: string;
  onChange: (id: string) => void;
  /** Workflow id to exclude from the list (the current workflow being edited). */
  excludeId?: string;
}

export function WorkflowPicker({ value, onChange, excludeId }: WorkflowPickerProps) {
  const { data, isLoading } = useWorkflows();
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    function onDoc(e: MouseEvent) {
      if (!rootRef.current) return;
      if (!rootRef.current.contains(e.target as Node)) setOpen(false);
    }
    if (open) document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const candidates = useMemo(() => {
    const list = (data ?? []).filter((w) => w.id !== excludeId);
    const needle = q.trim().toLowerCase();
    if (!needle) return list;
    return list.filter((w) => w.name.toLowerCase().includes(needle));
  }, [data, excludeId, q]);

  const selected = (data ?? []).find((w) => w.id === value);

  return (
    <div ref={rootRef} className="relative">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className={cn(
          "w-full flex items-center justify-between h-8 px-2 rounded-md border border-ink-200 text-[13px] bg-white",
          "hover:border-ink-300",
        )}
      >
        <span className={selected ? "text-ink-900 truncate" : "text-ink-500"}>
          {selected ? selected.name : isLoading ? "Loading workflows…" : "Select workflow…"}
        </span>
        <Icon name="chevronDown" className="w-3.5 h-3.5 text-ink-500 shrink-0" />
      </button>

      {open && (
        <div className="absolute z-50 mt-1 w-full rounded-md border border-ink-200 bg-white shadow-pop overflow-hidden">
          <div className="h-9 px-2 border-b border-ink-200 flex items-center gap-1.5">
            <Icon name="search" className="w-3.5 h-3.5 text-ink-400" />
            <input
              autoFocus
              placeholder="Filter…"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="flex-1 text-[12px] bg-transparent focus:outline-none"
            />
          </div>
          <ul className="max-h-60 overflow-y-auto scrollbar-thin">
            {candidates.length === 0 && (
              <li className="px-3 py-2 text-[12px] text-ink-500">
                {excludeId && (data ?? []).some((w) => w.id === excludeId) && (data ?? []).length === 1
                  ? "No other workflows to call yet."
                  : "No workflows match."}
              </li>
            )}
            {candidates.map((w) => (
              <li key={w.id}>
                <button
                  type="button"
                  onClick={() => { onChange(w.id); setOpen(false); setQ(""); }}
                  className="w-full flex items-center justify-between px-3 py-2 text-left text-[12px] hover:bg-ink-50"
                >
                  <span className="truncate">{w.name}</span>
                  <Pill tone={w.is_active ? "emerald" : "amber"}>{w.is_active ? "live" : "paused"}</Pill>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
