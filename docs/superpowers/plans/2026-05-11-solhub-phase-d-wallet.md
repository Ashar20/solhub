# Solhub Frontend — Phase D: Wallet + Vault Deposit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Phantom/Solflare via Solana wallet adapter. Add the Wallet screen showing personal balance + org's Turnkey-managed signing wallet. Implement USDC deposit into the `execution-vault` PDA via Anchor CPI.

**Architecture:** Wallet adapter as a top-level provider. Connection singleton reads `NEXT_PUBLIC_SOLANA_RPC_URL`. Deposit builds an `execution_vault.deposit_credits` ix using `@coral-xyz/anchor` + the program's IDL, signs with the connected wallet, submits via `connection.sendRawTransaction`, then refetches `/v1/orgs/me`.

**Tech Stack:** Adds `@solana/web3.js`, `@solana/wallet-adapter-base`, `@solana/wallet-adapter-react`, `@solana/wallet-adapter-react-ui`, `@solana/wallet-adapter-wallets`, `@coral-xyz/anchor`, `@solana/spl-token`.

**Hard dependency:** The `execution-vault` Anchor program must be deployed to devnet and its IDL exported. If the IDL isn't available, ship Phase D in **display-only mode** (Tasks 1–5 only). Tasks 6–8 (Anchor CPI) land when the IDL exists.

**Pre-requisite:** Phases A + B + C complete.

**Reference:** spec §8 (Wallet flow), §10 (Solana UI conventions). IDEA.md §3.2 (ExecutionVault program), §13 (env vars).

**Commit policy:** `git add web/` only.

---

## Task 1: Install wallet adapter + Anchor deps

- [ ] **Step 1**

```bash
cd web
pnpm add @solana/web3.js @solana/spl-token \
  @solana/wallet-adapter-base @solana/wallet-adapter-react \
  @solana/wallet-adapter-react-ui @solana/wallet-adapter-wallets \
  @coral-xyz/anchor
```

- [ ] **Step 2: Commit**

```bash
git add web/
git commit -m "chore(web): add Solana wallet adapter + Anchor"
```

---

## Task 2: Connection singleton + wallet provider

**Files:**
- Create: `web/src/lib/solana/connection.ts`
- Create: `web/src/lib/solana/wallet-provider.tsx`
- Modify: `web/src/components/Providers.tsx`

- [ ] **Step 1: `connection.ts`**

```ts
import { Connection, clusterApiUrl } from "@solana/web3.js";

const RPC = process.env.NEXT_PUBLIC_SOLANA_RPC_URL
  ?? clusterApiUrl((process.env.NEXT_PUBLIC_SOLANA_NETWORK as "devnet" | "mainnet-beta") ?? "devnet");

export const connection = new Connection(RPC, "confirmed");

export const NETWORK = (process.env.NEXT_PUBLIC_SOLANA_NETWORK as "mainnet" | "devnet") ?? "devnet";
```

- [ ] **Step 2: `wallet-provider.tsx`**

```tsx
"use client";
import { useMemo, type ReactNode } from "react";
import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import { PhantomWalletAdapter, SolflareWalletAdapter } from "@solana/wallet-adapter-wallets";
import "@solana/wallet-adapter-react-ui/styles.css";

const RPC = process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

export function SolanaWalletProvider({ children }: { children: ReactNode }) {
  const wallets = useMemo(() => [new PhantomWalletAdapter(), new SolflareWalletAdapter()], []);
  return (
    <ConnectionProvider endpoint={RPC}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
```

- [ ] **Step 3: Wrap in `Providers.tsx`**

```tsx
"use client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { SolanaWalletProvider } from "@/lib/solana/wallet-provider";

export function Providers({ children }: { children: React.ReactNode }) {
  const [qc] = useState(() => new QueryClient({
    defaultOptions: { queries: { staleTime: 30_000, gcTime: 5 * 60_000, retry: 1, refetchOnWindowFocus: false } },
  }));
  return (
    <QueryClientProvider client={qc}>
      <SolanaWalletProvider>{children}</SolanaWalletProvider>
    </QueryClientProvider>
  );
}
```

- [ ] **Step 4: Build + commit**

```bash
pnpm build
git add web/
git commit -m "feat(web): wallet adapter + connection provider"
```

