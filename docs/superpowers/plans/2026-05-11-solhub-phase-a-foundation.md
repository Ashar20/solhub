# Solhub Frontend — Phase A: Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bootstrap a Next.js 14 app at `web/` with the prototype's theme, typed API client, auth, and app shell. No real-data screens yet — this is the prerequisite phase that unblocks B–E.

**Architecture:** Next.js 14 App Router + TypeScript strict, Tailwind v3.4, TanStack Query v5, Zod v3, Vitest. Bearer-token auth in `localStorage`. Every API response Zod-parsed. Primitives port the prototype's design language pixel-faithfully.

**Tech Stack:** Next.js 14, React 18, TypeScript, Tailwind 3.4, TanStack Query 5, Zod 3, Vitest, @testing-library/react, lucide-react (not used — we port the template's SVG icon set instead)

**Reference:** `docs/superpowers/specs/2026-05-11-solhub-frontend-design.md` §3–5, §10.

**Commit policy in this plan:** Commit code under `web/` only. Never `git add .` or `git add -A`. Plan/spec files in `docs/superpowers/` stay **uncommitted** per repo owner's instruction.

---

## Task 1: Scaffold Next.js project

**Files:**
- Create: `web/` (entire tree via `create-next-app`)

- [ ] **Step 1: Create the Next.js project**

Run from repo root:
```bash
cd /home/philix/Documents/GitHub/solhub
pnpm create next-app@14 web --typescript --tailwind --eslint --app --src-dir --import-alias "@/*" --use-pnpm
```

Expected: `web/` populated with `package.json`, `tsconfig.json`, `next.config.mjs`, `tailwind.config.ts`, `src/app/layout.tsx`, `src/app/page.tsx`, `src/app/globals.css`.

- [ ] **Step 2: Pin Tailwind to v3.4 and add core deps**

```bash
cd web
pnpm add tailwindcss@3.4 @tanstack/react-query@5 zod@3 clsx tailwind-merge
pnpm add -D vitest @vitest/coverage-v8 jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event prettier eslint-config-prettier
```

Verify `package.json` has `"tailwindcss": "3.4.*"` (not v4).

- [ ] **Step 3: Replace `web/package.json` scripts**

```json
{
  "scripts": {
    "dev": "next dev -p 3000",
    "build": "next build",
    "start": "next start",
    "lint": "next lint",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest",
    "format": "prettier --write src"
  }
}
```

- [ ] **Step 4: Enable strict TS**

Edit `web/tsconfig.json` — set `"strict": true`, `"noUncheckedIndexedAccess": true`, `"noImplicitOverride": true`, `"forceConsistentCasingInFileNames": true`.

- [ ] **Step 5: Smoke-run dev**

```bash
pnpm dev
```

Open `http://localhost:3000` — should show the default Next.js page. Kill the server with Ctrl-C.

- [ ] **Step 6: Commit**

```bash
git add web/
git commit -m "feat(web): scaffold Next.js 14 app with TS, Tailwind 3.4, Vitest"
```

---

## Task 2: Port the prototype's Tailwind theme

**Files:**
- Modify: `web/tailwind.config.ts`
- Modify: `web/src/app/globals.css`

Reference: `solhub.zip/index.html` lines 11–57.

- [ ] **Step 1: Replace `web/tailwind.config.ts`**

```ts
import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['var(--font-inter)', 'ui-sans-serif', 'system-ui'],
        mono: ['var(--font-jetbrains-mono)', 'ui-monospace', 'monospace'],
        serif: ['"Instrument Serif"', 'serif'],
      },
      colors: {
        ink: { 950:'#0a0a0b', 900:'#171718', 800:'#27272a', 700:'#3f3f46', 600:'#52525b', 500:'#71717a', 400:'#a1a1aa', 300:'#d4d4d8', 200:'#e4e4e7', 100:'#f4f4f5', 50:'#fafafa' },
        violet: { 50:'#f5f3ff', 100:'#ede9fe', 200:'#ddd6fe', 400:'#a78bfa', 500:'#8b5cf6', 600:'#7c3aed', 700:'#6d28d9', 900:'#4c1d95' },
        sol: { green:'#14F195', purple:'#9945FF' },
      },
      boxShadow: {
        card: '0 1px 0 0 rgba(0,0,0,0.04), 0 1px 3px 0 rgba(24,24,27,0.04)',
        pop: '0 8px 24px -8px rgba(24,24,27,0.18), 0 2px 6px -2px rgba(24,24,27,0.08)',
        'inset-line': 'inset 0 -1px 0 0 #e4e4e7',
      },
    },
  },
  plugins: [],
};
export default config;
```

- [ ] **Step 2: Replace `web/src/app/globals.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

html, body, #__next { height: 100%; }
body {
  background: #fafafa;
  color: #0a0a0b;
  font-feature-settings: "cv11", "ss01", "ss03";
}

.grid-bg {
  background-image:
    linear-gradient(to right, rgba(24,24,27,0.045) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(24,24,27,0.045) 1px, transparent 1px);
  background-size: 24px 24px;
}
.dot-bg {
  background-image: radial-gradient(circle, rgba(24,24,27,0.18) 1px, transparent 1px);
  background-size: 20px 20px;
}
.scrollbar-thin::-webkit-scrollbar { width: 8px; height: 8px; }
.scrollbar-thin::-webkit-scrollbar-thumb { background: #d4d4d8; border-radius: 4px; }
.scrollbar-thin::-webkit-scrollbar-track { background: transparent; }

@keyframes flowDash { to { stroke-dashoffset: -24; } }
.edge-live { stroke-dasharray: 6 4; animation: flowDash 1s linear infinite; }

@keyframes pulse-ring {
  0%   { box-shadow: 0 0 0 0 rgba(20,241,149,0.45); }
  70%  { box-shadow: 0 0 0 10px rgba(20,241,149,0); }
  100% { box-shadow: 0 0 0 0 rgba(20,241,149,0); }
}
.ring-live { animation: pulse-ring 1.6s infinite; }

.no-scrollbar::-webkit-scrollbar { display: none; }
.no-scrollbar { scrollbar-width: none; }
```

- [ ] **Step 3: Wire fonts via next/font**

Replace `web/src/app/layout.tsx`:

```tsx
import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter", display: "swap" });
const jetbrainsMono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-jetbrains-mono", display: "swap" });

export const metadata: Metadata = {
  title: "Solhub — Solana Workflow OS",
  description: "Automation & execution reliability platform for Solana",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrainsMono.variable}`}>
      <body>{children}</body>
    </html>
  );
}
```

- [ ] **Step 4: Verify build**

```bash
pnpm build
```

Expected: clean build, no errors.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): port prototype theme tokens + fonts"
```

