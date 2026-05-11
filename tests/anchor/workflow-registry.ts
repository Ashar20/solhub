import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { WorkflowRegistry } from "../../target/types/workflow_registry";
import { expect } from "chai";
import * as crypto from "crypto";

describe("workflow-registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.WorkflowRegistry as Program<WorkflowRegistry>;
  const owner = provider.wallet as anchor.Wallet;
  const platformAuthority = anchor.web3.Keypair.generate();

  // Unique suffix per test run so PDA names don't collide across runs
  const runId = Date.now().toString(36);

  // Fund the platformAuthority via direct transfer from provider wallet
  // (devnet airdrop is rate-limited; provider.wallet already has SOL)
  before(async () => {
    const tx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: platformAuthority.publicKey,
        lamports: 0.02 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(tx);
  });

  // ── helpers ────────────────────────────────────────────────────────────────

  function workflowPda(ownerKey: anchor.web3.PublicKey, name: string) {
    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("workflow"), ownerKey.toBuffer(), Buffer.from(name)],
      program.programId
    );
    return pda;
  }

  function dummyStepsHash(): number[] {
    return Array.from(crypto.randomBytes(32));
  }

  // ── tests ──────────────────────────────────────────────────────────────────

  it("registers a workflow and verifies PDA state", async () => {
    const name = `my-workflow-1-${runId}`;
    const triggerType = 0;
    const stepsHash = dummyStepsHash();
    const stepsCid = "QmExampleCid1234567890abcdef1234567890abcdef12";

    const wfPda = workflowPda(owner.publicKey, name);

    await program.methods
      .registerWorkflow({ name, triggerType, stepsHash, stepsCid })
      .accounts({
        workflow: wfPda,
        owner: owner.publicKey,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    const account = await program.account.workflowAccount.fetch(wfPda);

    expect(account.owner.toBase58()).to.equal(owner.publicKey.toBase58());
    expect(account.name).to.equal(name);
    expect(account.triggerType).to.equal(triggerType);
    expect(account.stepsHash).to.deep.equal(stepsHash);
    expect(account.stepsCid).to.equal(stepsCid);
    expect(account.isActive).to.be.true;
    expect(account.executionCount.toNumber()).to.equal(0);
    expect(account.createdAt.toNumber()).to.be.greaterThan(0);
    expect(account.lastExecutedAt.toNumber()).to.equal(0);
    expect(account.platformAuthority.toBase58()).to.equal(
      platformAuthority.publicKey.toBase58()
    );
  });

  it("rejects double-registration with same (owner, name)", async () => {
    const name = `my-workflow-dupe-${runId}`;
    const stepsHash = dummyStepsHash();
    const stepsCid = "QmDupe";

    const wfPda = workflowPda(owner.publicKey, name);
    const params = { name, triggerType: 0, stepsHash, stepsCid };

    await program.methods
      .registerWorkflow(params)
      .accounts({
        workflow: wfPda,
        owner: owner.publicKey,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    try {
      await program.methods
        .registerWorkflow(params)
        .accounts({
          workflow: wfPda,
          owner: owner.publicKey,
          platformAuthority: platformAuthority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([platformAuthority])
        .rpc();
      expect.fail("Expected second registration to fail");
    } catch (err: any) {
      // Anchor throws when trying to init an already-initialised account
      expect(err.message).to.match(/already in use|custom program error|0x0/i);
    }
  });

  it("set_workflow_status toggles is_active", async () => {
    const name = `my-workflow-toggle-${runId}`;
    const wfPda = workflowPda(owner.publicKey, name);

    await program.methods
      .registerWorkflow({
        name,
        triggerType: 1,
        stepsHash: dummyStepsHash(),
        stepsCid: "QmToggle",
      })
      .accounts({
        workflow: wfPda,
        owner: owner.publicKey,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    // Deactivate
    await program.methods
      .setWorkflowStatus(false)
      .accounts({ workflow: wfPda, owner: owner.publicKey })
      .rpc();

    let account = await program.account.workflowAccount.fetch(wfPda);
    expect(account.isActive).to.be.false;

    // Reactivate
    await program.methods
      .setWorkflowStatus(true)
      .accounts({ workflow: wfPda, owner: owner.publicKey })
      .rpc();

    account = await program.account.workflowAccount.fetch(wfPda);
    expect(account.isActive).to.be.true;
  });

  it("set_workflow_status rejects non-owner", async () => {
    const name = `my-workflow-noauth-${runId}`;
    const wfPda = workflowPda(owner.publicKey, name);
    const stranger = anchor.web3.Keypair.generate();

    // Fund stranger via direct transfer (devnet airdrop is rate-limited)
    const fundTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: stranger.publicKey,
        lamports: 0.02 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(fundTx);

    await program.methods
      .registerWorkflow({
        name,
        triggerType: 0,
        stepsHash: dummyStepsHash(),
        stepsCid: "QmNoAuth",
      })
      .accounts({
        workflow: wfPda,
        owner: owner.publicKey,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    try {
      await program.methods
        .setWorkflowStatus(false)
        .accounts({ workflow: wfPda, owner: stranger.publicKey })
        .signers([stranger])
        .rpc();
      expect.fail("Expected non-owner to be rejected");
    } catch (err: any) {
      // has_one constraint should fire
      expect(err.message).to.match(/has.*one|constraint|A has_one constraint/i);
    }
  });

  it("record_execution increments execution_count and updates last_executed_at", async () => {
    const name = `my-workflow-exec-${runId}`;
    const wfPda = workflowPda(owner.publicKey, name);

    await program.methods
      .registerWorkflow({
        name,
        triggerType: 0,
        stepsHash: dummyStepsHash(),
        stepsCid: "QmExec",
      })
      .accounts({
        workflow: wfPda,
        owner: owner.publicKey,
        platformAuthority: platformAuthority.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([platformAuthority])
      .rpc();

    for (let i = 1; i <= 3; i++) {
      await program.methods
        .recordExecution()
        .accounts({ workflow: wfPda, platformAuthority: platformAuthority.publicKey })
        .signers([platformAuthority])
        .rpc();

      const account = await program.account.workflowAccount.fetch(wfPda);
      expect(account.executionCount.toNumber()).to.equal(i);
      expect(account.lastExecutedAt.toNumber()).to.be.greaterThan(0);
    }
  });
});
