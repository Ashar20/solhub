import { AnchorProvider, Program, type Idl } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import type { AnchorWallet } from "@solana/wallet-adapter-react";
import idl from "./idl/execution-vault.json";

// In Anchor v0.32 the program address is embedded in the IDL's `address` field.
// Optionally allow override via env.
const IDL_ADDRESS =
  (idl as { address?: string }).address ??
  "4CFgDzuLnfdTThgNXTknhXyshzsidDQFtNCxsoMnBHJn";

const PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_EXECUTION_VAULT_PROGRAM_ID ?? IDL_ADDRESS,
);

export { PROGRAM_ID as EXECUTION_VAULT_PROGRAM_ID };

export function executionVaultProgram(
  connection: Connection,
  wallet: AnchorWallet,
): Program {
  const provider = new AnchorProvider(
    connection,
    wallet,
    AnchorProvider.defaultOptions(),
  );
  // Anchor v0.32: constructor is (idl, provider?, coder?)
  // The IDL's `address` field determines the program ID.
  return new Program(idl as Idl, provider);
}

/** Derive the vault PDA from a depositor pubkey: seeds [b"vault", depositor]. */
export function findVaultPda(depositor: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), depositor.toBuffer()],
    PROGRAM_ID,
  );
}

/** Derive the creator account PDA: seeds [b"creator", owner]. */
export function findCreatorPda(owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("creator"), owner.toBuffer()],
    PROGRAM_ID,
  );
}