---

## Task 3: Vitest configuration

**Files:**
- Create: `web/vitest.config.ts`
- Create: `web/src/test/setup.ts`

- [ ] **Step 1: Create `web/vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
    css: false,
  },
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
});
```

Install the React plugin:
```bash
pnpm add -D @vitejs/plugin-react
```

- [ ] **Step 2: Create `web/src/test/setup.ts`**

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 3: Add a sentinel test**

Create `web/src/test/sanity.test.ts`:
```ts
import { describe, it, expect } from "vitest";
describe("sanity", () => {
  it("runs", () => { expect(1 + 1).toBe(2); });
});
```

- [ ] **Step 4: Verify**

```bash
pnpm test
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "test(web): vitest + jsdom setup"
```

---

## Task 4: `lib/utils/cn.ts`

**Files:**
- Create: `web/src/lib/utils/cn.ts`
- Test: `web/src/lib/utils/cn.test.ts`

- [ ] **Step 1: Write the failing test**

`web/src/lib/utils/cn.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { cn } from "./cn";

describe("cn", () => {
  it("joins truthy classes", () => {
    expect(cn("a", "b")).toBe("a b");
  });
  it("drops falsy values", () => {
    expect(cn("a", false, undefined, null, 0, "b")).toBe("a b");
  });
  it("merges conflicting Tailwind classes (later wins)", () => {
    expect(cn("px-2", "px-4")).toBe("px-4");
  });
});
```

- [ ] **Step 2: Verify it fails**

```bash
pnpm test src/lib/utils/cn.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

`web/src/lib/utils/cn.ts`:
```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 4: Verify pass**

```bash
pnpm test src/lib/utils/cn.test.ts
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): cn() helper"
```

---

## Task 5: `lib/utils/format.ts`

**Files:**
- Create: `web/src/lib/utils/format.ts`
- Test: `web/src/lib/utils/format.test.ts`

- [ ] **Step 1: Write the failing tests**

`web/src/lib/utils/format.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { formatLamports, formatUsdc, formatAddress, formatRelativeTime, formatSlot } from "./format";

describe("formatLamports", () => {
  it("formats with 9 decimals and SOL suffix", () => {
    expect(formatLamports(1_500_000_000n)).toBe("1.5 SOL");
    expect(formatLamports(1_000n)).toBe("0.000001 SOL");
    expect(formatLamports(0n)).toBe("0 SOL");
  });
});

describe("formatUsdc", () => {
  it("formats USDC with 6 decimals", () => {
    expect(formatUsdc(1_000_000n)).toBe("1.00 USDC");
    expect(formatUsdc(12_345_678n)).toBe("12.35 USDC");
  });
});

describe("formatAddress", () => {
  it("shortens to abc…xyz", () => {
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
    const now = new Date();
    expect(formatRelativeTime(now)).toBe("just now");
  });
});
```

- [ ] **Step 2: Implement**

`web/src/lib/utils/format.ts`:
```ts
export function formatLamports(lamports: bigint): string {
  if (lamports === 0n) return "0 SOL";
  const sol = Number(lamports) / 1e9;
  if (sol < 0.001) return `${sol.toFixed(9).replace(/0+$/, "").replace(/\.$/, "")} SOL`;
  return `${parseFloat(sol.toFixed(6))} SOL`;
}

export function formatUsdc(microUsdc: bigint): string {
  const usdc = Number(microUsdc) / 1e6;
  return `${usdc.toFixed(2)} USDC`;
}

export function formatAddress(addr: string, head = 5, tail = 4): string {
  if (addr.length <= head + tail + 1) return addr;
  return `${addr.slice(0, head)}…${addr.slice(-tail)}`;
}

export function formatSlot(slot: number | bigint): string {
  return Number(slot).toLocaleString("en-US");
}

export function formatRelativeTime(d: Date | string): string {
  const date = typeof d === "string" ? new Date(d) : d;
  const diffMs = Date.now() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 60) return "just now";
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
}

export function solscanTx(sig: string, network: "mainnet" | "devnet" = "mainnet") {
  const suffix = network === "devnet" ? "?cluster=devnet" : "";
  return `https://solscan.io/tx/${sig}${suffix}`;
}
export function solscanAccount(addr: string, network: "mainnet" | "devnet" = "mainnet") {
  const suffix = network === "devnet" ? "?cluster=devnet" : "";
  return `https://solscan.io/account/${addr}${suffix}`;
}
```

- [ ] **Step 3: Verify**

```bash
pnpm test src/lib/utils/format.test.ts
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): formatting helpers (lamports, USDC, address, slot, time)"
```

---

## Task 6: Zod schemas

**Files:**
- Create: `web/src/lib/api/schemas.ts`
- Test: `web/src/lib/api/schemas.test.ts`

Reference: `IDEA.md` §4.2 (workflow types), §4.3 (run types), §8.2 (request/response types), §11 (DB shape).

- [ ] **Step 1: Implement**

`web/src/lib/api/schemas.ts`:
```ts
import { z } from "zod";

export const TriggerConfigSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("cron"), schedule: z.string() }),
  z.object({
    type: z.literal("account_watch"),
    account: z.string(),
    condition: z.discriminatedUnion("kind", [
      z.object({ kind: z.literal("balance_above"), lamports: z.coerce.bigint() }),
      z.object({ kind: z.literal("balance_below"), lamports: z.coerce.bigint() }),
      z.object({ kind: z.literal("data_changes") }),
      z.object({ kind: z.literal("program_log"), pattern: z.string() }),
    ]),
  }),
  z.object({ type: z.literal("webhook"), secret: z.string() }),
]);
export type TriggerConfig = z.infer<typeof TriggerConfigSchema>;

export const OnErrorSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("abort") }),
  z.object({ kind: z.literal("skip") }),
  z.object({ kind: z.literal("retry"), max_attempts: z.number().int().positive() }),
]);
export type OnError = z.infer<typeof OnErrorSchema>;

export const WorkflowStepSchema = z.object({
  id: z.string(),
  plugin: z.string(),
  action: z.string(),
  params: z.record(z.unknown()),
  condition: z.string().nullable().optional(),
  on_error: OnErrorSchema,
});
export type WorkflowStep = z.infer<typeof WorkflowStepSchema>;

