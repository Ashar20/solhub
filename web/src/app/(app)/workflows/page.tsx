"use client";
import { useState } from "react";
import Link from "next/link";
import { Topbar } from "@/components/shell/Topbar";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { WorkflowRow } from "@/components/workflow/WorkflowRow";

type Tab = "all" | "active" | "inactive";

export default function WorkflowsList() {
  const [tab, setTab] = useState<Tab>("all");
  const [q, setQ] = useState("");

  // Backend only supports `active_only: boolean`, so for "All" + "Paused" we fetch all and filter client-side.
  const { data, isLoading } = useWorkflows(
    tab === "active" ? { active_only: true } : {},
  );

  const filtered = (data ?? [])
    .filter((w) => tab === "inactive" ? !w.is_active : true)
    .filter((w) => q.trim() === "" ? true : w.name.toLowerCase().includes(q.toLowerCase()));

  const tabClass = (t: Tab) =>
    "h-8 px-3 text-[12px] font-medium rounded-md " +
    (tab === t ? "bg-ink-900 text-white" : "text-ink-600 hover:bg-ink-100");

  return (
    <>
      <Topbar
        crumbs={["Workspace", "solhub-prod", "Workflows"]}
        right={
          <Link href="/workflows/new">
            <Btn variant="primary" icon={<Icon name="plus" className="w-3.5 h-3.5" />}>
              New workflow
            </Btn>
          </Link>
        }
      />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-1 p-0.5 rounded-md bg-ink-100">
            <button onClick={() => setTab("all")} className={tabClass("all")}>All</button>
            <button onClick={() => setTab("active")} className={tabClass("active")}>Live</button>
            <button onClick={() => setTab("inactive")} className={tabClass("inactive")}>Paused</button>
          </div>
          <input
            placeholder="Filter…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="h-8 px-3 rounded-md border border-ink-200 text-[12px] w-64 focus:outline-none"
          />
        </div>
        <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          <div className="grid grid-cols-[1fr_140px_120px_140px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
            <div>Name</div><div>Status</div><div>Runs</div><div className="text-right">Updated</div>
          </div>
          {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
          {!isLoading && filtered.length === 0 && (
            <div className="p-6 text-[12px] text-ink-500">No workflows match.</div>
          )}
          {filtered.map((w) => <WorkflowRow key={w.id} w={w} />)}
        </div>
      </main>
    </>
  );
}
