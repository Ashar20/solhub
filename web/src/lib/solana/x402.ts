import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import type { PaymentRequirements } from "@/lib/api/hub";

const MEMO_PROGRAM_ID = new PublicKey("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

export function buildMemoIx(payer: PublicKey, memo: string): TransactionInstruction {
  return new TransactionInstruction({
    keys: [{ pubkey: payer, isSigner: true, isWritable: false }],
    programId: MEMO_PROGRAM_ID,
    data: Buffer.from(memo, "utf8"),
  });
}

export async function buildPaymentTx(
  connection: Connection,
  payer: PublicKey,
  reqs: PaymentRequirements,
): Promise<Transaction> {
  const recipient = new PublicKey(reqs.recipient);
  const transferIx = SystemProgram.transfer({
    fromPubkey: payer,
    toPubkey: recipient,
    lamports: reqs.amount_lamports,
  });
  const memoIx = buildMemoIx(payer, reqs.memo);
  const tx = new Transaction().add(transferIx, memoIx);
  tx.feePayer = payer;
  const { blockhash } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  return tx;
}