export const WorkflowSchema = z.object({
  id: z.string().uuid(),
  org_id: z.string().uuid(),
  name: z.string(),
  trigger: TriggerConfigSchema,
  steps: z.array(WorkflowStepSchema),
  is_active: z.boolean(),
  is_public: z.boolean().optional(),
  onchain_pda: z.string().nullable().optional(),
  fee_per_exec_usdc: z.coerce.bigint().nullable().optional(),
  execution_count: z.coerce.bigint().default(0n),
  created_at: z.string(),
});
export type Workflow = z.infer<typeof WorkflowSchema>;

export const RunStatusSchema = z.enum([
  "pending", "triggered", "simulating", "bundling",
  "submitted", "confirmed", "retrying", "failed", "skipped",
]);
export type RunStatus = z.infer<typeof RunStatusSchema>;

export const StepLogSchema = z.object({
  step_id: z.string(),
  status: z.enum(["pending", "running", "success", "failed", "skipped"]),
  input: z.unknown(),
  output: z.unknown(),
  duration_ms: z.number().int().nonnegative(),
  error: z.string().nullable().optional(),
});
export type StepLog = z.infer<typeof StepLogSchema>;

export const WorkflowRunSchema = z.object({
  run_id: z.string().uuid(),
  workflow_id: z.string().uuid(),
  triggered_by: z.enum(["cron", "account_watch", "webhook", "manual", "mcp"]),
  status: RunStatusSchema,
  steps: z.array(StepLogSchema).default([]),
  started_at: z.string(),
  completed_at: z.string().nullable().optional(),
  slot: z.coerce.bigint().nullable().optional(),
  signature: z.string().nullable().optional(),
  fee_lamports: z.coerce.bigint().nullable().optional(),
  jito_tip_lamports: z.coerce.bigint().nullable().optional(),
  error: z.string().nullable().optional(),
});
export type WorkflowRun = z.infer<typeof WorkflowRunSchema>;

export const RunLogEventSchema = z.object({
  event: z.enum(["step_start", "step_complete", "run_complete", "error"]),
  step_id: z.string().nullable().optional(),
  data: z.unknown(),
  timestamp: z.string(),
});
export type RunLogEvent = z.infer<typeof RunLogEventSchema>;

export const HubWorkflowSchema = z.object({
  id: z.string(),
  name: z.string(),
  author: z.string(),
  description: z.string(),
  protocols: z.array(z.string()),
  fee_usdc: z.string(),
  runs: z.number().int().nonnegative(),
  success_rate: z.string(),
  verified: z.boolean(),
  audited: z.boolean(),
  apy: z.string().nullable().optional(),
});
export type HubWorkflow = z.infer<typeof HubWorkflowSchema>;

export const OrgSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  wallet_address: z.string().nullable(),
  credits_usdc: z.coerce.bigint(),
  created_at: z.string(),
});
export type Org = z.infer<typeof OrgSchema>;

export const ApiKeySchema = z.object({
  id: z.string().uuid(),
  name: z.string().nullable(),
  last_used_at: z.string().nullable(),
  created_at: z.string(),
  revoked_at: z.string().nullable(),
});
export type ApiKey = z.infer<typeof ApiKeySchema>;

export const AnalyticsSchema = z.object({
  range: z.string(),
  executions: z.number(),
  success_rate: z.number(),
  fee_spend_lamports: z.coerce.bigint(),
  credits_remaining: z.coerce.bigint(),
});
export type Analytics = z.infer<typeof AnalyticsSchema>;
```

- [ ] **Step 2: Test**

`web/src/lib/api/schemas.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { TriggerConfigSchema, WorkflowSchema, RunLogEventSchema } from "./schemas";

describe("TriggerConfigSchema", () => {
  it("parses a cron trigger", () => {
    const r = TriggerConfigSchema.parse({ type: "cron", schedule: "*/5 * * * *" });
    expect(r.type).toBe("cron");
  });
  it("rejects unknown type", () => {
    expect(() => TriggerConfigSchema.parse({ type: "bogus" })).toThrow();
  });
});

describe("WorkflowSchema", () => {
  it("parses a minimal workflow", () => {
    const w = WorkflowSchema.parse({
      id: "11111111-1111-1111-1111-111111111111",
      org_id: "22222222-2222-2222-2222-222222222222",
      name: "test",
      trigger: { type: "cron", schedule: "0 * * * *" },
      steps: [],
      is_active: true,
      execution_count: "0",
      created_at: "2026-05-11T00:00:00Z",
    });
    expect(w.execution_count).toBe(0n);
  });
});

describe("RunLogEventSchema", () => {
  it("parses a step_start event", () => {
    const e = RunLogEventSchema.parse({
      event: "step_start",
      step_id: "s1",
      data: {},
      timestamp: "2026-05-11T00:00:00Z",
    });
    expect(e.event).toBe("step_start");
  });
});
```

- [ ] **Step 3: Run**

```bash
pnpm test src/lib/api/schemas.test.ts
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add web/
git commit -m "feat(web): Zod schemas for workflows, runs, hub, org"
```

---

## Task 7: API client

**Files:**
- Create: `web/src/lib/api/client.ts`
- Test: `web/src/lib/api/client.test.ts`

- [ ] **Step 1: Implement**

`web/src/lib/api/client.ts`:
```ts
import { z } from "zod";

export class ApiError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}

const BEARER_KEY = "solhub.bearer";

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return window.localStorage.getItem(BEARER_KEY);
}
export function setToken(token: string): void {
  window.localStorage.setItem(BEARER_KEY, token);
}
export function clearToken(): void {
  window.localStorage.removeItem(BEARER_KEY);
}

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";

export interface RequestOpts {
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  body?: unknown;
  query?: Record<string, string | number | boolean | undefined>;
  /** When true, do NOT attach the Bearer token (used for public Hub endpoints). */
  anonymous?: boolean;
}

