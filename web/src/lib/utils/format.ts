export function formatLamports(lamports: bigint): string {
  if (lamports === 0n) return "0 SOL";
  const sol = Number(lamports) / 1e9;
  if (sol < 0.001) return `${sol.toFixed(9).replace(/0+$/, "").replace(/\.$/, "")} SOL`;
  return `${parseFloat(sol.toFixed(6))} SOL`;
}

export function formatUsdc(microUsdc: bigint): string {
  const usdc = Number(microUsdc) / 1e6;
  return `${usdc.toFixed(2)} USDC`;
}

export function formatAddress(addr: string, head = 5, tail = 4): string {
  if (addr.length <= head + tail + 1) return addr;
  return `${addr.slice(0, head)}…${addr.slice(-tail)}`;
}

export function formatSlot(slot: number | bigint): string {
  return Number(slot).toLocaleString("en-US");
}

export function formatRelativeTime(d: Date | string): string {
  const date = typeof d === "string" ? new Date(d) : d;
  const diffMs = Date.now() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 60) return "just now";
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

export type SolanaNetwork = "mainnet" | "devnet";

export function solscanTx(sig: string, network: SolanaNetwork = "mainnet"): string {
  const suffix = network === "devnet" ? "?cluster=devnet" : "";
  return `https://solscan.io/tx/${sig}${suffix}`;
}

export function solscanAccount(addr: string, network: SolanaNetwork = "mainnet"): string {
  const suffix = network === "devnet" ? "?cluster=devnet" : "";
  return `https://solscan.io/account/${addr}${suffix}`;
}
