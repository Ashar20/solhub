import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useSolBalance, useUsdcBalance } from "./use-balances";

const mockUseWallet = vi.fn();
vi.mock("@solana/wallet-adapter-react", () => ({
  useWallet: () => mockUseWallet(),
}));

const mockGetBalance = vi.fn();
const mockGetTokenAccountBalance = vi.fn();
vi.mock("@/lib/solana/connection", async () => {
  const original = await vi.importActual<typeof import("@/lib/solana/connection")>("@/lib/solana/connection");
  return {
    ...original,
    connection: {
      getBalance: (...args: unknown[]) => mockGetBalance(...args),
      getTokenAccountBalance: (...args: unknown[]) => mockGetTokenAccountBalance(...args),
    },
  };
});

// Mock getAssociatedTokenAddressSync to avoid crypto.subtle dependency in jsdom.
const mockGetAta = vi.fn();
vi.mock("@solana/spl-token", async () => {
  const original = await vi.importActual<typeof import("@solana/spl-token")>("@solana/spl-token");
  return {
    ...original,
    getAssociatedTokenAddressSync: (...args: unknown[]) => mockGetAta(...args),
  };
});

function wrap({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

// A known ATA address (pre-computed, used as stable stub return value).
const STUB_ATA = "7sHpZk76xsAuCa3DyLngjoZ2dyUd34HoDDzUUvfjDqU6";

beforeEach(async () => {
  mockUseWallet.mockReset();
  mockGetBalance.mockReset();
  mockGetTokenAccountBalance.mockReset();
  const { PublicKey } = await import("@solana/web3.js");
  mockGetAta.mockReturnValue(new PublicKey(STUB_ATA));
});

describe("useSolBalance", () => {
  it("returns 0 when wallet disconnected (disabled query stays loading)", () => {
    mockUseWallet.mockReturnValue({ publicKey: null });
    const { result } = renderHook(() => useSolBalance(), { wrapper: wrap });
    // disabled until publicKey exists
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("returns bigint from connection.getBalance", async () => {
    const { PublicKey } = await import("@solana/web3.js");
    const pk = new PublicKey("11111111111111111111111111111112");
    mockUseWallet.mockReturnValue({ publicKey: pk });
    mockGetBalance.mockResolvedValueOnce(1_500_000_000);
    const { result } = renderHook(() => useSolBalance(), { wrapper: wrap });
    await waitFor(() => expect(result.current.data).toBe(1_500_000_000n));
  });
});

// A real on-curve wallet address (Phantom devnet faucet, never funded).
const VALID_WALLET = "G66mBHXYyxVMDyZYbPA5N6U2pAU2L7k47CGB3d1BLeks";

describe("useUsdcBalance", () => {
  it("returns 0n when token account doesn't exist (caught error)", async () => {
    const { PublicKey } = await import("@solana/web3.js");
    const pk = new PublicKey(VALID_WALLET);
    mockUseWallet.mockReturnValue({ publicKey: pk });
    mockGetTokenAccountBalance.mockRejectedValueOnce(new Error("Account not found"));
    const { result } = renderHook(() => useUsdcBalance(), { wrapper: wrap });
    await waitFor(() => expect(result.current.data).toBe(0n));
  });

  it("returns parsed bigint balance", async () => {
    const { PublicKey } = await import("@solana/web3.js");
    const pk = new PublicKey(VALID_WALLET);
    mockUseWallet.mockReturnValue({ publicKey: pk });
    mockGetTokenAccountBalance.mockResolvedValueOnce({ value: { amount: "12345678", decimals: 6, uiAmount: 12.345678 } });
    const { result } = renderHook(() => useUsdcBalance(), { wrapper: wrap });
    await waitFor(() => expect(result.current.data).toBe(12_345_678n));
  });
});
