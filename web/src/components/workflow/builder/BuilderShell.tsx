"use client";
import { useEffect, useState } from "react";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";

export interface BuilderShellProps { id: string }

export function BuilderShell({ id }: BuilderShellProps) {
  const isNew = id === "new";
  const { data } = useWorkflow(isNew ? undefined : id);
  const [name, setName] = useState("Untitled workflow");

  useEffect(() => { if (data?.name) setName(data.name); }, [data?.name]);

  return (
    <div className="h-screen flex flex-col">
      <header className="h-12 border-b border-ink-200 bg-white flex items-center justify-between px-4">
        <div className="flex items-center gap-3 min-w-0">
          <SolhubLogo />
          <span className="text-ink-300">/</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Untitled workflow"
            className="text-[13px] font-medium bg-transparent focus:outline-none px-2 py-1 hover:bg-ink-50 rounded min-w-[200px]"
          />
          {!isNew && (data?.is_active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">draft</Pill>)}
          {isNew && <Pill tone="ink">new</Pill>}
        </div>
        <div className="flex items-center gap-2">
          <Btn variant="default" size="sm" disabled>Test run</Btn>
          <Btn variant="primary" size="sm" disabled>Save</Btn>
          <Btn variant="success" size="sm" disabled>Publish</Btn>
        </div>
      </header>
      <div className="flex-1 grid grid-cols-[240px_1fr_320px] overflow-hidden">
        <aside className="border-r border-ink-200 bg-white flex items-center justify-center text-[11px] text-ink-400 font-mono">
          tool palette (Task 7)
        </aside>
        <main className="bg-ink-50 flex items-center justify-center text-[11px] text-ink-400 font-mono">
          canvas (Task 6)
        </main>
        <aside className="border-l border-ink-200 bg-white flex items-center justify-center text-[11px] text-ink-400 font-mono">
          inspector (Task 8)
        </aside>
      </div>
    </div>
  );
}