---

## Task 3: ConnectButton in Topbar + network chip

**Files:**
- Create: `web/src/components/wallet/ConnectButton.tsx`
- Create: `web/src/components/wallet/NetworkChip.tsx`
- Modify: `web/src/components/shell/Topbar.tsx`
- Modify: app screens that pass `right=` to Topbar (Workflows page; others inherit).

- [ ] **Step 1: `NetworkChip.tsx`**

```tsx
import { Pill } from "@/components/primitives/Pill";
import { NETWORK } from "@/lib/solana/connection";

export function NetworkChip() {
  return <Pill tone={NETWORK === "mainnet" ? "emerald" : "amber"}>{NETWORK}</Pill>;
}
```

- [ ] **Step 2: `ConnectButton.tsx`**

```tsx
"use client";
import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { Btn } from "@/components/primitives/Btn";
import { formatAddress } from "@/lib/utils/format";

export function ConnectButton() {
  const { connected, publicKey, disconnect } = useWallet();
  const { setVisible } = useWalletModal();
  if (!connected) return <Btn size="sm" variant="default" onClick={() => setVisible(true)}>Connect</Btn>;
  return (
    <Btn size="sm" variant="default" onClick={disconnect}>
      {publicKey ? formatAddress(publicKey.toBase58()) : "Disconnect"}
    </Btn>
  );
}
```

- [ ] **Step 3: Update `Topbar.tsx` to always include connect+network chips**

```tsx
"use client";
import { Breadcrumb } from "./Breadcrumb";
import { Icon } from "@/components/primitives/Icon";
import { Kbd } from "@/components/primitives/Kbd";
import { ConnectButton } from "@/components/wallet/ConnectButton";
import { NetworkChip } from "@/components/wallet/NetworkChip";

export function Topbar({ crumbs, right }: { crumbs: string[]; right?: React.ReactNode }) {
  return (
    <header className="h-14 px-6 border-b border-ink-200 bg-white flex items-center justify-between">
      <Breadcrumb items={crumbs} />
      <div className="flex items-center gap-2">
        {right}
        <NetworkChip />
        <ConnectButton />
        <div className="flex items-center h-8 rounded-md border border-ink-200 bg-ink-50">
          <Icon name="search" className="w-3.5 h-3.5 text-ink-400 ml-2.5" />
          <input placeholder="Search workspace…" className="px-2 w-64 text-[12px] focus:outline-none bg-transparent" />
          <span className="mr-2"><Kbd>⌘K</Kbd></span>
        </div>
      </div>
    </header>
  );
}
```

- [ ] **Step 4: Smoke + commit**

```bash
pnpm dev
# Topbar shows network chip + Connect button on all (app) routes
# Click Connect → wallet modal opens
git add web/
git commit -m "feat(web): connect button + network chip in topbar"
```

---

## Task 4: Balance hooks

**Files:**
- Create: `web/src/lib/hooks/use-balances.ts`

- [ ] **Step 1: Implement**

```ts
"use client";
import { useQuery } from "@tanstack/react-query";
import { useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { connection, NETWORK } from "@/lib/solana/connection";

// USDC mints
const USDC_MINT_MAINNET = new PublicKey("EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v");
const USDC_MINT_DEVNET = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
export const USDC_MINT = NETWORK === "devnet" ? USDC_MINT_DEVNET : USDC_MINT_MAINNET;

export function useSolBalance() {
  const { publicKey } = useWallet();
  return useQuery({
    queryKey: ["balance", "sol", publicKey?.toBase58() ?? null] as const,
    queryFn: async () => {
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
    queryFn: async () => {
      if (!publicKey) return 0n;
      const ata = getAssociatedTokenAddressSync(USDC_MINT, publicKey, false, TOKEN_PROGRAM_ID);
      try {
        const info = await connection.getTokenAccountBalance(ata);
        return BigInt(info.value.amount);
      } catch { return 0n; }
    },
    enabled: !!publicKey,
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/
git commit -m "feat(web): SOL + USDC balance hooks"
```

---

## Task 5: Wallet screen (display-only)

**Files:**
- Create: `web/src/app/(app)/wallet/page.tsx`
- Create: `web/src/components/wallet/BalanceCard.tsx`