export async function apiRequest<T>(
  path: string,
  schema: z.ZodSchema<T>,
  opts: RequestOpts = {},
): Promise<T> {
  const url = new URL(path, API_BASE);
  if (opts.query) {
    for (const [k, v] of Object.entries(opts.query)) {
      if (v !== undefined) url.searchParams.set(k, String(v));
    }
  }

  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (!opts.anonymous) {
    const tok = getToken();
    if (tok) headers["Authorization"] = `Bearer ${tok}`;
  }

  const res = await fetch(url.toString(), {
    method: opts.method ?? "GET",
    headers,
    body: opts.body ? JSON.stringify(opts.body) : undefined,
  });

  if (res.status === 401 && !opts.anonymous) {
    clearToken();
    if (typeof window !== "undefined") window.location.href = "/login";
    throw new ApiError(401, "unauthorized", "Session expired");
  }

  if (!res.ok) {
    let code = "http_error";
    let message = res.statusText;
    try {
      const j = await res.json();
      code = j.code ?? code;
      message = j.message ?? message;
    } catch { /* response was not JSON */ }
    throw new ApiError(res.status, code, message);
  }

  // 204 No Content
  if (res.status === 204) return schema.parse(undefined);

  const body = await res.json();
  return schema.parse(body);
}
```

- [ ] **Step 2: Test**

`web/src/lib/api/client.test.ts`:
```ts
import { describe, it, expect, beforeEach, vi } from "vitest";
import { z } from "zod";
import { apiRequest, ApiError, setToken, clearToken } from "./client";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  clearToken();
});

const OkSchema = z.object({ ok: z.boolean() });

describe("apiRequest", () => {
  it("attaches Bearer token", async () => {
    setToken("test-key");
    F.mockResolvedValueOnce({ ok: true, status: 200, json: async () => ({ ok: true }) } as Response);
    await apiRequest("/v1/ping", OkSchema);
    const [, init] = F.mock.calls[0];
    expect((init as RequestInit).headers).toMatchObject({ Authorization: "Bearer test-key" });
  });

  it("omits Bearer for anonymous", async () => {
    setToken("test-key");
    F.mockResolvedValueOnce({ ok: true, status: 200, json: async () => ({ ok: true }) } as Response);
    await apiRequest("/v1/hub", OkSchema, { anonymous: true });
    const [, init] = F.mock.calls[0];
    expect((init as RequestInit).headers).not.toHaveProperty("Authorization");
  });

  it("throws ApiError on non-2xx", async () => {
    F.mockResolvedValueOnce({
      ok: false, status: 404, statusText: "Not Found",
      json: async () => ({ code: "not_found", message: "missing" }),
    } as Response);
    await expect(apiRequest("/v1/x", OkSchema)).rejects.toBeInstanceOf(ApiError);
  });
});
```

- [ ] **Step 3: Run & commit**

```bash
pnpm test src/lib/api/client.test.ts
git add web/
git commit -m "feat(web): typed API client with Zod parsing + auth"
```

---

## Task 8: API modules

**Files:**
- Create: `web/src/lib/api/orgs.ts`
- Create: `web/src/lib/api/workflows.ts`
- Create: `web/src/lib/api/runs.ts`
- Create: `web/src/lib/api/hub.ts`
- Create: `web/src/lib/api/analytics.ts`
- Create: `web/src/lib/api/index.ts`

Note: SSE is implemented in Phase B (`useRunStream`). Here we only ship the REST surface.

- [ ] **Step 1: `orgs.ts`**

```ts
import { apiRequest } from "./client";
import { OrgSchema, ApiKeySchema, type ApiKey, type Org } from "./schemas";
import { z } from "zod";

export const getMe = () => apiRequest("/v1/orgs/me", OrgSchema);

export const listApiKeys = () =>
  apiRequest("/v1/orgs/me/api_keys", z.array(ApiKeySchema));

const CreateApiKeyResponse = z.object({
  api_key: ApiKeySchema,
  /** Raw key, shown to user ONCE — per IDEA.md Non-Negotiable Rule #8 */
  raw_key: z.string(),
});
export const createApiKey = (name: string) =>
  apiRequest("/v1/orgs/me/api_keys", CreateApiKeyResponse, {
    method: "POST",
    body: { name },
  });

export const revokeApiKey = (id: string) =>
  apiRequest(`/v1/orgs/me/api_keys/${id}`, z.void(), { method: "DELETE" });
```

- [ ] **Step 2: `workflows.ts`**

```ts
import { apiRequest } from "./client";
import { WorkflowSchema, TriggerConfigSchema, WorkflowStepSchema, type Workflow } from "./schemas";
import { z } from "zod";

const CreateWorkflowResponse = z.object({
  workflow_id: z.string().uuid(),
  status: z.string(),
  next_run: z.string().nullable().optional(),
  onchain_pda: z.string().nullable().optional(),
});

export interface ListWorkflowsParams {
  status?: "active" | "inactive" | "all";
  trigger_type?: "cron" | "account_watch" | "webhook";
  limit?: number;
}

export const listWorkflows = (params: ListWorkflowsParams = {}) =>
  apiRequest("/v1/workflows", z.array(WorkflowSchema), { query: params });

export const getWorkflow = (id: string) =>
  apiRequest(`/v1/workflows/${id}`, WorkflowSchema);

export interface CreateWorkflowBody {
  name: string;
  trigger: z.infer<typeof TriggerConfigSchema>;
  steps: z.infer<typeof WorkflowStepSchema>[];
  fee_per_execution_usdc?: number;
  is_public?: boolean;
}

export const createWorkflow = (body: CreateWorkflowBody) =>
  apiRequest("/v1/workflows", CreateWorkflowResponse, { method: "POST", body });

export const updateWorkflow = (id: string, body: Partial<CreateWorkflowBody> & { is_active?: boolean }) =>
  apiRequest(`/v1/workflows/${id}`, WorkflowSchema, { method: "PATCH", body });

export const deleteWorkflow = (id: string) =>
  apiRequest(`/v1/workflows/${id}`, z.void(), { method: "DELETE" });

const TriggerResponse = z.object({
  run_id: z.string().uuid(),
  status: z.string(),
  estimated_slot: z.coerce.bigint().nullable().optional(),
});
export const triggerWorkflow = (id: string, param_overrides: Record<string, unknown> = {}) =>
  apiRequest(`/v1/workflows/${id}/trigger`, TriggerResponse, {
    method: "POST",
    body: param_overrides,
  });
```

- [ ] **Step 3: `runs.ts`**

```ts
import { apiRequest } from "./client";
import { WorkflowRunSchema } from "./schemas";
import { z } from "zod";

export interface ListRunsParams {
  workflow_id?: string;
  status?: string;
  from?: string;
  to?: string;
  limit?: number;
}

export const listRuns = (params: ListRunsParams = {}) =>
  apiRequest("/v1/runs", z.array(WorkflowRunSchema), { query: params });

export const getRun = (run_id: string) =>
  apiRequest(`/v1/runs/${run_id}`, WorkflowRunSchema);

/**
 * Builds the SSE URL for a run's log stream.
 * Uses `?token=<bearer>` because EventSource cannot send custom headers.
 * Requires Backend Gap #1 (see spec §9).
 */
