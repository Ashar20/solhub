import { Pill } from "@/components/primitives/Pill";

export function StatusPill({ active }: { active: boolean }) {
  return active ? <Pill tone="emerald">live</Pill> : <Pill tone="amber">paused</Pill>;
}
