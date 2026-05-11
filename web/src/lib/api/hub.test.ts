import { describe, it, expect, vi, beforeEach } from "vitest";
import { paymentInfo, callHubWorkflow } from "./hub";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.setItem("solhub.bearer", "k");
});

function ok(body: unknown, status = 200) {
  return {
    ok: status < 400,
    status,
    statusText: "OK",
    text: async () => JSON.stringify(body),
  } as Response;
}

describe("paymentInfo", () => {
  it("GETs /v1/hub/:id/payment_info anonymously", async () => {
    F.mockResolvedValueOnce(
      ok({
        network: "solana-devnet",
        asset: "SOL",
        amount_lamports: 1000,
        recipient: "abc",
        memo: "hub-call:wf-1",
      }),
    );
    const r = await paymentInfo("wf-1");
    expect(r.amount_lamports).toBe(1000);
    expect(r.network).toBe("solana-devnet");
    expect(r.memo).toBe("hub-call:wf-1");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/hub/wf-1/payment_info");
    expect((init as RequestInit).headers).not.toHaveProperty("Authorization");
  });

  it("coerces string amount_lamports to number", async () => {
    F.mockResolvedValueOnce(
      ok({
        network: "solana-devnet",
        asset: "SOL",
        amount_lamports: "5000",
        recipient: "abc",
        memo: "hub-call:wf-2",
      }),
    );
    const r = await paymentInfo("wf-2");
    expect(r.amount_lamports).toBe(5000);
  });
});

describe("callHubWorkflow", () => {
  it("attaches X-PAYMENT header when signature provided", async () => {
    F.mockResolvedValueOnce(ok({ run_id: "11111111-1111-1111-1111-111111111111" }));
    await callHubWorkflow("wf-1", { paymentSignature: "SIG123" });
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).headers).toMatchObject({
      "X-PAYMENT": "solana:devnet:tx:SIG123",
    });
  });

  it("omits X-PAYMENT header when no signature", async () => {
    F.mockResolvedValueOnce(ok({ run_id: "11111111-1111-1111-1111-111111111111" }));
    await callHubWorkflow("wf-1");
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).headers).not.toHaveProperty("X-PAYMENT");
  });

  it("POSTs with Authorization header", async () => {
    F.mockResolvedValueOnce(
      ok({ run_id: "11111111-1111-1111-1111-111111111111", status: "Pending" }),
    );
    const result = await callHubWorkflow("wf-1");
    expect(result.run_id).toBe("11111111-1111-1111-1111-111111111111");
    const [url, init] = F.mock.calls[0]!;
    expect(String(url)).toContain("/v1/hub/wf-1/call");
    expect((init as RequestInit).method).toBe("POST");
    expect((init as RequestInit).headers).toMatchObject({ Authorization: "Bearer k" });
  });

  it("includes custom params in request body", async () => {
    F.mockResolvedValueOnce(ok({ run_id: "11111111-1111-1111-1111-111111111111" }));
    await callHubWorkflow("wf-1", { params: { amount: 100 } });
    const [, init] = F.mock.calls[0]!;
    const body = JSON.parse((init as RequestInit).body as string);
    expect(body.amount).toBe(100);
  });
});