export function runStreamUrl(run_id: string): string {
  const base = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";
  const token = typeof window !== "undefined"
    ? window.localStorage.getItem("solhub.bearer")
    : null;
  const url = new URL(`/v1/runs/${run_id}/logs`, base);
  if (token) url.searchParams.set("token", token);
  return url.toString();
}
```

- [ ] **Step 4: `hub.ts`**

```ts
import { apiRequest } from "./client";
import { HubWorkflowSchema, WorkflowRunSchema } from "./schemas";
import { z } from "zod";

export const listHub = () =>
  apiRequest("/v1/hub", z.array(HubWorkflowSchema), { anonymous: true });

// Backend Gap #3 — see spec §9
export const getHubWorkflow = (id: string) =>
  apiRequest(`/v1/hub/${id}`, HubWorkflowSchema, { anonymous: true });

export const callHubWorkflow = (id: string, params: Record<string, unknown> = {}) =>
  apiRequest(`/v1/hub/${id}/call`, z.object({ run_id: z.string().uuid() }), {
    method: "POST",
    body: params,
  });
```

- [ ] **Step 5: `analytics.ts`**

```ts
import { apiRequest } from "./client";
import { AnalyticsSchema } from "./schemas";

export const getAnalytics = (range: "1d" | "7d" | "30d" = "7d") =>
  apiRequest("/v1/analytics", AnalyticsSchema, { query: { range } });
```

- [ ] **Step 6: `index.ts`**

```ts
export * as orgs from "./orgs";
export * as workflows from "./workflows";
export * as runs from "./runs";
export * as hub from "./hub";
export * as analytics from "./analytics";
export * from "./client";
export * from "./schemas";
```

- [ ] **Step 7: Typecheck + commit**

```bash
pnpm typecheck
git add web/
git commit -m "feat(web): API modules (orgs, workflows, runs, hub, analytics)"
```

---

## Task 9: Auth store + hook

**Files:**
- Create: `web/src/lib/auth/use-auth.ts`
- Test: `web/src/lib/auth/use-auth.test.tsx`

- [ ] **Step 1: Implement**

```tsx
"use client";
import { useEffect, useState } from "react";
import { getToken, setToken as setStorageToken, clearToken as clearStorageToken } from "@/lib/api/client";

export function useAuth() {
  const [token, setTokenState] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setTokenState(getToken());
    setReady(true);
    const onStorage = (e: StorageEvent) => {
      if (e.key === "solhub.bearer") setTokenState(e.newValue);
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  return {
    token,
    isAuthenticated: !!token,
    ready,
    signIn: (t: string) => { setStorageToken(t); setTokenState(t); },
    signOut: () => { clearStorageToken(); setTokenState(null); },
  };
}
```

- [ ] **Step 2: Test**

```tsx
import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAuth } from "./use-auth";

beforeEach(() => { window.localStorage.clear(); });

describe("useAuth", () => {
  it("starts unauthenticated", () => {
    const { result } = renderHook(() => useAuth());
    expect(result.current.isAuthenticated).toBe(false);
  });
  it("signIn persists token", () => {
    const { result } = renderHook(() => useAuth());
    act(() => result.current.signIn("abc123"));
    expect(result.current.isAuthenticated).toBe(true);
    expect(window.localStorage.getItem("solhub.bearer")).toBe("abc123");
  });
});
```

- [ ] **Step 3: Commit**

```bash
pnpm test src/lib/auth/use-auth.test.tsx
git add web/
git commit -m "feat(web): useAuth hook backed by localStorage"
```

---

## Task 10: `Icon` primitive

**Files:**
- Create: `web/src/components/primitives/Icon.tsx`
- Test: `web/src/components/primitives/Icon.test.tsx`

Reference: `solhub.zip/lib.jsx` lines 3–62. Port the full icon set.

- [ ] **Step 1: Implement**

`web/src/components/primitives/Icon.tsx`:
```tsx
import { cn } from "@/lib/utils/cn";

