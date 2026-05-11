# Solhub Frontend — Phase E: Remaining Screens Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish v1. Add Marketplace detail (+ hub call), Settings (org + API key management), AI Builder, Versions. After Phase E the app meets the spec's "definition of done".

**Architecture:** Same patterns as A–D. Settings uses the existing `orgs` API module + the "raw key shown once" pattern. AI Builder posts to `POST /v1/ai/build` (Backend Gap #2). Versions stubs gracefully when the backend endpoint isn't ready.

**Tech Stack:** No new deps.

**Pre-requisite:** Phases A + B + C complete. (Phase D is independent; either order works for E.)

**Reference:** spec §6 (remaining rows), §9 (Backend gaps #2, #3, #4).

**Commit policy:** `git add web/` only.

---

## Task 1: Marketplace detail page

**Files:**
- Create: `web/src/app/(app)/marketplace/[id]/page.tsx`
- Create: `web/src/lib/hooks/use-hub-call.ts`

- [ ] **Step 1: `use-hub-call.ts`**

```ts
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { hub } from "@/lib/api";

export function useHubCall() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; params?: Record<string, unknown> }) =>
      hub.callHubWorkflow(vars.id, vars.params),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["runs"] }),
  });
}
```

- [ ] **Step 2: Page**

```tsx
"use client";
import { use } from "react";
import { useRouter } from "next/navigation";
import { Topbar } from "@/components/shell/Topbar";
import { useHubWorkflow } from "@/lib/hooks/use-hub";
import { useHubCall } from "@/lib/hooks/use-hub-call";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { ProtocolBadge } from "@/components/marketplace/ProtocolBadge";
import { Icon } from "@/components/primitives/Icon";

export default function MarketplaceDetail({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const { data, isLoading, error } = useHubWorkflow(id);
  const call = useHubCall();
  const router = useRouter();

  async function onUse() {
    const r = await call.mutateAsync({ id });
    router.push(`/runs/${r.run_id}`);
  }

  return (
    <>
      <Topbar crumbs={["Hub", "Marketplace", data?.name ?? id]} />
      <main className="flex-1 p-6 overflow-y-auto">
        {isLoading && <div className="text-[12px] text-ink-500">Loading…</div>}
        {error && (
          <div className="rounded-lg border border-rose-200 bg-rose-50 p-4 text-[12px] text-rose-700">
            Couldn't load this workflow. The detail endpoint may not be available yet.
          </div>
        )}
        {data && (
          <div className="grid grid-cols-[1fr_320px] gap-6">
            <div>
              <div className="flex items-center gap-2 mb-2">
                <h1 className="text-[22px] font-semibold tracking-tight">{data.name}</h1>
                {data.verified && <Pill tone="emerald"><Icon name="check" className="w-3 h-3" />verified</Pill>}
                {data.audited && <Pill tone="cyan"><Icon name="shield" className="w-3 h-3" />audited</Pill>}
              </div>
              <div className="text-[12px] font-mono text-ink-500 mb-3">{data.author}</div>
              <p className="text-[13px] text-ink-700 leading-relaxed mb-4">{data.description}</p>
              <div className="flex flex-wrap gap-1">
                {data.protocols.map((p) => <ProtocolBadge key={p} name={p} />)}
              </div>
            </div>
            <aside className="rounded-xl border border-ink-200 bg-white shadow-card p-4 space-y-3 h-fit">
              <div className="grid grid-cols-2 gap-3 text-[12px] font-mono">
                <div><div className="text-ink-400">Runs</div><div className="text-ink-900">{data.runs.toLocaleString()}</div></div>
                <div><div className="text-ink-400">Success</div><div className="text-ink-900">{data.success_rate}</div></div>
                <div><div className="text-ink-400">Fee</div><div className="text-ink-900">{data.fee_usdc}</div></div>
                {data.apy && <div><div className="text-ink-400">APY</div><div className="text-emerald-700">{data.apy}</div></div>}
              </div>
              <Btn variant="primary" size="lg" className="w-full justify-center" onClick={onUse} disabled={call.isPending}>
                {call.isPending ? "Submitting…" : "Use this workflow"}
              </Btn>
            </aside>
          </div>
        )}
      </main>
    </>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): marketplace detail + hub call action"
```

---

## Task 2: Settings — org info + API keys

**Files:**
- Create: `web/src/app/(app)/settings/page.tsx`
- Create: `web/src/components/settings/ApiKeyList.tsx`
- Create: `web/src/components/settings/CreateApiKeyDialog.tsx`
- Create: `web/src/lib/hooks/use-api-keys.ts`

- [ ] **Step 1: `use-api-keys.ts`**

```ts
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { orgs } from "@/lib/api";

export const useApiKeys = () => useQuery({
  queryKey: ["org", "me", "api_keys"] as const,
  queryFn: orgs.listApiKeys,
});

export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => orgs.createApiKey(name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["org", "me", "api_keys"] }),
  });
}

export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => orgs.revokeApiKey(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["org", "me", "api_keys"] }),
  });
}
```

- [ ] **Step 2: `ApiKeyList.tsx`**

```tsx
"use client";
import { useApiKeys, useRevokeApiKey } from "@/lib/hooks/use-api-keys";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { Pill } from "@/components/primitives/Pill";
import { formatRelativeTime } from "@/lib/utils/format";

export function ApiKeyList() {
  const { data, isLoading } = useApiKeys();
  const revoke = useRevokeApiKey();

  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
      <div className="grid grid-cols-[1fr_140px_140px_80px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
        <div>Name</div><div>Last used</div><div>Created</div><div className="text-right">Action</div>
      </div>
      {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
      {!isLoading && (data ?? []).length === 0 && (
        <div className="p-6 text-[12px] text-ink-500">No API keys yet.</div>
      )}
      {(data ?? []).map((k) => (
        <div key={k.id} className="grid grid-cols-[1fr_140px_140px_80px] items-center px-4 h-11 border-b border-ink-100 text-[13px]">
          <div className="flex items-center gap-2">
            <div className="font-medium text-ink-900">{k.name ?? "(unnamed)"}</div>
            {k.revoked_at && <Pill tone="rose">revoked</Pill>}
          </div>
          <div className="text-[12px] text-ink-500 font-mono">
            {k.last_used_at ? formatRelativeTime(k.last_used_at) : "—"}
          </div>
          <div className="text-[12px] text-ink-500 font-mono">{formatRelativeTime(k.created_at)}</div>
          <div className="flex justify-end">
            {!k.revoked_at && (
              <Btn
                variant="danger"
                size="sm"
                onClick={() => { if (confirm(`Revoke key "${k.name ?? "(unnamed)"}"?`)) revoke.mutate(k.id); }}
                icon={<Icon name="trash" className="w-3.5 h-3.5" />}
              >
                Revoke
              </Btn>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: `CreateApiKeyDialog.tsx`**

The raw key is shown **once** here, per IDEA.md Non-Negotiable Rule #8.

```tsx
"use client";
import { useState } from "react";
import { useCreateApiKey } from "@/lib/hooks/use-api-keys";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";

export function CreateApiKeyDialog({ onClose }: { onClose: () => void }) {
  const [name, setName] = useState("");
  const [created, setCreated] = useState<{ raw_key: string } | null>(null);
  const [copied, setCopied] = useState(false);
  const create = useCreateApiKey();

  async function submit() {
    const r = await create.mutateAsync(name.trim() || "untitled");
    setCreated({ raw_key: r.raw_key });
  }

  async function copy() {
    if (!created) return;
    await navigator.clipboard.writeText(created.raw_key);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[480px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        {!created ? (
          <>
            <h2 className="text-[16px] font-semibold tracking-tight">New API key</h2>
            <label className="block">
              <span className="text-[12px] font-medium">Name</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. server-backend"
                className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px]"
              />
            </label>
            <div className="flex justify-end gap-2">
              <Btn variant="ghost" onClick={onClose} disabled={create.isPending}>Cancel</Btn>
              <Btn variant="primary" onClick={submit} disabled={create.isPending}>
                {create.isPending ? "Creating…" : "Create"}
              </Btn>
            </div>
          </>
        ) : (
          <>
            <h2 className="text-[16px] font-semibold tracking-tight">Save this key now</h2>
            <p className="text-[12px] text-ink-500">It will not be shown again.</p>
            <div className="flex items-center gap-2 rounded-md border border-ink-200 bg-ink-50 px-3 py-2">
              <code className="flex-1 text-[12px] font-mono break-all">{created.raw_key}</code>
              <button onClick={copy} className="text-ink-700 hover:text-ink-900">
                <Icon name={copied ? "check" : "copy"} className="w-4 h-4" />
              </button>
            </div>
            <div className="flex justify-end">
              <Btn variant="primary" onClick={onClose}>Done</Btn>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Settings page**

```tsx
"use client";
import { useState } from "react";
import { Topbar } from "@/components/shell/Topbar";
import { useMe } from "@/lib/hooks/use-org";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { ApiKeyList } from "@/components/settings/ApiKeyList";
import { CreateApiKeyDialog } from "@/components/settings/CreateApiKeyDialog";
import { formatAddress, formatUsdc } from "@/lib/utils/format";

export default function SettingsPage() {
  const me = useMe();
  const [open, setOpen] = useState(false);
  return (
    <>
      <Topbar crumbs={["Account", "Settings"]} />
      <main className="flex-1 p-6 overflow-y-auto space-y-6">
        <section>
          <h2 className="text-[14px] font-semibold tracking-tight mb-2">Organisation</h2>
          <div className="rounded-xl border border-ink-200 bg-white shadow-card p-4 grid grid-cols-3 gap-3 text-[13px]">
            <div>
              <div className="text-[11px] uppercase font-mono text-ink-500">Name</div>
              <div className="mt-0.5">{me.data?.name ?? "—"}</div>
            </div>
            <div>
              <div className="text-[11px] uppercase font-mono text-ink-500">Signing wallet</div>
              <div className="mt-0.5 font-mono">{me.data?.wallet_address ? formatAddress(me.data.wallet_address) : "—"}</div>
            </div>
            <div>
              <div className="text-[11px] uppercase font-mono text-ink-500">Credits</div>
              <div className="mt-0.5 font-mono">{me.data ? formatUsdc(me.data.credits_usdc) : "—"}</div>
            </div>
          </div>
        </section>
        <section>
          <div className="flex items-center justify-between mb-2">
            <h2 className="text-[14px] font-semibold tracking-tight">API keys</h2>
            <Btn variant="primary" size="sm" icon={<Icon name="plus" className="w-3.5 h-3.5" />} onClick={() => setOpen(true)}>
              New key
            </Btn>
          </div>
          <ApiKeyList />
        </section>
        {open && <CreateApiKeyDialog onClose={() => setOpen(false)} />}
      </main>
    </>
  );
}
```

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): settings — org info + API key management"
```

---

## Task 3: AI Builder

**Files:**
- Create: `web/src/lib/api/ai.ts`
- Create: `web/src/app/(app)/ai/page.tsx`

`POST /v1/ai/build` is Backend Gap #2. We ship the UI now; it shows a graceful error if the backend hasn't built the endpoint.

- [ ] **Step 1: `ai.ts`**

```ts
import { z } from "zod";
import { apiRequest } from "./client";
import { TriggerConfigSchema, WorkflowStepSchema } from "./schemas";

const AiBuildResponse = z.object({
  draft: z.object({
    name: z.string(),
    trigger: TriggerConfigSchema,
    steps: z.array(WorkflowStepSchema),
  }),
  rationale: z.string().nullable().optional(),
});

export const buildFromPrompt = (prompt: string) =>
  apiRequest("/v1/ai/build", AiBuildResponse, { method: "POST", body: { prompt } });

export type AiBuildResult = z.infer<typeof AiBuildResponse>;
```

- [ ] **Step 2: Page**

```tsx
"use client";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { Topbar } from "@/components/shell/Topbar";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { buildFromPrompt, type AiBuildResult } from "@/lib/api/ai";
import { workflows } from "@/lib/api";

const EXAMPLES = [
  "Every Sunday at 09:00 UTC, claim Kamino rewards and swap them to mSOL via Jupiter.",
  "When the SOL price drops below $140 on Pyth, buy 50 USDC of SOL via Jupiter and alert Telegram.",
  "Every hour, rebalance my JLP position if exposure drifts more than 5% from target.",
];

export default function AiBuilderPage() {
  const [prompt, setPrompt] = useState("");
  const [result, setResult] = useState<AiBuildResult | null>(null);
  const router = useRouter();

  const build = useMutation({
    mutationFn: (p: string) => buildFromPrompt(p),
    onSuccess: (r) => setResult(r),
  });

  async function openInBuilder() {
    if (!result) return;
    const r = await workflows.createWorkflow(result.draft);
    router.push(`/workflows/${r.workflow_id}`);
  }

  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "AI Builder"]} />
      <main className="flex-1 p-6 overflow-y-auto grid grid-cols-[1fr_360px] gap-6">
        <section>
          <h1 className="text-[22px] font-semibold tracking-tight mb-1">Describe a workflow</h1>
          <p className="text-[13px] text-ink-500 mb-4">
            Plain English. The model returns a draft workflow you can refine in the builder.
          </p>
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Every 6 hours, check Kamino LTV. If above 0.7, withdraw 10% collateral and swap to USDC."
            rows={6}
            className="w-full rounded-lg border border-ink-200 p-3 text-[13px] focus:outline-none focus:ring-2 focus:ring-violet-500/30"
          />
          <div className="mt-2 flex items-center gap-2">
            <Btn variant="primary" onClick={() => build.mutate(prompt)} disabled={build.isPending || prompt.trim() === ""}>
              {build.isPending ? "Thinking…" : "Generate workflow"}
            </Btn>
            {build.isError && (
              <span className="text-[12px] text-rose-600">
                AI build endpoint not available yet — see Backend Gap #2.
              </span>
            )}
          </div>

          {result && (
            <div className="mt-6 rounded-xl border border-ink-200 bg-white shadow-card p-4">
              <div className="flex items-center justify-between mb-2">
                <div>
                  <div className="text-[15px] font-semibold tracking-tight">{result.draft.name}</div>
                  <div className="text-[12px] font-mono text-ink-500">trigger: {result.draft.trigger.type}</div>
                </div>
                <Btn variant="primary" onClick={openInBuilder}>Open in builder</Btn>
              </div>
              <ol className="space-y-1.5">
                {result.draft.steps.map((s) => (
                  <li key={s.id} className="flex items-center gap-2 text-[12px]">
                    <Pill tone="violet">{s.plugin}</Pill>
                    <Pill tone="ink">{s.action}</Pill>
                    <span className="font-mono text-ink-500 truncate">{JSON.stringify(s.params)}</span>
                  </li>
                ))}
              </ol>
              {result.rationale && (
                <p className="mt-3 text-[12px] text-ink-500 leading-relaxed">{result.rationale}</p>
              )}
            </div>
          )}
        </section>
        <aside>
          <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500 mb-2">Examples</div>
          <ul className="space-y-1.5">
            {EXAMPLES.map((ex) => (
              <li key={ex}>
                <button
                  onClick={() => setPrompt(ex)}
                  className="w-full text-left text-[12px] p-3 rounded-lg border border-ink-200 hover:bg-ink-50"
                >
                  {ex}
                </button>
              </li>
            ))}
          </ul>
        </aside>
      </main>
    </>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): AI Builder screen (depends on Backend Gap #2)"
```

---

## Task 4: Versions screen

**Files:**
- Create: `web/src/lib/api/versions.ts`
- Create: `web/src/app/(app)/versions/page.tsx`

This depends on Backend Gap #4 (no endpoint exists yet). Ship a graceful empty-state UI now.

- [ ] **Step 1: `versions.ts`**

```ts
import { z } from "zod";
import { apiRequest } from "./client";

const WorkflowVersion = z.object({
  version: z.string(),
  workflow_id: z.string().uuid(),
  created_at: z.string(),
  author: z.string().nullable().optional(),
  summary: z.string().nullable().optional(),
});

export const listVersions = (workflow_id: string) =>
  apiRequest(`/v1/workflows/${workflow_id}/versions`, z.array(WorkflowVersion));

export const rollbackVersion = (workflow_id: string, version: string) =>
  apiRequest(`/v1/workflows/${workflow_id}/rollback`, z.object({ ok: z.boolean() }), {
    method: "POST",
    body: { version },
  });

export type WorkflowVersion = z.infer<typeof WorkflowVersion>;
```

- [ ] **Step 2: Page**

```tsx
"use client";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Topbar } from "@/components/shell/Topbar";
import { useWorkflows } from "@/lib/hooks/use-workflows";
import { listVersions } from "@/lib/api/versions";
import { Pill } from "@/components/primitives/Pill";
import { formatRelativeTime } from "@/lib/utils/format";

export default function VersionsPage() {
  const wfs = useWorkflows();
  const [selected, setSelected] = useState<string | null>(null);

  const versions = useQuery({
    queryKey: ["versions", selected] as const,
    queryFn: () => listVersions(selected!),
    enabled: !!selected,
    retry: false, // backend gap — don't hammer
  });

  return (
    <>
      <Topbar crumbs={["Operate", "Versions"]} />
      <main className="flex-1 grid grid-cols-[320px_1fr] gap-4 p-6 overflow-hidden">
        <aside className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          <div className="h-9 px-3 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500 flex items-center">Workflows</div>
          <ul className="overflow-y-auto scrollbar-thin max-h-[calc(100vh-160px)]">
            {(wfs.data ?? []).map((w) => (
              <li key={w.id}>
                <button
                  onClick={() => setSelected(w.id)}
                  className={
                    "w-full text-left px-3 py-2 text-[13px] border-b border-ink-100 hover:bg-ink-50 " +
                    (selected === w.id ? "bg-ink-100 font-medium" : "")
                  }
                >
                  {w.name}
                </button>
              </li>
            ))}
          </ul>
        </aside>
        <section className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
          {!selected && (
            <div className="p-6 text-[12px] text-ink-500">Select a workflow to view its versions.</div>
          )}
          {selected && versions.isLoading && (
            <div className="p-6 text-[12px] text-ink-500">Loading…</div>
          )}
          {selected && versions.error && (
            <div className="p-6 text-[12px] text-ink-500">
              Versions endpoint not available yet (Backend Gap #4).
            </div>
          )}
          {selected && versions.data && versions.data.length === 0 && (
            <div className="p-6 text-[12px] text-ink-500">No version history.</div>
          )}
          {selected && versions.data && versions.data.length > 0 && (
            <ul>
              {versions.data.map((v) => (
                <li key={v.version} className="grid grid-cols-[100px_1fr_140px] items-center px-4 h-11 border-b border-ink-100 text-[13px]">
                  <Pill tone="ink">{v.version}</Pill>
                  <div className="truncate">{v.summary ?? "—"}</div>
                  <div className="text-right text-[11px] text-ink-500 font-mono">
                    {formatRelativeTime(v.created_at)}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </main>
    </>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(web): versions screen with backend-gap fallback"
```

---

## Task 5: Final acceptance smoke pass

- [ ] **Step 1: Boot dev with backend**

```bash
cd web && pnpm dev
```

- [ ] **Step 2: Manual checklist — all v1 screens**

| Route | Expected |
|---|---|
| `/login` | API-key paste flow works |
| `/dashboard` | KPIs + recent activity render |
| `/workflows` | List with filters, search, row actions |
| `/workflows/new` | Empty builder, palette, inspector empty state |
| `/workflows/[id]` | Hydrates server state; auto-save active |
| `/runs` | List with status pills, polling |
| `/runs/[run_id]` | Step timeline + live log (SSE or polling) |
| `/marketplace` | Card grid |
| `/marketplace/[id]` | Detail + Use button |
| `/wallet` | Personal + org cards; deposit dialog (if IDL ready) |
| `/versions` | Workflow list + version pane; graceful fallback |
| `/settings` | Org info + API key list + create key (raw shown once) |
| `/ai` | Prompt → draft → "open in builder" |

- [ ] **Step 3: All checks**

```bash
pnpm typecheck && pnpm test && pnpm build
```

- [ ] **Step 4: Final commit if needed**

```bash
git add web/
git commit -m "fix(web): phase E smoke pass adjustments"  # only if needed
```

---

## Self-review checklist

- [ ] Spec §6 — Marketplace detail, Settings, AI Builder, Versions all wired. ✓
- [ ] Spec §9 — Backend Gaps #2, #3, #4 all referenced and gracefully handled. ✓
- [ ] IDEA.md Non-Negotiable Rule #8 (raw API key shown once) honored in `CreateApiKeyDialog`. ✓
- [ ] No `git add .` anywhere. ✓
- [ ] Versions screen does NOT retry on error (avoids hammering an unbuilt endpoint). ✓

---

## End-of-phase acceptance / v1 DONE

v1 is **done** when:
- All ten + detail routes render with real data when endpoints exist, graceful empty/loading/error states when they don't.
- Phantom/Solflare connect, and a USDC deposit lands in the `execution-vault` PDA on devnet (subject to Phase D IDL).
- `pnpm typecheck && pnpm test && pnpm build` pass clean.
- Manual smoke checklist above passes.

Match the spec's §14 "Definition of done".