This task is sufficient for a Phase D MVP if the `execution-vault` IDL isn't ready yet. Deposit is added in Task 7.

- [ ] **Step 1: `BalanceCard.tsx`**

```tsx
import { Pill } from "@/components/primitives/Pill";
import { formatAddress, solscanAccount } from "@/lib/utils/format";

export function BalanceCard({
  title, subtitle, address, lines, network = "devnet", footer,
}: {
  title: string;
  subtitle?: string;
  address: string | null;
  lines: { label: string; value: React.ReactNode }[];
  network?: "mainnet" | "devnet";
  footer?: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 flex flex-col">
      <div className="flex items-center justify-between mb-2">
        <div>
          <div className="text-[14px] font-semibold tracking-tight">{title}</div>
          {subtitle && <div className="text-[11px] text-ink-500">{subtitle}</div>}
        </div>
        {address && (
          <a href={solscanAccount(address, network)} target="_blank" rel="noreferrer">
            <Pill tone="ink">{formatAddress(address)}</Pill>
          </a>
        )}
      </div>
      <div className="grid grid-cols-2 gap-2 mt-2">
        {lines.map((l) => (
          <div key={l.label}>
            <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500">{l.label}</div>
            <div className="text-[15px] font-medium font-mono mt-0.5">{l.value}</div>
          </div>
        ))}
      </div>
      {footer && <div className="mt-3 pt-3 border-t border-ink-100">{footer}</div>}
    </div>
  );
}
```

- [ ] **Step 2: Page**

```tsx
"use client";
import { useWallet } from "@solana/wallet-adapter-react";
import { Topbar } from "@/components/shell/Topbar";
import { useMe } from "@/lib/hooks/use-org";
import { useSolBalance, useUsdcBalance } from "@/lib/hooks/use-balances";
import { BalanceCard } from "@/components/wallet/BalanceCard";
import { formatLamports, formatUsdc } from "@/lib/utils/format";
import { NETWORK } from "@/lib/solana/connection";

export default function WalletPage() {
  const me = useMe();
  const { publicKey, connected } = useWallet();
  const sol = useSolBalance();
  const usdc = useUsdcBalance();

  return (
    <>
      <Topbar crumbs={["Account", "Wallet & Permissions"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        <div className="grid grid-cols-2 gap-4">
          <BalanceCard
            title="Personal wallet"
            subtitle="Connected via wallet adapter"
            address={connected ? publicKey?.toBase58() ?? null : null}
            network={NETWORK}
            lines={[
              { label: "SOL", value: sol.data != null ? formatLamports(sol.data) : "—" },
              { label: "USDC", value: usdc.data != null ? formatUsdc(usdc.data) : "—" },
            ]}
          />
          <BalanceCard
            title="Org signing wallet"
            subtitle="Read-only · Turnkey-managed"
            address={me.data?.wallet_address ?? null}
            network={NETWORK}
            lines={[
              { label: "Credits", value: me.data ? formatUsdc(me.data.credits_usdc) : "—" },
            ]}
          />
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): wallet screen (display-only)"
```

---

## Task 6: Anchor program client for execution-vault

**Files:**
- Create: `web/src/lib/anchor/idl/execution-vault.json` (copy from on-chain build output)
- Create: `web/src/lib/anchor/programs.ts`

If the IDL doesn't yet exist (the Anchor program isn't deployed), **stop here** and ship Phase D with display-only mode (Tasks 1–5). Proceed when the IDL is generated by `anchor build`.

- [ ] **Step 1: Source the IDL**

After `anchor build` in `programs/execution-vault/`, the IDL appears at `target/idl/execution_vault.json`. Copy it:

```bash
cp /home/philix/Documents/GitHub/solhub/target/idl/execution_vault.json \
   /home/philix/Documents/GitHub/solhub/web/src/lib/anchor/idl/execution-vault.json
```

If the program ID isn't in the IDL's `address` field, also note it in `web/.env.local`:
```
NEXT_PUBLIC_EXECUTION_VAULT_PROGRAM_ID=<the program id>
```

- [ ] **Step 2: `programs.ts`**