const PATHS: Record<string, React.ReactNode> = {
  dashboard: <><rect x="3" y="3" width="7" height="9" rx="1.5"/><rect x="14" y="3" width="7" height="5" rx="1.5"/><rect x="14" y="12" width="7" height="9" rx="1.5"/><rect x="3" y="16" width="7" height="5" rx="1.5"/></>,
  workflows: <><circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M8 7l3 9M16 7l-3 9"/></>,
  builder: <><path d="M4 6h7M13 6h7M4 12h7M13 18h7M4 18h7M13 12h7"/><circle cx="11" cy="6" r="1.5"/><circle cx="13" cy="12" r="1.5"/><circle cx="11" cy="18" r="1.5"/></>,
  runs: <><path d="M5 4l4 4-4 4M9 8h7a4 4 0 014 4v0a4 4 0 01-4 4H5"/></>,
  marketplace: <><path d="M3 7l1.5-3h15L21 7M3 7v12a1 1 0 001 1h16a1 1 0 001-1V7M3 7h18M9 11a3 3 0 006 0"/></>,
  wallet: <><rect x="3" y="6" width="18" height="13" rx="2"/><path d="M3 9h15a3 3 0 013 3v0a3 3 0 01-3 3H3"/><circle cx="17" cy="12" r="1" fill="currentColor" stroke="none"/></>,
  versions: <><circle cx="6" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="12" r="2"/><path d="M6 8v8M8 6h6a4 4 0 014 4M8 18h6a4 4 0 004-4"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"/></>,
  ai: <><path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z"/><circle cx="18" cy="18" r="2"/></>,
  bell: <><path d="M6 8a6 6 0 0112 0c0 7 3 8 3 8H3s3-1 3-8M10 21a2 2 0 004 0"/></>,
  search: <><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></>,
  plus: <path d="M12 5v14M5 12h14"/>,
  play: <path d="M7 4l13 8-13 8z" fill="currentColor" stroke="none"/>,
  pause: <><rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none"/><rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none"/></>,
  check: <path d="M5 12l4 4 10-10"/>,
  x: <path d="M5 5l14 14M19 5L5 19"/>,
  arrow: <path d="M5 12h14M13 6l6 6-6 6"/>,
  chevron: <path d="M9 6l6 6-6 6"/>,
  chevronDown: <path d="M6 9l6 6 6-6"/>,
  dot: <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/>,
  bolt: <path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" fill="currentColor" stroke="none"/>,
  clock: <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
  eye: <><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></>,
  code: <path d="M9 8l-5 4 5 4M15 8l5 4-5 4M14 4l-4 16"/>,
  db: <><ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/></>,
  flow: <><circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8 11l8-4M8 13l8 4"/></>,
  filter: <path d="M3 5h18l-7 9v6l-4-2v-4z"/>,
  git: <><circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><circle cx="18" cy="12" r="2.5"/><path d="M6 8v8M8 6h6a4 4 0 014 4"/></>,
  spark: <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M5.6 18.4l2.8-2.8M15.6 8.4l2.8-2.8"/>,
  upload: <path d="M12 17V3M5 10l7-7 7 7M3 21h18"/>,
  download: <path d="M12 3v14M5 14l7 7 7-7M3 21h18"/>,
  copy: <><rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v9a2 2 0 002 2h3"/></>,
  trash: <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13a2 2 0 002 2h6a2 2 0 002-2l1-13M9 7V4a1 1 0 011-1h4a1 1 0 011 1v3"/>,
  expand: <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>,
  drag: <><circle cx="9" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="18" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="18" r="1" fill="currentColor" stroke="none"/></>,
  refresh: <path d="M3 12a9 9 0 0115-6.7L21 8M21 3v5h-5M21 12a9 9 0 01-15 6.7L3 16M3 21v-5h5"/>,
  sliders: <><path d="M4 6h10M18 6h2M4 12h2M10 12h10M4 18h12M20 18h-2"/><circle cx="16" cy="6" r="2"/><circle cx="8" cy="12" r="2"/><circle cx="18" cy="18" r="2"/></>,
  layers: <path d="M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5M3 18l9 5 9-5"/>,
  bug: <><rect x="8" y="6" width="8" height="14" rx="4"/><path d="M8 12H4M16 12h4M9 6c0-1.7 1.3-3 3-3s3 1.3 3 3M5 4l3 3M19 4l-3 3M5 20l3-3M19 20l-3-3"/></>,
  cloud: <path d="M7 18a5 5 0 010-10 6 6 0 0111 1 4 4 0 010 8z"/>,
  shield: <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z"/>,
  key: <><circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M14 9l3 3"/></>,
  logout: <path d="M15 3h3a2 2 0 012 2v14a2 2 0 01-2 2h-3M10 17l5-5-5-5M15 12H3"/>,
  bookmark: <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/>,
  star: <path d="M12 2l3 7h7l-5.5 4 2 7-6.5-4-6.5 4 2-7L2 9h7z"/>,
  coins: <><circle cx="9" cy="9" r="6"/><path d="M22 13.6A6 6 0 1116.4 8M5.5 13.5l3 3"/></>,
  info: <><circle cx="12" cy="12" r="9"/><path d="M12 8h.01M11 12h1v4h1"/></>,
  warn: <path d="M12 3l10 18H2L12 3zM12 10v5M12 18h.01"/>,
  fire: <path d="M12 22c4 0 7-3 7-7 0-4-3-5-3-9 0 0-2 1-4 5-1-2-2-3-2-3s-5 4-5 9 3 5 7 5z"/>,
};

export type IconName = keyof typeof PATHS;

export interface IconProps extends React.SVGProps<SVGSVGElement> {
  name: IconName;
  className?: string;
  stroke?: number;
}

export function Icon({ name, className, stroke = 1.6, ...rest }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("w-4 h-4", className)}
      aria-hidden="true"
      {...rest}
    >
      {PATHS[name]}
    </svg>
  );
}
```

- [ ] **Step 2: Test**

```tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { Icon } from "./Icon";

describe("Icon", () => {
  it("renders an SVG for a known name", () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector("svg")).toBeInTheDocument();
  });
  it("applies className", () => {
    const { container } = render(<Icon name="search" className="w-8 h-8" />);
    expect(container.querySelector("svg")?.className.baseVal).toContain("w-8");
  });
});
```

- [ ] **Step 3: Commit**

```bash
pnpm test src/components/primitives/Icon.test.tsx
git add web/
git commit -m "feat(web): Icon primitive ported from prototype"
```

---

## Task 11: `Btn`, `Pill`, `Kbd` primitives

**Files:**
- Create: `web/src/components/primitives/Btn.tsx`
- Create: `web/src/components/primitives/Pill.tsx`
- Create: `web/src/components/primitives/Kbd.tsx`

Reference: `solhub.zip/lib.jsx` lines 91–124.

- [ ] **Step 1: `Btn.tsx`**

```tsx
import * as React from "react";
import { cn } from "@/lib/utils/cn";

const SIZES = {
  sm: "h-7 px-2.5 text-[12px]",
  md: "h-8 px-3 text-[13px]",
  lg: "h-10 px-4 text-[14px]",
} as const;

const VARIANTS = {
  default: "bg-white hover:bg-ink-50 border-ink-200 text-ink-900",
  primary: "bg-ink-950 hover:bg-ink-900 border-ink-950 text-white",
  accent: "bg-violet-600 hover:bg-violet-700 border-violet-600 text-white",
  success: "bg-emerald-600 hover:bg-emerald-700 border-emerald-600 text-white",
  ghost: "bg-transparent hover:bg-ink-100 border-transparent text-ink-700",
  danger: "bg-white hover:bg-rose-50 border-rose-200 text-rose-700",
} as const;

export interface BtnProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof VARIANTS;
  size?: keyof typeof SIZES;
  icon?: React.ReactNode;
}

export function Btn({ children, variant = "default", size = "md", icon, className, ...rest }: BtnProps) {
  return (
    <button
      {...rest}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md border font-medium transition-colors",
        SIZES[size],
        VARIANTS[variant],
        className,
      )}
    >
      {icon}
      {children}
    </button>
  );
}
```

- [ ] **Step 2: `Pill.tsx`**

```tsx
import * as React from "react";
import { cn } from "@/lib/utils/cn";

const TONES = {
  ink: "bg-ink-100 text-ink-700 border-ink-200",
  violet: "bg-violet-50 text-violet-700 border-violet-200",
  emerald: "bg-emerald-50 text-emerald-700 border-emerald-200",
  amber: "bg-amber-50 text-amber-700 border-amber-200",
  rose: "bg-rose-50 text-rose-700 border-rose-200",
  cyan: "bg-cyan-50 text-cyan-700 border-cyan-200",
  sol: "bg-gradient-to-r from-sol-purple/10 to-sol-green/10 text-ink-900 border-ink-200",
} as const;

