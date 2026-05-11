import { Topbar } from "@/components/shell/Topbar";

export default function DashboardPlaceholder() {
  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "Dashboard"]} />
      <main className="flex-1 p-6 grid-bg">
        <div className="text-[13px] text-ink-500">
          Phase A scaffolding complete. Real screens land in Phase B.
        </div>
      </main>
    </>
  );
}