```ts
import { AnchorProvider, Program, type Idl } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import type { AnchorWallet } from "@solana/wallet-adapter-react";
import idl from "./idl/execution-vault.json";

const PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_EXECUTION_VAULT_PROGRAM_ID ?? (idl as { address?: string }).address ?? "11111111111111111111111111111111",
);

export function executionVaultProgram(connection: Connection, wallet: AnchorWallet) {
  const provider = new AnchorProvider(connection, wallet, AnchorProvider.defaultOptions());
  return new Program(idl as Idl, PROGRAM_ID, provider);
}

export { PROGRAM_ID as EXECUTION_VAULT_PROGRAM_ID };
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): execution-vault anchor program client"
```

---

## Task 7: DepositDialog

**Files:**
- Create: `web/src/components/wallet/DepositDialog.tsx`
- Create: `web/src/lib/anchor/deposit.ts`
- Modify: `web/src/app/(app)/wallet/page.tsx`

- [ ] **Step 1: `deposit.ts`**

```ts
import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { type Program } from "@coral-xyz/anchor";
import { USDC_MINT } from "@/lib/hooks/use-balances";
import { EXECUTION_VAULT_PROGRAM_ID } from "./programs";

/** Derive the org vault PDA: seeds = ["vault", org_id_pubkey] */
export function findVaultPda(orgPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), orgPubkey.toBuffer()],
    EXECUTION_VAULT_PROGRAM_ID,
  );
}

export async function buildDepositTx(
  program: Program,
  payer: PublicKey,
  orgPubkey: PublicKey,
  amountUsdcMicro: bigint,
): Promise<Transaction> {
  const [vault] = findVaultPda(orgPubkey);
  const payerAta = getAssociatedTokenAddressSync(USDC_MINT, payer, false, TOKEN_PROGRAM_ID);
  const vaultAta = getAssociatedTokenAddressSync(USDC_MINT, vault, true, TOKEN_PROGRAM_ID);

  const ix = await program.methods
    .depositCredits(amountUsdcMicro as unknown as never) // anchor BN coercion
    .accounts({
      vault,
      payer,
      payerTokenAccount: payerAta,
      vaultTokenAccount: vaultAta,
      mint: USDC_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();

  const tx = new Transaction().add(ix);
  return tx;
}
```

Note: Account names must match the on-chain `DepositCredits` accounts struct exactly. Read `programs/execution-vault/src/lib.rs` to confirm — adjust field names if they differ.

- [ ] **Step 2: `DepositDialog.tsx`**

```tsx
"use client";
import { useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { useWallet, useConnection, useAnchorWallet } from "@solana/wallet-adapter-react";
import { useQueryClient } from "@tanstack/react-query";
import { Btn } from "@/components/primitives/Btn";
import { executionVaultProgram } from "@/lib/anchor/programs";
import { buildDepositTx } from "@/lib/anchor/deposit";
import { formatUsdc } from "@/lib/utils/format";

export function DepositDialog({
  orgPubkey, usdcBalance, onClose,
}: { orgPubkey: string; usdcBalance: bigint; onClose: () => void }) {
  const [amount, setAmount] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const { publicKey, sendTransaction } = useWallet();
  const wallet = useAnchorWallet();
  const { connection } = useConnection();
  const qc = useQueryClient();

  async function submit() {
    if (!publicKey || !wallet) return;
    const micro = BigInt(Math.floor(parseFloat(amount || "0") * 1_000_000));
    if (micro <= 0n) { setErr("Enter an amount"); return; }
    if (micro > usdcBalance) { setErr("Exceeds personal USDC balance"); return; }
    setBusy(true); setErr(null);
    try {
      const program = executionVaultProgram(connection, wallet);
      const tx = await buildDepositTx(program, publicKey, new PublicKey(orgPubkey), micro);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      const sig = await sendTransaction(tx, connection);
      await connection.confirmTransaction(sig, "confirmed");
      qc.invalidateQueries({ queryKey: ["org", "me"] });
      qc.invalidateQueries({ queryKey: ["balance"] });
      onClose();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Transaction failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[420px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        <div>
          <h2 className="text-[16px] font-semibold tracking-tight">Deposit USDC</h2>
          <p className="text-[12px] text-ink-500">Available: {formatUsdc(usdcBalance)}</p>
        </div>
        <label className="block">
          <span className="text-[12px] font-medium">Amount (USDC)</span>
          <input
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            placeholder="10.00"
            className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px] font-mono"
          />
        </label>
        {err && <p className="text-[12px] text-rose-600">{err}</p>}
        <div className="flex justify-end gap-2">
          <Btn variant="ghost" onClick={onClose} disabled={busy}>Cancel</Btn>
          <Btn variant="primary" onClick={submit} disabled={busy || !amount}>
            {busy ? "Submitting…" : "Deposit"}
          </Btn>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Wire into Wallet screen footer**

In `wallet/page.tsx`, add state + button on the Org signing wallet card's `footer`:

```tsx
const [open, setOpen] = useState(false);
// ...
<BalanceCard
  title="Org signing wallet"
  // ...same props...
  footer={
    <div className="flex justify-end">
      <Btn
        variant="primary"
        size="sm"
        disabled={!connected || !me.data?.wallet_address}
        onClick={() => setOpen(true)}
      >
        Deposit credits
      </Btn>
    </div>
  }