export function Pill({
  children, tone = "ink", className,
}: { children: React.ReactNode; tone?: keyof typeof TONES; className?: string }) {
  return (
    <span className={cn(
      "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-[11px] font-medium font-mono",
      TONES[tone], className,
    )}>{children}</span>
  );
}
```

- [ ] **Step 3: `Kbd.tsx`**

```tsx
export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex items-center justify-center min-w-[18px] h-[18px] px-1 rounded border border-ink-200 bg-white text-[10px] font-mono text-ink-600 shadow-sm">
      {children}
    </kbd>
  );
}
```

- [ ] **Step 4: Snapshot smoke test**

`web/src/components/primitives/primitives.test.tsx`:
```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Btn } from "./Btn";
import { Pill } from "./Pill";
import { Kbd } from "./Kbd";

describe("primitives", () => {
  it("Btn renders children", () => {
    render(<Btn>Click</Btn>);
    expect(screen.getByRole("button", { name: "Click" })).toBeInTheDocument();
  });
  it("Pill applies tone class", () => {
    const { container } = render(<Pill tone="violet">v</Pill>);
    expect(container.firstChild).toHaveClass("bg-violet-50");
  });
  it("Kbd renders", () => {
    render(<Kbd>⌘K</Kbd>);
    expect(screen.getByText("⌘K")).toBeInTheDocument();
  });
});
```

- [ ] **Step 5: Run + commit**

```bash
pnpm test src/components/primitives/primitives.test.tsx
git add web/
git commit -m "feat(web): Btn, Pill, Kbd primitives"
```

---

## Task 12: Logo primitives

**Files:**
- Create: `web/src/components/primitives/SolhubLogo.tsx`
- Create: `web/src/components/primitives/SolanaMark.tsx`

Reference: `solhub.zip/lib.jsx` lines 64–87.

- [ ] **Step 1: `SolanaMark.tsx`**

```tsx
export function SolanaMark({ className = "w-5 h-5" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" aria-hidden="true">
      <defs>
        <linearGradient id="solg" x1="0" y1="0" x2="24" y2="24">
          <stop offset="0" stopColor="#9945FF"/>
          <stop offset="1" stopColor="#14F195"/>
        </linearGradient>
      </defs>
      <path d="M4 7l4-3h12l-4 3H4zM4 13l4-3h12l-4 3H4zM4 19l4-3h12l-4 3H4z" fill="url(#solg)"/>
    </svg>
  );
}
```

- [ ] **Step 2: `SolhubLogo.tsx`**

```tsx
import { cn } from "@/lib/utils/cn";

export function SolhubLogo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2", className)}>
      <div className="w-7 h-7 rounded-lg bg-ink-950 flex items-center justify-center relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-sol-purple/60 to-sol-green/40" />
        <span className="relative font-mono text-white text-[13px] font-semibold tracking-tight">sh</span>
      </div>
      <div className="leading-none">
        <div className="text-[15px] font-semibold tracking-tight">solhub</div>
        <div className="text-[9px] font-mono text-ink-500 uppercase tracking-[0.18em]">workflow os</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): SolhubLogo + SolanaMark"
```

---

## Task 13: QueryClient Provider

**Files:**
- Create: `web/src/components/Providers.tsx`
- Modify: `web/src/app/layout.tsx`

- [ ] **Step 1: `Providers.tsx`**

```tsx
"use client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

export function Providers({ children }: { children: React.ReactNode }) {
  const [qc] = useState(() => new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        gcTime: 5 * 60_000,
        retry: 1,
        refetchOnWindowFocus: false,
      },
    },
  }));
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}
```

- [ ] **Step 2: Wire into layout**

Modify `web/src/app/layout.tsx` `<body>`:
```tsx
<body>
  <Providers>{children}</Providers>
</body>
```

(Add `import { Providers } from "@/components/Providers";` at top.)

- [ ] **Step 3: Build + commit**

```bash
pnpm build
git add web/
git commit -m "feat(web): TanStack Query provider"
```

---

## Task 14: Sidebar

**Files:**
- Create: `web/src/components/shell/Sidebar.tsx`
- Test: `web/src/components/shell/Sidebar.test.tsx`

- [ ] **Step 1: Implement**

```tsx
"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Icon, type IconName } from "@/components/primitives/Icon";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { cn } from "@/lib/utils/cn";

const NAV: { href: string; label: string; icon: IconName }[] = [
  { href: "/dashboard",   label: "Dashboard",   icon: "dashboard" },
  { href: "/workflows",   label: "Workflows",   icon: "workflows" },
  { href: "/ai",          label: "AI Builder",  icon: "ai" },
  { href: "/runs",        label: "Runs & Logs", icon: "runs" },
  { href: "/marketplace", label: "Marketplace", icon: "marketplace" },
  { href: "/wallet",      label: "Wallet",      icon: "wallet" },
  { href: "/versions",    label: "Versions",    icon: "versions" },
  { href: "/settings",    label: "Settings",    icon: "settings" },
];

