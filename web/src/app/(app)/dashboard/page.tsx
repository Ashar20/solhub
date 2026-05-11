"use client";
import { Topbar } from "@/components/shell/Topbar";
import { KpiCard } from "@/components/dashboard/KpiCard";
import { RecentList } from "@/components/dashboard/RecentList";
import { useAnalytics } from "@/lib/hooks/use-analytics";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { useRuns } from "@/lib/hooks/use-runs";
import { useMe } from "@/lib/hooks/use-org";
import { formatLamports, formatUsdc, formatRelativeTime } from "@/lib/utils/format";

function successRate(successful: number, total: number): string {
  if (total === 0) return "—";
  return `${((successful / total) * 100).toFixed(1)}%`;
}

export default function Dashboard() {
  const analytics = useAnalytics();
  const workflows = useWorkflows({ limit: 5 });
  const runs = useRuns({ limit: 10 });
  const me = useMe();

  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "Dashboard"]} />
      <main className="flex-1 p-6 grid-bg overflow-y-auto">
        <div className="grid grid-cols-4 gap-3 mb-4">
          <KpiCard
            label="Executions"
            value={analytics.data?.total_executions ?? "—"}
          />
          <KpiCard
            label="Success rate"
            value={analytics.data
              ? successRate(analytics.data.successful, analytics.data.total_executions)
              : "—"}
            sub={analytics.data ? `${analytics.data.successful}/${analytics.data.total_executions} succeeded` : undefined}
          />
          <KpiCard
            label="Fee spend"
            value={analytics.data ? formatLamports(BigInt(analytics.data.total_fee_lamports)) : "—"}
            sub={analytics.data && analytics.data.failed > 0 ? `${analytics.data.failed} failed` : undefined}
          />
          <KpiCard
            label="Credits"
            value={me.data ? formatUsdc(BigInt(me.data.credits_usdc)) : "—"}
          />
        </div>
        <div className="grid grid-cols-2 gap-4">
          <RecentList
            title="Recent workflows"
            emptyText={workflows.isLoading ? "Loading…" : "No workflows yet."}
            items={(workflows.data ?? []).map((w) => ({
              id: w.id,
              primary: w.name,
              secondary: `${w.trigger_type} · ${w.is_active ? "active" : "paused"}`,
              href: `/workflows/${w.id}`,
            }))}
          />
          <RecentList
            title="Recent runs"
            emptyText={runs.isLoading ? "Loading…" : "No runs yet."}
            items={(runs.data ?? []).map((r) => ({
              id: r.run_id,
              primary: r.status,
              secondary: formatRelativeTime(r.started_at),
              href: `/runs/${r.run_id}`,
            }))}
          />
        </div>
      </main>
    </>
  );
}
