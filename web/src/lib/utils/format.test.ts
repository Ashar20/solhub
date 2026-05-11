import { describe, it, expect } from "vitest";
import { formatLamports, formatUsdc, formatAddress, formatRelativeTime, formatSlot, solscanTx, solscanAccount } from "./format";

describe("formatLamports", () => {
  it("formats with SOL suffix", () => {
    expect(formatLamports(1_500_000_000n)).toBe("1.5 SOL");
    expect(formatLamports(0n)).toBe("0 SOL");
  });
  it("handles tiny values", () => {
    expect(formatLamports(1_000n)).toBe("0.000001 SOL");
  });
});

describe("formatUsdc", () => {
  it("formats USDC with 2 decimals", () => {
    expect(formatUsdc(1_000_000n)).toBe("1.00 USDC");
    expect(formatUsdc(12_345_678n)).toBe("12.35 USDC");
  });
});

describe("formatAddress", () => {
  it("shortens to head…tail", () => {
    expect(formatAddress("So11111111111111111111111111111111111111112")).toBe("So111…1112");
  });
  it("returns input if already short", () => {
    expect(formatAddress("abc")).toBe("abc");
  });
});

describe("formatSlot", () => {
  it("formats with thousands separators", () => {
    expect(formatSlot(312_998_421)).toBe("312,998,421");
  });
});

describe("formatRelativeTime", () => {
  it("returns 'just now' under 60s", () => {
    expect(formatRelativeTime(new Date())).toBe("just now");
  });
  it("returns minutes for under an hour", () => {
    const d = new Date(Date.now() - 5 * 60_000);
    expect(formatRelativeTime(d)).toBe("5m ago");
  });
});

describe("solscanTx", () => {
  it("appends devnet cluster when network=devnet", () => {
    expect(solscanTx("abc", "devnet")).toBe("https://solscan.io/tx/abc?cluster=devnet");
  });
  it("omits cluster for mainnet", () => {
    expect(solscanTx("abc", "mainnet")).toBe("https://solscan.io/tx/abc");
  });
});

describe("solscanAccount", () => {
  it("works for devnet", () => {
    expect(solscanAccount("abc", "devnet")).toBe("https://solscan.io/account/abc?cluster=devnet");
  });
});
