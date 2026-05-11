import { Pill, type PillTone } from "@/components/primitives/Pill";
import type { RunStatus } from "@/lib/api/schemas";

const TONE: Record<RunStatus, PillTone> = {
  Pending: "ink",
  Triggered: "ink",
  Simulating: "ink",
  Bundling: "violet",
  Submitted: "violet",
  Confirmed: "emerald",
  Retrying: "amber",
  Failed: "rose",
  Skipped: "ink",
};

export function RunStatusPill({ status }: { status: RunStatus }) {
  return <Pill tone={TONE[status]}>{status.toLowerCase()}</Pill>;
}