export function Sidebar() {
  const pathname = usePathname();
  return (
    <aside className="w-56 shrink-0 h-screen border-r border-ink-200 bg-white flex flex-col">
      <div className="h-14 px-4 flex items-center border-b border-ink-200">
        <SolhubLogo />
      </div>
      <nav className="flex-1 p-2 space-y-0.5 overflow-y-auto scrollbar-thin">
        {NAV.map((n) => {
          const active = pathname?.startsWith(n.href) ?? false;
          return (
            <Link
              key={n.href}
              href={n.href}
              className={cn(
                "flex items-center gap-2.5 px-2.5 h-8 rounded-md text-[13px] font-medium",
                active
                  ? "bg-ink-100 text-ink-900"
                  : "text-ink-600 hover:bg-ink-50 hover:text-ink-900",
              )}
            >
              <Icon name={n.icon} className="w-4 h-4" />
              {n.label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
```

- [ ] **Step 2: Test**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "./Sidebar";

vi.mock("next/navigation", () => ({ usePathname: () => "/workflows" }));

describe("Sidebar", () => {
  it("renders all nav items", () => {
    render(<Sidebar />);
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Workflows")).toBeInTheDocument();
    expect(screen.getByText("Marketplace")).toBeInTheDocument();
  });
  it("highlights the active route", () => {
    render(<Sidebar />);
    const active = screen.getByText("Workflows").closest("a");
    expect(active?.className).toContain("bg-ink-100");
  });
});
```

- [ ] **Step 3: Commit**

```bash
pnpm test src/components/shell/Sidebar.test.tsx
git add web/
git commit -m "feat(web): Sidebar with route highlighting"
```

---

## Task 15: Topbar + Breadcrumb

**Files:**
- Create: `web/src/components/shell/Breadcrumb.tsx`
- Create: `web/src/components/shell/Topbar.tsx`

- [ ] **Step 1: `Breadcrumb.tsx`**

```tsx
import { Icon } from "@/components/primitives/Icon";

export function Breadcrumb({ items }: { items: string[] }) {
  return (
    <div className="flex items-center text-[12px] text-ink-500 gap-1.5">
      {items.map((item, i) => (
        <span key={i} className="flex items-center gap-1.5">
          {i > 0 && <Icon name="chevron" className="w-3 h-3 text-ink-300" />}
          <span className={i === items.length - 1 ? "text-ink-900 font-medium" : ""}>
            {item}
          </span>
        </span>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: `Topbar.tsx`**

```tsx
"use client";
import { Breadcrumb } from "./Breadcrumb";
import { Icon } from "@/components/primitives/Icon";
import { Kbd } from "@/components/primitives/Kbd";

export function Topbar({ crumbs, right }: { crumbs: string[]; right?: React.ReactNode }) {
  return (
    <header className="h-14 px-6 border-b border-ink-200 bg-white flex items-center justify-between">
      <Breadcrumb items={crumbs} />
      <div className="flex items-center gap-2">
        {right}
        <div className="flex items-center h-8 rounded-md border border-ink-200 bg-ink-50">
          <Icon name="search" className="w-3.5 h-3.5 text-ink-400 ml-2.5" />
          <input
            placeholder="Search workspace…"
            className="px-2 w-64 text-[12px] focus:outline-none bg-transparent"
          />
          <span className="mr-2"><Kbd>⌘K</Kbd></span>
        </div>
      </div>
    </header>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): Topbar + Breadcrumb shell components"
```

---

## Task 16: Route groups + auth gate

**Files:**
- Delete: `web/src/app/page.tsx` (default scaffold)
- Create: `web/src/app/(auth)/layout.tsx`
- Create: `web/src/app/(auth)/login/page.tsx` (placeholder — full impl in Phase B)
- Create: `web/src/app/(app)/layout.tsx`
- Create: `web/src/app/(app)/dashboard/page.tsx` (placeholder)
- Create: `web/src/app/page.tsx` (redirect to /dashboard)

- [ ] **Step 1: Root index redirects**

`web/src/app/page.tsx`:
```tsx
import { redirect } from "next/navigation";
export default function RootIndex() { redirect("/dashboard"); }
```

- [ ] **Step 2: `(auth)/layout.tsx`**

```tsx
export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return (
    <main className="min-h-screen grid place-items-center bg-ink-50 grid-bg">
      {children}
    </main>
  );
}
```

- [ ] **Step 3: `(auth)/login/page.tsx` placeholder**

```tsx
export default function LoginPlaceholder() {
  return (
    <div className="w-[400px] rounded-xl border border-ink-200 bg-white shadow-card p-8">
      <h1 className="text-lg font-semibold">Login</h1>
      <p className="text-[13px] text-ink-500 mt-1">Implemented in Phase B.</p>
    </div>
  );
}
```

- [ ] **Step 4: `(app)/layout.tsx` with auth gate**

```tsx
"use client";
import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Sidebar } from "@/components/shell/Sidebar";
import { useAuth } from "@/lib/auth/use-auth";

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, ready } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (ready && !isAuthenticated) router.replace("/login");
  }, [ready, isAuthenticated, router]);

  if (!ready) return null;
  if (!isAuthenticated) return null;

  return (
    <div className="h-screen flex bg-ink-50">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        {children}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Dashboard placeholder**

`web/src/app/(app)/dashboard/page.tsx`:
```tsx
import { Topbar } from "@/components/shell/Topbar";

export default function DashboardPlaceholder() {
  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "Dashboard"]} />
      <main className="flex-1 p-6 grid-bg">
        <div className="text-[13px] text-ink-500">
          Phase A scaffolding complete. Real screens land in Phase B.
        </div>
      </main>
    </>
  );
}
```

- [ ] **Step 6: Build + manual smoke**

```bash
pnpm build
pnpm dev
```

Visit `http://localhost:3000` (no token in `localStorage`) — should redirect: `/` → `/dashboard` → app layout sees no token → `/login`. The login placeholder should render.

Set a fake token in DevTools console: `localStorage.setItem("solhub.bearer", "x")`. Reload — dashboard placeholder should render.

- [ ] **Step 7: Commit**

```bash
git add web/
git commit -m "feat(web): route groups, auth gate, placeholder screens"
```

---

## Task 17: `.env.example` + README

**Files:**
- Create: `web/.env.example`
- Create: `web/README.md`

- [ ] **Step 1: `.env.example`**

```
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com

# Filled in once execution-vault is deployed (Phase D)
NEXT_PUBLIC_EXECUTION_VAULT_PROGRAM_ID=
NEXT_PUBLIC_WORKFLOW_REGISTRY_PROGRAM_ID=
```

- [ ] **Step 2: `web/README.md`**

```markdown
# Solhub Web

Frontend for the SolHub (SolanaKeeper) backend. Next.js 14 + TS + Tailwind + TanStack Query.

## Dev

    cp .env.example .env.local
    pnpm install
    pnpm dev

## Scripts

- `pnpm dev` — dev server on :3000
- `pnpm build` — production build
- `pnpm test` — Vitest
- `pnpm typecheck` — tsc --noEmit
- `pnpm lint` — next lint

## Phases

See `docs/superpowers/plans/2026-05-11-solhub-phase-*.md`.
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "docs(web): .env.example + README"
```

---

## Self-review checklist

- [ ] Spec §3 (Stack): all libraries pinned and used? Yes — Next 14, TS strict, Tailwind 3.4, TanStack Query 5, Zod 3.
- [ ] Spec §4 (Repo layout): all foundation files created? Yes — providers, sidebar, topbar, breadcrumb, route groups, primitives, lib/api/, lib/auth/, lib/utils/.
- [ ] Spec §5 (Auth flow): localStorage `solhub.bearer`, 401 → clear+redirect, `(app)` auth gate — all wired.
- [ ] No `git add .` anywhere. ✓
- [ ] Zod schemas mirror IDEA.md §4 + §8 + §11 — verified against the source.
- [ ] No placeholders, TODOs, or "implement later" — every step has runnable code.

---

## End-of-phase acceptance

```bash
cd web
pnpm typecheck    # passes
pnpm test         # all pass
pnpm build        # builds clean
pnpm dev          # boots; /login → redirects; with token → dashboard placeholder
```

Phase A is **done** when all four pass and there's a clean commit graph under `web/`.
