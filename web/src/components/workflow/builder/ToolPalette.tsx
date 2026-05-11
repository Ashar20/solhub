"use client";
import { useState, useMemo } from "react";
import { REGISTRY } from "@/lib/plugins/registry";
import { Icon } from "@/components/primitives/Icon";
import { Pill } from "@/components/primitives/Pill";

export interface ToolPaletteProps {
  onAdd: (plugin: string, action: string) => void;
}

export function ToolPalette({ onAdd }: ToolPaletteProps) {
  const [q, setQ] = useState("");

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return REGISTRY
      .map((p) => ({
        ...p,
        actions: p.actions.filter((a) =>
          (`${p.name} ${p.id} ${a.name} ${a.id} ${a.description}`).toLowerCase().includes(needle),
        ),
      }))
      .filter((p) => p.actions.length > 0);
  }, [q]);

  return (
    <div className="h-full flex flex-col">
      <div className="h-10 px-3 border-b border-ink-200 flex items-center gap-2">
        <Icon name="search" className="w-3.5 h-3.5 text-ink-400" />
        <input
          placeholder="Search tools…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          className="flex-1 text-[12px] bg-transparent focus:outline-none"
        />
      </div>
      <div className="flex-1 overflow-y-auto scrollbar-thin p-2 space-y-3">
        {filtered.length === 0 && (
          <div className="text-[12px] text-ink-500 px-2">No tools match.</div>
        )}
        {filtered.map((p) => (
          <div key={p.id}>
            <div className="flex items-center justify-between px-2 mb-1">
              <span className="text-[10px] uppercase tracking-wider font-mono text-ink-500">{p.name}</span>
              {p.status === "stub" && <Pill tone="amber">stub</Pill>}
            </div>
            <ul>
              {p.actions.map((a) => (
                <li key={a.id}>
                  <button
                    type="button"
                    onClick={() => onAdd(p.id, a.id)}
                    className="w-full text-left px-2 py-1.5 rounded hover:bg-ink-50 text-[12px]"
                    aria-label={`Add ${p.name} ${a.name}`}
                  >
                    <div className="font-medium text-ink-900">{a.name}</div>
                    <div className="text-[11px] text-ink-500 line-clamp-1">{a.description}</div>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
