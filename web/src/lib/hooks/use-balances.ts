"use client";
import { useQuery } from "@tanstack/react-query";
import { useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { connection, NETWORK } from "@/lib/solana/connection";

// USDC mints (verified on-chain).
const USDC_MINT_MAINNET = new PublicKey("EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v");
const USDC_MINT_DEVNET = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
export const USDC_MINT = NETWORK === "devnet" ? USDC_MINT_DEVNET : USDC_MINT_MAINNET;

export function useSolBalance() {
  const { publicKey } = useWallet();
  return useQuery({
    queryKey: ["balance", "sol", publicKey?.toBase58() ?? null] as const,
    queryFn: async (): Promise<bigint> => {
      if (!publicKey) return 0n;
      const lamports = await connection.getBalance(publicKey);
      return BigInt(lamports);
    },
    enabled: !!publicKey,
  });
}

export function useUsdcBalance() {
  const { publicKey } = useWallet();
  return useQuery({
    queryKey: ["balance", "usdc", publicKey?.toBase58() ?? null] as const,
    queryFn: async (): Promise<bigint> => {
      if (!publicKey) return 0n;
      const ata = getAssociatedTokenAddressSync(USDC_MINT, publicKey, false, TOKEN_PROGRAM_ID);
      try {
        const info = await connection.getTokenAccountBalance(ata);
        return BigInt(info.value.amount);
      } catch {
        return 0n;
      }
    },
    enabled: !!publicKey,
  });
}
