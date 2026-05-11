import { Connection, clusterApiUrl } from "@solana/web3.js";

export type SolanaNetwork = "mainnet" | "devnet";

export const NETWORK: SolanaNetwork =
  (process.env.NEXT_PUBLIC_SOLANA_NETWORK as SolanaNetwork) ?? "devnet";

const RPC_URL =
  process.env.NEXT_PUBLIC_SOLANA_RPC_URL ??
  clusterApiUrl(NETWORK === "mainnet" ? "mainnet-beta" : "devnet");

export const connection = new Connection(RPC_URL, "confirmed");
