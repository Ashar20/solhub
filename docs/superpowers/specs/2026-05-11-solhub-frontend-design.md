# Solhub Frontend — Design Spec

**Date:** 2026-05-11
**Status:** Approved — ready for implementation plan
**Scope:** Production frontend for SolHub (SolanaKeeper) that ports the prototype in `solhub.zip` into a Next.js app wired to the Rust/Axum backend defined in `IDEA.md`.

---

## 1. Inputs

- **Backend spec:** `IDEA.md` — Rust/Axum REST API at `:8080`, Bearer-token auth, Anchor programs (`workflow-registry`, `execution-vault`, `condition-oracle`), Jito + Geyser data plane, MCP server, PostgreSQL.
- **Design template:** `solhub.zip` — Babel-in-browser React prototype, ~2.5k LOC across 5 `.jsx` files. Tailwind via CDN. Ten screens: `login, dashboard, workflows, ai, builder, runs, marketplace, wallet, versions, settings`. Light theme, palette `ink.*` / `violet.*` / `sol.green` (#14F195) / `sol.purple` (#9945FF). Fonts: Inter, JetBrains Mono, Instrument Serif.
- **Screenshot:** Retell-style builder canvas — left tool palette, central node graph with edge animation, right inspector panel.

The template is a visual reference, not a deployable app. We're porting its look and information architecture into a real Next.js project.

## 2. Goals & non-goals

**Goals**
- Pixel-faithful port of the prototype's look and IA into Next.js 14 + TypeScript.
- Full API surface wired against the Rust API as it lands (graceful empty states where endpoints don't exist yet).
- Workflow builder uses React Flow with typed, Zod-driven inspector forms per plugin/action.
- Wallet adapter (Phantom + Solflare) + on-chain USDC deposit into `execution-vault` PDA on devnet.
- SSE-driven live run logs.

**Non-goals (v1)**
- Squads multisig, SSO/SAML, audit log exports (IDEA.md Phase 4).
- Dynamic plugin discovery endpoint — frontend ships a static registry mirroring IDEA.md §7.3.
- i18n, dark mode, mobile layout.
- E2E (Playwright) tests — unit + component only in v1.
- Real-time collaborative editing.

## 3. Architecture

**Stack**
- **Next.js 14** (App Router) + **TypeScript** (strict).
- **Tailwind CSS v3.4** — ports the template's theme verbatim (ink, violet, sol.green, sol.purple, shadows: card/pop/inset-line, grid-bg/dot-bg, keyframes flowDash/pulse-ring).
- **TanStack Query** for server state. One `QueryClient` in the root `<Providers>`. Default `staleTime: 30s`, `gcTime: 5m`.
- **React Hook Form + Zod** for forms. Zod schemas in `lib/api/schemas.ts` mirror `api/src/types.rs` and are the source of truth for request/response types.
- **React Flow** for the workflow builder canvas. Custom node types styled with the template's Tailwind classes for visual parity.
- **Solana wallet adapter** (`@solana/wallet-adapter-react`, `@solana/wallet-adapter-wallets`) — Phantom + Solflare for v1.
- **Anchor TS client** (`@coral-xyz/anchor`) for the `execution-vault` CPI.
- **Native EventSource** for `/v1/runs/:id/logs` SSE.
- **Self-hosted fonts** via `next/font/google` (Inter, JetBrains Mono) + `public/fonts/` for Instrument Serif.

**API client**
- Thin typed wrapper in `web/src/lib/api/client.ts` over `fetch`.
- `NEXT_PUBLIC_API_BASE_URL` (default `http://localhost:8080`).
- Bearer token from `localStorage` key `solhub.bearer`, set by the login screen.
- Every response parsed through Zod — runtime safety against a still-changing backend.
- 401 → clear token + hard redirect to `/login`.
- Non-2xx → `ApiError { status, code, message }`.

**Why hand-typed (not OpenAPI-generated):** Backend isn't done; generators drift. IDEA.md is stable enough to hand-type. Easy to swap for codegen later without rewriting call sites.

## 4. Repo layout

Add `web/` at repo root, sibling to `api/`, `engine/`, `cli/`, `mcp-server/`.

```
web/
├── package.json
├── tsconfig.json
├── next.config.mjs
├── tailwind.config.ts
├── postcss.config.mjs
├── .env.example
├── .eslintrc.cjs
├── public/
│   └── fonts/                          # Instrument Serif woff2
└── src/
    ├── app/
    │   ├── layout.tsx                  # Providers: QueryClient, WalletAdapter, Auth
    │   ├── globals.css                 # Tailwind + ported keyframes
    │   ├── (auth)/login/page.tsx
    │   ├── (app)/
    │   │   ├── layout.tsx              # Sidebar + Topbar shell (auth-gated)
    │   │   ├── dashboard/page.tsx
    │   │   ├── workflows/page.tsx
    │   │   ├── workflows/[id]/page.tsx # Builder canvas (`new` is a sentinel for blank)
    │   │   ├── ai/page.tsx
    │   │   ├── runs/page.tsx
    │   │   ├── runs/[run_id]/page.tsx
    │   │   ├── marketplace/page.tsx
    │   │   ├── marketplace/[id]/page.tsx
    │   │   ├── wallet/page.tsx
    │   │   ├── versions/page.tsx
    │   │   └── settings/page.tsx
    │   └── not-found.tsx
    ├── components/
    │   ├── primitives/                 # Btn, Pill, Kbd, Icon, SolanaMark, SolhubLogo
    │   ├── shell/                      # Sidebar, Topbar, Breadcrumb
    │   ├── workflow/                   # NodeCard, EdgeStyles, ToolPalette, Inspector
    │   ├── runs/                       # RunRow, StepTimeline, LiveLogStream
    │   ├── marketplace/                # PlaybookCard, ProtocolBadge
    │   └── wallet/                     # ConnectButton, BalanceCard, DepositDialog
    ├── lib/
    │   ├── api/
    │   │   ├── client.ts
    │   │   ├── workflows.ts
    │   │   ├── runs.ts
    │   │   ├── hub.ts
    │   │   ├── analytics.ts
    │   │   ├── orgs.ts
    │   │   └── schemas.ts
    │   ├── anchor/
    │   │   ├── idl/                    # execution-vault.json (workflow-registry read-only via API)
    │   │   ├── programs.ts
    │   │   └── deposit.ts
    │   ├── solana/
    │   │   ├── connection.ts
    │   │   └── wallet-provider.tsx
    │   ├── auth/
    │   │   ├── store.ts
    │   │   └── use-auth.ts
    │   ├── hooks/
    │   │   ├── use-workflows.ts
    │   │   ├── use-run-stream.ts
    │   │   └── use-deposit.ts
    │   └── utils/
    │       ├── format.ts               # lamports, USDC, dates, slots
    │       └── cn.ts
    └── mocks/                          # test/Storybook only; NOT used in app code paths
        └── fixtures.ts
```

**Conventions**
- Route groups `(auth)` and `(app)` separate the login screen from the authenticated shell.
- `(app)/layout.tsx` enforces auth client-side (token in `localStorage`) and renders Sidebar + Topbar once.
- `components/primitives` are typed equivalents of the template's `Btn`, `Pill`, `Icon`, etc. — same Tailwind classes, should render byte-identical.
- The template's `MOCK` object is retired from app code. Empty states + loading skeletons handle absent data.

## 5. Auth flow

1. `(auth)/login/page.tsx` — paste API key → validate by calling `GET /v1/orgs/me` with the key as Bearer. On 200, write key to `localStorage.solhub.bearer` and redirect to `/dashboard`. On 401, show inline error.
2. `(app)/layout.tsx` runs a client-side check at mount: if `solhub.bearer` is missing, redirect to `/login`. Server Components can't read `localStorage`, so this is intentionally client-side.
3. `client.ts` reads the token on every call. On 401, clears the token and hard-redirects to `/login`.

## 6. Screens → endpoints

| Screen | Reads | Writes | Notes |
|---|---|---|---|
| **Login** | `GET /v1/orgs/me` (validates pasted key) | — | Stores `solhub.bearer` on success. |
| **Dashboard** | `GET /v1/analytics?range=7d`, `GET /v1/workflows?limit=5`, `GET /v1/runs?limit=10` | — | KPI tiles + recent activity. |
| **Workflows** (list) | `GET /v1/workflows` with `status`, `trigger_type` filters | `POST /v1/workflows/:id/trigger`, `PATCH /v1/workflows/:id`, `DELETE /v1/workflows/:id` | Client-side search. "New" → `/workflows/new`. |
| **Builder** (`/workflows/[id]`) | `GET /v1/workflows/:id` (skipped when `id === "new"`) | `POST /v1/workflows`, `PATCH /v1/workflows/:id`, `POST /v1/workflows/:id/trigger` (test run) | React Flow canvas. Left palette from static `plugins.ts` registry mirroring IDEA.md §7.3. Right inspector = Zod-driven form per node. Auto-save draft to `localStorage` keyed by workflow id. Publish = PATCH `is_active: true`. |
| **AI Builder** | — | `POST /v1/ai/build` *(new — Backend gap #2)* | Prompt → workflow JSON preview → "Open in builder". |
| **Runs** (list) | `GET /v1/runs`, polls every 5s when focused | — | Status pills, slot, signature → Solscan, Jito tip column. |
| **Run detail** (`/runs/[run_id]`) | `GET /v1/runs/:run_id`; `GET /v1/runs/:run_id/logs` SSE | — | Step timeline + live log tail. Closes EventSource on `run_complete` or unmount. |
| **Marketplace** | `GET /v1/hub` (public, no auth) | `POST /v1/hub/:id/call` | Protocol badges, fee, run count, success, verified/audited. |
| **Marketplace detail** | `GET /v1/hub/:id` *(new — Backend gap #3)* | `POST /v1/hub/:id/call` | Read-only canvas preview + reviews + run history. |
| **Wallet** | `GET /v1/orgs/me`; on-chain reads via `lib/solana/connection`; wallet adapter for personal balance | `execution_vault.deposit_credits` ix; after confirm, refetch `/v1/orgs/me` | Connect Phantom/Solflare. Personal wallet card + read-only Turnkey org wallet card. Deposit dialog. Withdraw (`withdraw_creator`) if user is a Hub creator. |
| **Versions** | `GET /v1/workflows/:id/versions` *(new — Backend gap #4)* | `POST /v1/workflows/:id/rollback` *(new)* | Per-workflow version timeline + diff. Stub UI if endpoints not ready. |
| **Settings** | `GET /v1/orgs/me`, `GET /v1/orgs/me/api_keys` | `POST /v1/orgs/me/api_keys`, `DELETE /v1/orgs/me/api_keys/:key_id` | Raw key shown once on create — matches IDEA.md Non-Negotiable Rule #8. |

## 7. SSE — live run logs

- Custom hook `useRunStream(run_id)` opens an `EventSource`, parses `RunLogEvent` (Zod), pushes events into local state.
- Handles the spec's four event types: `step_start`, `step_complete`, `run_complete`, `error`.
- Reconnects with exponential backoff on `onerror`. Closes on unmount or `run_complete`.
- **Backend gap #1:** `EventSource` cannot send custom headers. The SSE endpoint must accept the Bearer token as a `?token=<value>` query parameter in addition to the `Authorization` header.
- **Fallback** until that ships: Run detail page polls `GET /v1/runs/:run_id` every 1s. Hook abstracts the choice so the call site is identical.

## 8. Wallet + Vault deposit flow

1. User clicks **Connect Wallet** in topbar (or Wallet screen). Wallet adapter opens Phantom/Solflare modal.
2. Wallet screen renders two cards:
   - **Personal wallet** — SOL balance via `connection.getBalance`, USDC balance via `getTokenAccountsByOwner`. Address with Solscan link.
   - **Org signing wallet (read-only)** — address + USDC credits from `GET /v1/orgs/me`.
3. **Deposit** button opens `DepositDialog`:
   - Input USDC amount (validated against personal USDC balance).
   - Builds `execution_vault.deposit_credits(amount)` instruction via `@coral-xyz/anchor` using the IDL in `lib/anchor/idl/execution-vault.json`.
   - Signs with wallet adapter, submits via `connection.sendRawTransaction`.
   - Awaits `confirmed` commitment.
   - Calls `GET /v1/orgs/me` to refresh credit balance (or `POST /v1/orgs/me/credits/refresh` — Backend gap #5).
4. **Withdraw creator earnings** — same pattern, calling `execution_vault.withdraw_creator(amount)`.

**Hard dependency:** `execution-vault` IDL must exist. IDEA.md puts that program in Phase 2. See §11 Risks.

## 9. Backend gaps

These endpoints aren't in IDEA.md §8.4 but the v1 frontend needs them. Documenting here so the backend team can fill them in.

1. **SSE token-via-query:** `GET /v1/runs/:run_id/logs?token=<bearer>` — accept token in query for EventSource compatibility.
2. **AI build:** `POST /v1/ai/build` — body `{prompt: string}` → returns a draft `WorkflowConfig`. Calls Anthropic server-side (`ANTHROPIC_API_KEY` per spec env).
3. **Hub detail:** `GET /v1/hub/:id` — single marketplace workflow detail.
4. **Versions:** `GET /v1/workflows/:id/versions` + `POST /v1/workflows/:id/rollback`.
5. **Credit refresh:** `POST /v1/orgs/me/credits/refresh` — trigger backend re-read of on-chain vault balance after a deposit.

Frontend ships with graceful empty/error states for all five so screens don't break when endpoints return 404.

## 10. UI conventions specific to Solana

- Addresses use a `formatAddress(pubkey)` helper rendering `abc…xyz`, with hover-to-copy and Solscan link.
- Tx signatures + slots link to Solscan.
- All lamport/USDC values formatted via `lib/utils/format.ts` — never display raw u64.
- Network indicator chip in topbar (mainnet/devnet) sourced from `NEXT_PUBLIC_SOLANA_NETWORK`.

## 11. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Backend endpoints not ready when frontend phases run | TanStack Query handles loading/error/empty uniformly. Each screen renders an empty state instead of breaking. "Backend gaps" list is the integration contract. |
| `execution-vault` IDL doesn't exist yet (Phase 2 in IDEA.md) | Phase D ships display-only (balances + addresses, no deposit button) if IDL not ready. Deposit dialog lands when IDL ships. |
| EventSource auth via query param requires backend change | Fallback to polling `GET /v1/runs/:run_id` every 1s until the change lands. Hook hides the choice. |
| React Flow visual parity with template's hand-rolled SVG | Custom node types styled with same Tailwind classes. Budget half a Phase C task for polish. |
| Tailwind v4 vs v3 mismatch | Pin v3.4 in `package.json`. |
| Template uses Babel-in-browser globals (`window.MOCK`, global `Icon`) | Each file becomes a module with explicit imports/exports during port. |
| Self-hosted vs CDN fonts | `next/font/google` for Inter and JetBrains Mono; Instrument Serif as woff2 in `public/fonts/` (logo only). |

## 12. Phasing

Implementation plan will sequence work in five phases. Each merges to main and produces a working app.

**Phase A — Foundation.** Next.js scaffold, Tailwind theme port, fonts, providers, auth store, full `lib/api/` + Zod schemas, `components/primitives` (pixel-faithful), Sidebar + Topbar shell, `(auth)`/`(app)` route groups.

**Phase B — Read-only screens.** Login, Dashboard, Workflows list, Runs list, Run detail (incl. SSE/polling), Marketplace list.

**Phase C — Mutations + builder.** Workflows CRUD, manual trigger, enable/disable/delete. React Flow canvas: nodes, edges, palette, Zod-driven inspector. Auto-save draft, publish, test-run.

**Phase D — Wallet + Solana.** Wallet adapter, network indicator, Wallet screen, Vault deposit (subject to IDL availability).

**Phase E — Remaining screens.** Marketplace detail + `hub.call`, Settings + API key management, AI Builder, Versions.

Demo-able after B. Usable internal beta after C. v1 after E.

## 13. Testing

- **Unit (Vitest):** every Zod schema, every `lib/api/*` function (mocked fetch), every `lib/utils/` helper, `useRunStream` (mock EventSource).
- **Component (Vitest + RTL):** primitives, NodeCard, RunRow, DepositDialog using fixtures from `mocks/fixtures.ts`.
- **Types:** `tsc --noEmit` in CI.
- **Manual smoke checklist:** login → create workflow → trigger → watch SSE log to completion → deposit credits on devnet → verify on Solscan.

No E2E framework in v1 — Playwright is a follow-up.

## 14. Definition of done (v1)

- `pnpm dev` in `web/` boots a working app against the Rust API at `localhost:8080`.
- All ten screens render real data when endpoints exist, graceful empty/loading states when they don't.
- Phantom/Solflare connect; a USDC deposit lands in `execution-vault` PDA on devnet.
- `pnpm typecheck && pnpm test && pnpm build` pass clean in CI.

## 15. Open security note (unrelated to design)

`CLAUDE.md` in this repo contains a literal GitHub PAT. It should be revoked on GitHub regardless of whether the repo is public — anyone who scraped it has the token. This spec is committed using the local git identity, not via the leaked PAT.