/>
{open && me.data?.wallet_address && (
  <DepositDialog
    orgPubkey={me.data.wallet_address}
    usdcBalance={usdc.data ?? 0n}
    onClose={() => setOpen(false)}
  />
)}
```

(Imports: `useState`, `Btn`, `DepositDialog`.)

- [ ] **Step 4: Manual smoke on devnet**

1. Get devnet USDC from a faucet (or mint to your wallet for testing).
2. Connect Phantom on devnet.
3. Open `/wallet` → click "Deposit credits" → enter 1 → submit.
4. Confirm tx on Solscan (devnet).
5. Org credit balance should refetch from `/v1/orgs/me`.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): USDC deposit dialog wired to execution-vault"
```

---

## Task 8: WithdrawDialog (creator earnings)

**Files:**
- Create: `web/src/components/wallet/WithdrawDialog.tsx`
- Modify: `web/src/lib/anchor/deposit.ts` (add `buildWithdrawTx`)
- Modify: `web/src/app/(app)/wallet/page.tsx`

This is only relevant if the connected user is a creator with earned balance.

- [ ] **Step 1: Add `buildWithdrawTx` to `deposit.ts`**

```ts
export async function buildWithdrawTx(
  program: Program,
  payer: PublicKey,
  amountUsdcMicro: bigint,
): Promise<Transaction> {
  const [creatorAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from("creator"), payer.toBuffer()],
    EXECUTION_VAULT_PROGRAM_ID,
  );
  const payerAta = getAssociatedTokenAddressSync(USDC_MINT, payer, false, TOKEN_PROGRAM_ID);
  const creatorAta = getAssociatedTokenAddressSync(USDC_MINT, creatorAccount, true, TOKEN_PROGRAM_ID);

  const ix = await program.methods
    .withdrawCreator(amountUsdcMicro as unknown as never)
    .accounts({
      creatorAccount,
      owner: payer,
      ownerTokenAccount: payerAta,
      creatorTokenAccount: creatorAta,
      mint: USDC_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .instruction();

  return new Transaction().add(ix);
}
```

Confirm account names against `programs/execution-vault/src/lib.rs` `WithdrawCreator`.

- [ ] **Step 2: `WithdrawDialog.tsx`** — same shape as DepositDialog, calls `buildWithdrawTx`. Skipping full code: it's a near-copy with the dialog title "Withdraw earnings" and uses the creator's accrued balance (fetched via `program.account.creatorAccount.fetch(creatorPda)` rather than user USDC balance).

