"use client";
import { useState } from "react";
import Link from "next/link";
import { Topbar } from "@/components/shell/Topbar";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";

export default function VersionsPage() {
  const { data, isLoading } = useWorkflows();
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <>
      <Topbar crumbs={["Operate", "Versions"]} />
      <main className="flex-1 grid grid-cols-[320px_1fr] gap-4 p-6 overflow-hidden">
        <aside className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden flex flex-col min-h-0">
          <div className="h-9 px-3 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500 flex items-center">
            Workflows
          </div>
          <ul className="overflow-y-auto scrollbar-thin flex-1">
            {isLoading && (
              <li className="p-3 text-[12px] text-ink-500">Loading&hellip;</li>
            )}
            {!isLoading && (data ?? []).length === 0 && (
              <li className="p-3 text-[12px] text-ink-500">No workflows yet.</li>
            )}
            {(data ?? []).map((w) => (
              <li key={w.id}>
                <button
                  type="button"
                  onClick={() => setSelected(w.id)}
                  className={
                    "w-full text-left px-3 py-2 text-[13px] border-b border-ink-100 hover:bg-ink-50 " +
                    (selected === w.id ? "bg-ink-100 font-medium" : "")
                  }
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate">{w.name}</span>
                    <Pill tone={w.is_active ? "emerald" : "amber"}>
                      {w.is_active ? "live" : "paused"}
                    </Pill>
                  </div>
                  <div className="text-[10px] font-mono text-ink-500 mt-0.5">
                    {w.execution_count} runs
                  </div>
                </button>
              </li>
            ))}
          </ul>
        </aside>
        <section className="rounded-xl border border-ink-200 bg-white shadow-card p-6 flex flex-col">
          {!selected ? (
            <div className="m-auto text-center max-w-md">
              <Icon name="git" className="w-8 h-8 text-ink-300 mx-auto mb-3" />
              <h3 className="text-[14px] font-semibold tracking-tight mb-1">Versions</h3>
              <p className="text-[12px] text-ink-500 leading-relaxed">
                Select a workflow on the left to view its version history.
              </p>
            </div>
          ) : (
            <div className="m-auto text-center max-w-md">
              <Icon name="info" className="w-8 h-8 text-ink-300 mx-auto mb-3" />
              <h3 className="text-[14px] font-semibold tracking-tight mb-1">
                Version history not yet wired
              </h3>
              <p className="text-[12px] text-ink-500 leading-relaxed mb-3">
                The backend does not yet expose workflow versions. When it does, this panel will
                show every revision, the diff between revisions, and a rollback action.
              </p>
              <Link
                href={`/workflows/${selected}`}
                className="inline-flex items-center gap-1 text-[12px] text-violet-700 hover:underline"
              >
                Open current revision in builder
                <Icon name="arrow" className="w-3 h-3" />
              </Link>
            </div>
          )}
        </section>
      </main>
    </>
  );
}
