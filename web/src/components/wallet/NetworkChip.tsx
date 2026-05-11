import { Pill } from "@/components/primitives/Pill";
import { NETWORK } from "@/lib/solana/connection";

export function NetworkChip() {
  return <Pill tone={NETWORK === "mainnet" ? "emerald" : "amber"}>{NETWORK}</Pill>;
}