```tsx
"use client";
import { useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { useConnection, useAnchorWallet, useWallet } from "@solana/wallet-adapter-react";
import { useQueryClient } from "@tanstack/react-query";
import { Btn } from "@/components/primitives/Btn";
import { executionVaultProgram, EXECUTION_VAULT_PROGRAM_ID } from "@/lib/anchor/programs";
import { buildWithdrawTx } from "@/lib/anchor/deposit";
import { formatUsdc } from "@/lib/utils/format";

export function WithdrawDialog({ onClose }: { onClose: () => void }) {
  const { publicKey, sendTransaction } = useWallet();
  const wallet = useAnchorWallet();
  const { connection } = useConnection();
  const qc = useQueryClient();
  const [available, setAvailable] = useState<bigint | null>(null);
  const [amount, setAmount] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      if (!publicKey || !wallet) return;
      const program = executionVaultProgram(connection, wallet);
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from("creator"), publicKey.toBuffer()],
        EXECUTION_VAULT_PROGRAM_ID,
      );
      try {
        const acc = await program.account.creatorAccount.fetch(pda) as { balance: { toString(): string } };
        setAvailable(BigInt(acc.balance.toString()));
      } catch {
        setAvailable(0n);
      }
    })();
  }, [publicKey, wallet, connection]);

  async function submit() {
    if (!publicKey || !wallet) return;
    const micro = BigInt(Math.floor(parseFloat(amount || "0") * 1_000_000));
    if (micro <= 0n) return setErr("Enter an amount");
    if (available != null && micro > available) return setErr("Exceeds available balance");
    setBusy(true); setErr(null);
    try {
      const program = executionVaultProgram(connection, wallet);
      const tx = await buildWithdrawTx(program, publicKey, micro);
      tx.feePayer = publicKey;
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      const sig = await sendTransaction(tx, connection);
      await connection.confirmTransaction(sig, "confirmed");
      qc.invalidateQueries({ queryKey: ["balance"] });
      onClose();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Transaction failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[420px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        <div>
          <h2 className="text-[16px] font-semibold tracking-tight">Withdraw earnings</h2>
          <p className="text-[12px] text-ink-500">Available: {available != null ? formatUsdc(available) : "—"}</p>
        </div>
        <label className="block">
          <span className="text-[12px] font-medium">Amount (USDC)</span>
          <input
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            inputMode="decimal"
            placeholder="10.00"
            className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px] font-mono"
          />
        </label>
        {err && <p className="text-[12px] text-rose-600">{err}</p>}
        <div className="flex justify-end gap-2">
          <Btn variant="ghost" onClick={onClose} disabled={busy}>Cancel</Btn>
          <Btn variant="primary" onClick={submit} disabled={busy || !amount}>
            {busy ? "Submitting…" : "Withdraw"}
          </Btn>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Wire into Wallet page**

Add a third card (or extend the Personal card footer) showing a "Withdraw earnings" button that only renders when the creator account exists and has non-zero balance.

```tsx
const [openWithdraw, setOpenWithdraw] = useState(false);
// ...within JSX:
<div className="mt-4">
  <Btn variant="default" size="sm" onClick={() => setOpenWithdraw(true)} disabled={!connected}>
    Withdraw creator earnings
  </Btn>
</div>
{openWithdraw && <WithdrawDialog onClose={() => setOpenWithdraw(false)} />}
```

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): creator earnings withdraw flow"
```

---

## Task 9: Acceptance smoke pass

- [ ] **Step 1: Manual checklist** (devnet)

| Step | Expected |
|---|---|
| Topbar shows "devnet" pill + Connect | ✓ |
| Click Connect → choose Phantom | wallet modal works |
| `/wallet` shows personal SOL + USDC | balances render |
| `/wallet` shows org wallet address + credits | from `/v1/orgs/me` |
| Click "Deposit credits" → submit 1 USDC | tx confirms; credits refetched |
| Disconnect → reconnect | balances refetch |
| Withdraw flow loads if creator account exists | balance available |

- [ ] **Step 2: typecheck + test + build**

```bash
pnpm typecheck && pnpm test && pnpm build
```

- [ ] **Step 3: Final commit if needed**

```bash
git add web/
git commit -m "fix(web): phase D smoke pass adjustments"  # only if needed
```

---

## Self-review checklist

- [ ] Spec §8 — Connect, personal card, org card (read-only), deposit dialog, withdraw flow. ✓
- [ ] Spec §10 — Solscan links on addresses + network indicator chip. ✓
- [ ] Hard dependency on IDL flagged. Tasks 6–8 only run when `target/idl/execution_vault.json` exists. ✓
- [ ] Account names in `buildDepositTx`/`buildWithdrawTx` must match on-chain structs — explicit "verify against lib.rs" note. ✓
- [ ] No `git add .` anywhere. ✓

---

## End-of-phase acceptance

Phase D is **done** when:
- Display-only mode (Tasks 1–5) works regardless of IDL availability.
- When IDL exists, deposit + withdraw confirm on devnet.
- `pnpm typecheck && pnpm test && pnpm build` pass.
