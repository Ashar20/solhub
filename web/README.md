# Solhub Web

Frontend for the SolHub (SolanaKeeper) Solana automation platform. Next.js 14 + TypeScript + Tailwind + TanStack Query.

## Run locally

```bash
cp .env.example .env.local
pnpm install
pnpm dev    # http://localhost:3000
```

The backend Rust API must be running on `http://localhost:8080` (or wherever `NEXT_PUBLIC_API_BASE_URL` points). Sign in via the `/login` route with an API key from the backend.

## Scripts

| Command | What it does |
|---|---|
| `pnpm dev` | Dev server on `:3000` (hot reload) |
| `pnpm build` | Production build |
| `pnpm start` | Serve the production build |
| `pnpm lint` | `next lint` |
| `pnpm typecheck` | `tsc --noEmit` |
| `pnpm test` | Vitest suite (one-shot) |
| `pnpm test:watch` | Vitest watch mode |
| `pnpm format` | Prettier write |

## Layout

```
src/
├─ app/                     # App Router
│  ├─ (auth)/login/         # Public — API-key sign-in
│  └─ (app)/                # Authenticated shell (Sidebar + Topbar)
├─ components/
│  ├─ primitives/           # Btn, Pill, Kbd, Icon, SolhubLogo, SolanaMark
│  └─ shell/                # Sidebar, Topbar, Breadcrumb
└─ lib/
   ├─ api/                  # Typed fetch client + Zod schemas per resource
   ├─ auth/                 # useAuth + localStorage Bearer token
   └─ utils/                # cn, format
```

## Phases

This frontend is built in five phases. See `docs/superpowers/plans/2026-05-11-solhub-phase-*.md` (repo-root) for the full per-phase task plan.

- **Phase A** — Foundation (scaffold, theme, primitives, API client, auth gate)  ← **current**
- **Phase B** — Read-only screens (login, dashboard, workflows list, runs, marketplace)
- **Phase C** — Mutations + React Flow workflow builder
- **Phase D** — Wallet adapter + Vault deposit via Anchor
- **Phase E** — Marketplace detail, Settings, AI Builder, Versions
