import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { BN, type Program } from "@coral-xyz/anchor";
import { USDC_MINT } from "@/lib/hooks/use-balances";
import { findVaultPda, findCreatorPda } from "./programs";

/**
 * Build a deposit_credits transaction.
 *
 * IDL account order (exact):
 *   vault, depositorTokenAccount, vaultTokenAccount, depositor,
 *   tokenProgram, systemProgram
 *
 * Note: the deposit instruction transfers USDC from depositor's ATA to the
 * vault's ATA.  The vault ATA is owned by the vault PDA (allowOwnerOffCurve=true).
 */
export async function buildDepositTx(
  program: Program,
  depositor: PublicKey,
  amountMicroUsdc: bigint,
): Promise<Transaction> {
  const [vault] = findVaultPda(depositor);
  const depositorTokenAccount = getAssociatedTokenAddressSync(
    USDC_MINT,
    depositor,
    false,
    TOKEN_PROGRAM_ID,
  );
  const vaultTokenAccount = getAssociatedTokenAddressSync(
    USDC_MINT,
    vault,
    true,
    TOKEN_PROGRAM_ID,
  );

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const ix = await (program.methods as any)
    .depositCredits(new BN(amountMicroUsdc.toString()))
    .accounts({
      vault,
      depositorTokenAccount,
      vaultTokenAccount,
      depositor,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();

  return new Transaction().add(ix);
}

/**
 * Build a withdraw_creator transaction.
 *
 * IDL account order (exact):
 *   creatorAccount, creatorTokenAccount, vaultTokenAccount, vaultPda,
 *   owner, tokenProgram
 *
 * Note: creator's token account is the destination; vault token account is
 * the source; the vault PDA signs via seeds.
 */
export async function buildWithdrawTx(
  program: Program,
  owner: PublicKey,
  amountMicroUsdc: bigint,
): Promise<Transaction> {
  const [creatorAccount] = findCreatorPda(owner);
  const [vaultPda] = findVaultPda(owner);
  const creatorTokenAccount = getAssociatedTokenAddressSync(
    USDC_MINT,
    owner,
    false,
    TOKEN_PROGRAM_ID,
  );
  const vaultTokenAccount = getAssociatedTokenAddressSync(
    USDC_MINT,
    vaultPda,
    true,
    TOKEN_PROGRAM_ID,
  );

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const ix = await (program.methods as any)
    .withdrawCreator(new BN(amountMicroUsdc.toString()))
    .accounts({
      creatorAccount,
      creatorTokenAccount,
      vaultTokenAccount,
      vaultPda,
      owner,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();

  return new Transaction().add(ix);
}

/**
 * Fetch the creator account on-chain and return its accumulated balance
 * in micro-USDC.  Returns 0n if the account does not exist yet.
 */
export async function fetchCreatorBalance(
  program: Program,
  owner: PublicKey,
): Promise<bigint> {
  const [pda] = findCreatorPda(owner);
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const acc = await (program.account as any).creatorAccount.fetch(pda);
    return BigInt(acc.balance.toString());
  } catch {
    return 0n;
  }
}
