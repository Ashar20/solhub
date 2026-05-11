"use client";
import { Topbar } from "@/components/shell/Topbar";
import { useRuns } from "@/lib/hooks/use-runs";
import { RunRow } from "@/components/runs/RunRow";

export default function RunsPage() {
  const { data, isLoading } = useRuns({ limit: 50 });
  return (
    <>
      <Topbar crumbs={["Workspace", "Operate", "Runs & Logs"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          <div className="grid grid-cols-[100px_1fr_120px_140px_140px_120px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
            <div>Status</div>
            <div>Workflow</div>
            <div>Started</div>
            <div>Slot</div>
            <div>Jito tip</div>
            <div className="text-right">Signature</div>
          </div>
          {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
          {!isLoading && (data ?? []).length === 0 && (
            <div className="p-6 text-[12px] text-ink-500">No runs yet.</div>
          )}
          {(data ?? []).map((r) => <RunRow key={r.run_id} r={r} />)}
        </div>
      </main>
    </>
  );
}
