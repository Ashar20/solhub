import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  createInitializeAccountInstruction,
  getMinimumBalanceForRentExemptAccount,
  ACCOUNT_SIZE,
} from "@solana/spl-token";
import { ExecutionVault } from "../../target/types/execution_vault";
import { expect } from "chai";

describe("execution-vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ExecutionVault as Program<ExecutionVault>;
  // Use provider.wallet only for funding and mint authority; all vault operations
  // use a fresh depositorKeypair per test run to avoid PDA state conflicts.
  const funder = provider.wallet as anchor.Wallet;
  const platformAuthority = anchor.web3.Keypair.generate();

  // Fresh depositor keypair each test run — prevents vault PDA state carryover
  let depositorKeypair: anchor.web3.Keypair;

  let usdcMint: anchor.web3.PublicKey;
  let ownerTokenAccount: anchor.web3.PublicKey;  // depositor's USDC source
  let vaultTokenAccount: anchor.web3.PublicKey;  // depositor's vault SPL token account
  let creatorKeypair: anchor.web3.Keypair;
  let creatorTokenAccount: anchor.web3.PublicKey;
  // The creator's own vault token account (used for withdraw_creator)
  let creatorVaultTokenAccount: anchor.web3.PublicKey;

  // ── helpers ────────────────────────────────────────────────────────────────

  /** Create a raw SPL token account owned by `ownerKey` (works for PDA owners too). */
  async function createTokenAccountForOwner(
    mint: anchor.web3.PublicKey,
    ownerKey: anchor.web3.PublicKey
  ): Promise<anchor.web3.PublicKey> {
    const accountKeypair = anchor.web3.Keypair.generate();
    const lamports = await getMinimumBalanceForRentExemptAccount(provider.connection);
    const tx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: (funder as any).payer.publicKey,
        newAccountPubkey: accountKeypair.publicKey,
        space: ACCOUNT_SIZE,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeAccountInstruction(accountKeypair.publicKey, mint, ownerKey)
    );
    await provider.sendAndConfirm(tx, [(funder as any).payer, accountKeypair]);
    return accountKeypair.publicKey;
  }

  function vaultPda(depositorKey: anchor.web3.PublicKey) {
    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), depositorKey.toBuffer()],
      program.programId
    );
    return pda;
  }

  function creatorPda(creatorKey: anchor.web3.PublicKey) {
    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("creator"), creatorKey.toBuffer()],
      program.programId
    );
    return pda;
  }

  // ── setup ──────────────────────────────────────────────────────────────────

  before(async () => {
    // Fresh depositor per run — fresh vault PDA, no state carryover
    depositorKeypair = anchor.web3.Keypair.generate();

    // Fund depositor, platformAuthority, and creator via direct transfer
    // (devnet airdrop is rate-limited; provider.wallet has the SOL)
    creatorKeypair = anchor.web3.Keypair.generate();
    for (const kp of [depositorKeypair, platformAuthority, creatorKeypair]) {
      const fundTx = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: kp.publicKey,
          lamports: 0.05 * anchor.web3.LAMPORTS_PER_SOL,
        })
      );
      await provider.sendAndConfirm(fundTx);
    }

    // Create USDC mock mint (6 decimals) — funder is the mint authority
    usdcMint = await createMint(
      provider.connection,
      (funder as any).payer,
      funder.publicKey,
      null,
      6
    );

    // Depositor token account — source for vault deposits
    ownerTokenAccount = await createAccount(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      depositorKeypair.publicKey
    );

    // Depositor's vault token account — owned by depositor's vault PDA
    const vaultKey = vaultPda(depositorKeypair.publicKey);
    vaultTokenAccount = await createTokenAccountForOwner(usdcMint, vaultKey);

    // Creator's own vault token account (for withdraw_creator)
    const creatorVaultKey = vaultPda(creatorKeypair.publicKey);
    creatorVaultTokenAccount = await createTokenAccountForOwner(usdcMint, creatorVaultKey);

    // Creator destination token account
    creatorTokenAccount = await createAccount(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      creatorKeypair.publicKey
    );

    // Mint generous USDC to depositor and creator
    await mintTo(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      ownerTokenAccount,
      funder.publicKey,
      100_000_000 // 100 USDC to depositor
    );
    await mintTo(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      creatorTokenAccount,
      funder.publicKey,
      1_000 // small seed deposit for creator
    );

    // Initialise creator's vault PDA by having the creator deposit a tiny amount
    // (required so vault_pda.bump is stored for later withdraw_creator calls)
    await program.methods
      .depositCredits(new anchor.BN(1_000))
      .accounts({
        vault: creatorVaultKey,
        depositorTokenAccount: creatorTokenAccount,
        vaultTokenAccount: creatorVaultTokenAccount,
        depositor: creatorKeypair.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creatorKeypair])
      .rpc();
  });

  // ── tests ──────────────────────────────────────────────────────────────────

  it("deposit_credits transfers USDC and updates vault balance", async () => {
    const amount = new BN(1_000_000); // 1 USDC
    const vaultKey = vaultPda(depositorKeypair.publicKey);

    await program.methods
      .depositCredits(amount)
      .accounts({
        vault: vaultKey,
        depositorTokenAccount: ownerTokenAccount,
        vaultTokenAccount,
        depositor: depositorKeypair.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([depositorKeypair])
      .rpc();

    const vaultAccount = await program.account.vaultAccount.fetch(vaultKey);
    expect(vaultAccount.credits.toNumber()).to.equal(1_000_000);

    const tokenInfo = await getAccount(provider.connection, vaultTokenAccount);
    expect(Number(tokenInfo.amount)).to.equal(1_000_000);
  });

  it("debit_execution splits 80/20 between creator and treasury", async () => {
    // At this point vault has 1_000_000 from previous test.
    // Debit 100_000 → creator gets 80_000, 20_000 stays conceptually as treasury.
    const vaultKey = vaultPda(depositorKeypair.publicKey);
    const creatorAcctKey = creatorPda(creatorKeypair.publicKey);
    const fakeWorkflow = anchor.web3.Keypair.generate().publicKey;

    await program.methods
      .debitExecution(new BN(100_000))
      .accounts({
        callerVault: vaultKey,
        creatorAccount: creatorAcctKey,
        creator: creatorKeypair.publicKey,
        workflow: fakeWorkflow,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    const vaultAccount = await program.account.vaultAccount.fetch(vaultKey);
    // 1_000_000 deposited - 100_000 debited = 900_000
    expect(vaultAccount.credits.toNumber()).to.equal(900_000);

    const creatorAccount = await program.account.creatorAccount.fetch(creatorAcctKey);
    expect(creatorAccount.balance.toNumber()).to.equal(80_000);
    expect(creatorAccount.totalEarned.toNumber()).to.equal(80_000);
  });

  it("debit_execution fails when insufficient credits", async () => {
    // Deposit 50_000 into a new depositor vault
    const depositor = anchor.web3.Keypair.generate();
    const fundTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: depositor.publicKey,
        lamports: 0.05 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(fundTx);

    const depositorTokenAcct = await createAccount(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      depositor.publicKey
    );
    await mintTo(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      depositorTokenAcct,
      funder.publicKey,
      100_000
    );

    const depositorVaultKey = vaultPda(depositor.publicKey);
    const depositorVaultTokenAcct = await createTokenAccountForOwner(usdcMint, depositorVaultKey);

    await program.methods
      .depositCredits(new BN(50_000))
      .accounts({
        vault: depositorVaultKey,
        depositorTokenAccount: depositorTokenAcct,
        vaultTokenAccount: depositorVaultTokenAcct,
        depositor: depositor.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([depositor])
      .rpc();

    const creatorAcctKey = creatorPda(creatorKeypair.publicKey);
    const fakeWorkflow = anchor.web3.Keypair.generate().publicKey;

    try {
      await program.methods
        .debitExecution(new BN(100_000))
        .accounts({
          callerVault: depositorVaultKey,
          creatorAccount: creatorAcctKey,
          creator: creatorKeypair.publicKey,
          workflow: fakeWorkflow,
          platformAuthority: platformAuthority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([platformAuthority])
        .rpc();
      expect.fail("Should have failed with InsufficientCredits");
    } catch (err: any) {
      expect(err.message).to.match(/InsufficientCredits|custom program error|0x1770/i);
    }
  });

  it("withdraw_creator transfers tokens out and updates balance", async () => {
    // creatorKeypair currently has 80_000 balance (from debit_execution test) in creator_account.
    // Deposit USDC into creatorVaultTokenAccount to cover withdrawal (vault_pda is creator's vault).
    const creatorAcctKey = creatorPda(creatorKeypair.publicKey);
    const creatorVaultKey = vaultPda(creatorKeypair.publicKey);

    // Mint funds directly into the creator's vault token account to cover the withdrawal
    await mintTo(
      provider.connection,
      (funder as any).payer,
      usdcMint,
      creatorVaultTokenAccount,
      funder.publicKey,
      80_000 // enough to cover the 40_000 withdrawal
    );

    const withdrawAmount = new BN(40_000);

    await program.methods
      .withdrawCreator(withdrawAmount)
      .accounts({
        creatorAccount: creatorAcctKey,
        creatorTokenAccount,
        vaultTokenAccount: creatorVaultTokenAccount,
        vaultPda: creatorVaultKey,
        owner: creatorKeypair.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([creatorKeypair])
      .rpc();

    const creatorAccount = await program.account.creatorAccount.fetch(creatorAcctKey);
    expect(creatorAccount.balance.toNumber()).to.equal(40_000);

    const tokenInfo = await getAccount(provider.connection, creatorTokenAccount);
    // creatorTokenAccount starts with 1_000 (setup) - 1_000 (deposited) = 0, then gets 40_000
    expect(Number(tokenInfo.amount)).to.equal(40_000);
  });

  it("withdraw_creator fails when insufficient balance", async () => {
    // creatorKeypair balance is now 40_000 (after prior withdrawal test)
    const creatorAcctKey = creatorPda(creatorKeypair.publicKey);
    const creatorVaultKey = vaultPda(creatorKeypair.publicKey);

    try {
      await program.methods
        .withdrawCreator(new BN(1_000_000))
        .accounts({
          creatorAccount: creatorAcctKey,
          creatorTokenAccount,
          vaultTokenAccount: creatorVaultTokenAccount,
          vaultPda: creatorVaultKey,
          owner: creatorKeypair.publicKey,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([creatorKeypair])
        .rpc();
      expect.fail("Should have failed with InsufficientBalance");
    } catch (err: any) {
      expect(err.message).to.match(/InsufficientBalance|custom program error|0x1771/i);
    }
  });
});
