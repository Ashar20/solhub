import type { StepLog } from "@/lib/api/schemas";
import { Pill, type PillTone } from "@/components/primitives/Pill";

const TONE: Record<StepLog["status"], PillTone> = {
  Pending: "ink",
  Running: "violet",
  Success: "emerald",
  Completed: "emerald",
  Failed: "rose",
  Skipped: "ink",
  WaitingApproval: "amber",
};

export function StepTimeline({ steps }: { steps: StepLog[] }) {
  if (steps.length === 0) {
    return <div className="text-[12px] text-ink-500 p-4">No steps recorded yet.</div>;
  }
  return (
    <ol className="space-y-2">
      {steps.map((s, i) => (
        <li key={s.step_id} className="flex items-start gap-3 rounded-lg border border-ink-200 bg-white p-3">
          <div className="w-5 h-5 rounded-full bg-ink-100 text-[10px] font-mono flex items-center justify-center text-ink-700">
            {i + 1}
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <div className="text-[13px] font-medium">{s.step_id}</div>
              <Pill tone={TONE[s.status]}>{s.status.toLowerCase()}</Pill>
              <span className="text-[11px] text-ink-500 font-mono">{s.duration_ms} ms</span>
            </div>
            {s.error && (
              <pre className="mt-1 text-[11px] font-mono text-rose-700 whitespace-pre-wrap">{s.error}</pre>
            )}
          </div>
        </li>
      ))}
    </ol>
  );
}
