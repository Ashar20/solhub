import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ConditionOracle } from "../../target/types/condition_oracle";
import { expect } from "chai";

describe("condition-oracle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ConditionOracle as Program<ConditionOracle>;
  const caller = provider.wallet as anchor.Wallet;

  it("evaluate emits ConditionEvaluated event with met=true by default", async () => {
    const stateAccount = anchor.web3.Keypair.generate().publicKey;

    // Subscribe before sending the transaction.
    // Use a promise that resolves on first event so we don't depend on a fixed sleep.
    const events: any[] = [];
    let resolveEvent!: () => void;
    const eventReceived = new Promise<void>((res) => { resolveEvent = res; });

    // Anchor converts PascalCase event names to camelCase for addEventListener
    const listener = program.addEventListener(
      "conditionEvaluated",
      (event) => {
        events.push(event);
        resolveEvent();
      }
    );

    // Small delay to let the WebSocket subscription register before firing the tx
    await new Promise((r) => setTimeout(r, 500));

    // Call evaluate with empty params
    const txSig = await program.methods
      .evaluate(Buffer.from([]))
      .accounts({
        stateAccount,
        caller: caller.publicKey,
      })
      .rpc({ commitment: "confirmed" });

    // Primary path: wait for WebSocket event (up to 15 s)
    await Promise.race([
      eventReceived,
      new Promise((resolve) => setTimeout(resolve, 15_000)),
    ]);

    await program.removeEventListener(listener);

    // Fallback: parse logs from the confirmed transaction directly if WebSocket missed it
    if (events.length === 0) {
      const tx = await provider.connection.getTransaction(txSig, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });
      const logs = tx?.meta?.logMessages ?? [];
      const eventParser = new anchor.EventParser(program.programId, program.coder);
      for (const event of eventParser.parseLogs(logs)) {
        // EventParser returns camelCase event names (e.g. "conditionEvaluated")
        if (event.name === "conditionEvaluated") {
          events.push(event.data);
        }
      }
    }

    expect(events.length).to.be.greaterThanOrEqual(1);
    const evt = events[0];
    expect(evt.met).to.be.true;

    // evaluated_at field may be camelCase (from WebSocket) or snake_case (from log fallback)
    // and may be a BN or a hex-encoded i64 (le bytes as hex)
    const evaluatedAtRaw = evt.evaluatedAt ?? evt.evaluated_at;
    let evaluatedAt: number;
    if (typeof evaluatedAtRaw === "object" && typeof evaluatedAtRaw.toNumber === "function") {
      evaluatedAt = evaluatedAtRaw.toNumber();
    } else if (typeof evaluatedAtRaw === "string") {
      // hex LE i64 from EventParser fallback
      const buf = Buffer.from(evaluatedAtRaw, "hex");
      evaluatedAt = Number(buf.readBigInt64LE());
    } else {
      evaluatedAt = Number(evaluatedAtRaw);
    }
    expect(evaluatedAt).to.be.greaterThan(0);

    const paramsHash = evt.paramsHash ?? evt.params_hash;
    expect(paramsHash).to.be.an("array").with.length(32);
  });
});
