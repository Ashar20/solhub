# SolHub MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`. Orchestrator is Opus; each task is dispatched to a fresh Sonnet subagent.

**Goal:** Deliver a working SolHub MVP — Solana automation/execution platform — with all components scaffolded, the REST API end-to-end tested, all three Anchor programs compiling, the engine + plugins + CLI + MCP server functional, and an integration test suite covering the critical paths.

**Architecture:** Rust Cargo workspace with crates for `programs/*` (Anchor), `engine`, `api`, `cli`. TypeScript `mcp-server` lives outside the workspace. SQLite (via sqlx) is used for the dev/test DB to avoid a Postgres install; production migrations are written PG-flavoured but kept SQLite-compatible. External services (Jito, Turnkey, Yellowstone, ShredStream) are abstracted behind traits with a `mock` impl wired up for integration tests; a `live` impl exists but is feature-gated behind `--features live-net`.

**Tech Stack:** Rust 1.95 · Anchor 0.30 · Axum 0.7 · sqlx (sqlite+postgres) · tokio · tonic · `@modelcontextprotocol/sdk` · clap

---

## Execution Waves

```
Wave 1 (sequential):  Bootstrap — workspace + tooling
Wave 2 (parallel x6): Anchor programs · DB layer · Engine core types · Plugin trait + plugins · MCP server · CLI scaffold
Wave 3 (parallel x3): REST API · Executor + triggers · CLI commands
Wave 4 (sequential):  E2E integration tests
Wave 5 (sequential):  Commit + summary
```

---

## Wave 1 — Bootstrap

### Task 1.1: Cargo workspace + repo skeleton

**Files to create (this task creates all directories and skeleton files referenced later):**
- `Cargo.toml` (workspace manifest)
- `Anchor.toml`
- `.env.example`
- `.gitignore`
- `README.md` (replace existing)
- `rust-toolchain.toml` (pin 1.95)
- `programs/workflow-registry/{Cargo.toml,Xargo.toml,src/lib.rs}`
- `programs/execution-vault/{Cargo.toml,Xargo.toml,src/lib.rs}`
- `programs/condition-oracle/{Cargo.toml,Xargo.toml,src/lib.rs}`
- `engine/Cargo.toml`, `engine/src/main.rs`, `engine/src/lib.rs`
- `api/Cargo.toml`, `api/src/main.rs`, `api/src/lib.rs`
- `cli/Cargo.toml`, `cli/src/main.rs`
- `mcp-server/{package.json,tsconfig.json,src/index.ts}`
- `migrations/.gitkeep`
- `tests/.gitkeep`

**Acceptance:** `cargo build --workspace` succeeds with empty `lib.rs` bodies. `npm install` succeeds in `mcp-server/`. `.gitignore` excludes `.env`, `target/`, `node_modules/`, `test-ledger/`, `.anchor/`.

---

## Wave 2 — Parallel Component Build

### Task 2.1: Anchor programs (workflow-registry, execution-vault, condition-oracle)

Implement all three programs per IDEA.md §3 verbatim. Add anchor-spl as a dep where needed. Add an in-tree TypeScript test suite under `tests/anchor/` covering happy + error paths for every instruction. Use `anchor_client` for any required Rust-side helpers.

**Acceptance:** `anchor build` (when `anchor` is installed) produces three `.so` artifacts. `anchor test` runs all TS tests against the local validator. If anchor is unavailable, fall back to `cargo build --manifest-path programs/<each>/Cargo.toml` with bpf target verification skipped.

### Task 2.2: Database layer (sqlx, dual SQLite + Postgres)

- `migrations/001_workflows.sql`, `002_runs.sql`, `003_api_keys.sql`, `004_marketplace.sql` — per IDEA.md §11 but with SQLite-compatible SQL (use `TEXT` for UUIDs, `INTEGER` for `BIGINT`, no `uuid_generate_v4()` extension — generate UUIDs in Rust).
- New crate `db/` (add to workspace) exposing `Db` struct with `connect(url) -> Result<Db>`, query helpers, and typed row structs for `Workflow`, `WorkflowRun`, `ApiKey`, `Organization`.
- Methods: `create_workflow`, `get_workflow`, `list_workflows`, `update_workflow`, `delete_workflow`, `record_run_start`, `update_run_status`, `append_step_log`, `get_org_by_api_key_hash`, `create_org`, `create_api_key`, `revoke_api_key`.
- Unit tests in `db/src/lib.rs` using `sqlx::test` macro against in-memory SQLite covering every method.

**Acceptance:** `cargo test -p db` all green.

### Task 2.3: Engine core types + state machine

Implement `engine/src/state/{mod.rs,workflow.rs,run.rs}` per IDEA.md §4.2 and §4.3 verbatim. Add `RunStatus` transition validation in `state/run.rs::WorkflowRun::transition_to(new: RunStatus)` that rejects illegal transitions (e.g., `Confirmed -> Pending`). Add `engine/src/config.rs` loading env vars via `envy` or manual parsing.

**Acceptance:** `cargo test -p engine -- state::` covers every valid + every invalid transition.

### Task 2.4: Plugin trait + reference plugins

- `engine/src/plugins/mod.rs` — `SolanaKeeperPlugin` trait per IDEA.md §7.1, plus `PluginRegistry` (HashMap<String, Box<dyn SolanaKeeperPlugin>>), plus `PluginError` enum.
- `engine/src/plugins/jupiter.rs` — full implementation per §7.2. Use `reqwest` against the Jupiter v6 HTTP API directly (avoid the `jupiter-swap-api-client` crate dependency hell — IDEA.md mentions it but reqwest is more reliable).
- `engine/src/plugins/pyth.rs` — read-only `read_price` action using `pyth-sdk-solana`.
- `engine/src/plugins/notifications/telegram.rs` — `send_message` via Telegram Bot API HTTP.
- `engine/src/plugins/notifications/discord.rs` — `send_message` via Discord webhook URL.
- Stubs for `kamino.rs`, `marinade.rs`, `drift.rs`, `orca.rs`, `raydium.rs` that return `PluginError::NotImplemented` for `build_transactions`, but DO register `ActionDefinition`s with full schemas so the API can advertise them.
- Unit tests: each plugin's `actions()` returns the correct schema; Jupiter's `build_transactions` for `swap` produces a deserializable `VersionedTransaction` (mock the HTTP call with `mockito`).

**Acceptance:** `cargo test -p engine -- plugins::` all green. Jupiter test confirms a real Jupiter API call returns a tx (gated behind `--ignored` so it doesn't run by default).

### Task 2.5: MCP server (TypeScript)

Implement `mcp-server/src/` per IDEA.md §9: `index.ts` with `StdioServerTransport` and all 7 `sk.*` tools. Each tool handler makes an HTTP call to the local REST API (configured via `SOLHUB_API_URL` env var, defaulting to `http://localhost:8080`). Add `tools/*.ts` files for each tool's request/response logic. Use `zod` for input validation.

**Acceptance:** `npm run build` succeeds. `node dist/index.js` starts the server and responds to `tools/list` with all 7 tools.

### Task 2.6: CLI scaffold (clap)

`cli/src/main.rs` + `cli/src/commands/{workflow.rs,run.rs,execute.rs,auth.rs,billing.rs,config.rs}`. Use `clap` derive macros. Each subcommand stubbed with the correct argument structure per IDEA.md §12.1; implementations call the REST API via `reqwest`. Config stored in `~/.config/skh/config.toml` (api_key, api_url, rpc_url).

**Acceptance:** `cargo build -p cli`. `./target/debug/skh --help` shows the full command tree.

---

## Wave 3 — Integration Build

### Task 3.1: REST API (Axum)

Implement `api/src/` per IDEA.md §8:
- `main.rs` — router setup, all routes, app state with db pool + plugin registry.
- `routes/workflows.rs` — CRUD + trigger handlers wired to db crate.
- `routes/runs.rs` — list, get, SSE log stream (use `axum::response::sse`).
- `routes/webhooks.rs` — HMAC validation per non-negotiable #9; trigger workflow run.
- `routes/analytics.rs` — aggregate queries.
- `routes/orgs.rs` — org info + api key CRUD.
- `routes/hub.rs` — public marketplace listing + call.
- `middleware/auth.rs` — Bearer token → org lookup (skips `/v1/webhooks/*`).
- `middleware/rate_limit.rs` — token bucket per API key using `tower_governor` or hand-rolled.
- `types.rs` — request/response structs per §8.2.

**Tests** (`api/tests/integration.rs`):
- Uses `sqlx::test` to spin an isolated in-memory DB per test.
- Spawns the Axum app on an ephemeral port.
- Covers: create workflow (200 + persisted), list workflows, get workflow, update workflow, delete workflow, trigger workflow (creates a run), get run, list runs, create api key, revoke api key, webhook with valid HMAC (200), webhook with invalid HMAC (401), unauthorized request without bearer (401), rate limit exceeded (429).

**Acceptance:** `cargo test -p api` all green. 100% of routes from §8.4 have at least one happy-path + one error-path test.

### Task 3.2: Executor + triggers + bundle builder

- `engine/src/trigger/{mod.rs,cron.rs,account_watch.rs,webhook.rs}` — cron uses `tokio-cron-scheduler`; account_watch + webhook are receivers driven by external events (Geyser router & API webhook handler post events into them via a channel).
- `engine/src/executor/{mod.rs,bundle_builder.rs,tip_calculator.rs,retry.rs,simulator.rs}` per IDEA.md §6. `BundleBuilder` is a trait with a `JitoBundleBuilder` impl (real Jito searcher client) and a `MockBundleBuilder` impl that simulates locally and returns a fake bundle_id. Tests use the mock.
- `engine/src/geyser/{mod.rs,yellowstone.rs,shredstream.rs,router.rs}` — implement per §5 but ALL behind `feature = "live-net"`. Behind default features, expose only the `DualFeedRouter` with a `MockFeed` that emits canned events for tests.
- `engine/src/wallet/{mod.rs,turnkey.rs,agentic.rs}` — `Signer` trait. `TurnkeyWallet` real impl behind `feature = "live-net"`. `LocalKeypairSigner` impl (reads from file path) used by tests + dev. `AgenticWallet` per §10.2.
- Wire it all in `engine/src/main.rs` — long-running tokio service that reads workflows from DB, schedules them, executes runs end-to-end. On startup it reads `DATABASE_URL` and `SOLANA_RPC_URL` from env.

**Tests:**
- `engine/tests/executor.rs` — drive a workflow end-to-end: cron trigger fires → Jupiter plugin builds a (mocked-Jupiter) tx → mock bundle builder accepts it → run log written to DB → status transitions to `Confirmed`.
- `engine/tests/retry.rs` — retry policy retries 3x with exponential backoff (use `tokio::time::pause()` to fast-forward).
- `engine/tests/router.rs` — dual-feed dedup window correctly drops duplicates within 500ms.

**Acceptance:** `cargo test -p engine` all green.

### Task 3.3: CLI command implementations + auth flow

Fill in `cli/src/commands/*.rs` to call the REST API. Auth: `skh auth login` opens browser to a placeholder URL and prompts for API key (interactive); `skh auth status` shows current config.

**Acceptance:** Manual smoke: `./target/debug/skh workflow list` against a running local API returns the workflow list. Add an integration test in `cli/tests/cli.rs` using `assert_cmd` that spawns the API + the CLI and verifies output.

---

## Wave 4 — End-to-End Verification

### Task 4.1: Full-stack E2E test suite

`tests/e2e/main.rs` (new top-level test binary):

1. Spawn API server in a tokio task on an ephemeral port with an in-memory SQLite.
2. Spawn engine in another task pointed at the same DB.
3. POST to `/v1/workflows` to create a cron workflow with a Jupiter swap step using mocked Jupiter.
4. POST to `/v1/workflows/:id/trigger` to manually fire it.
5. Poll `/v1/runs?workflow_id=...` until status is `Confirmed` (timeout 30s).
6. GET `/v1/runs/:run_id` and assert the step log contains the Jupiter output.
7. Verify webhook flow: POST signed webhook → run created.
8. Verify SSE: connect to `/v1/runs/:run_id/logs`, receive at least `run_complete` event.
9. Verify analytics endpoint returns non-zero execution count.
10. Verify MCP server flow: spawn the MCP server pointed at the API, send a `tools/call` for `sk.list_workflows`, receive the workflow back.

**Acceptance:** `cargo test --test e2e` all green. 10/10 scenarios pass.

### Task 4.2: Verification + commit

- Run the full test matrix: `cargo build --workspace`, `cargo test --workspace`, `cargo test --test e2e`, `cd mcp-server && npm run build && npm test`.
- Verify no warnings, no `unwrap()` in non-test code, no `println!` debug leftovers.
- Stage everything with `git add -A` (the `.gitignore` excludes secrets).
- Commit as Ashar20 with message `feat: scaffold solhub mvp with anchor programs, engine, api, mcp server, cli`.
- Do NOT push — user can review and push.

---

## Self-Review

- [x] Spec coverage — all 16 IDEA.md sections mapped to tasks except §16 Phase 4 (enterprise) which is explicitly out of scope.
- [x] No placeholders — every task has acceptance criteria and exact file paths.
- [x] Type consistency — `Workflow`, `WorkflowRun`, `WorkflowStep`, `RunStatus`, `TriggerConfig`, `SolanaKeeperPlugin`, `ActionDefinition` are defined in Task 2.3/2.4 and used unchanged downstream.
- [x] Tooling fallback — Anchor/Solana CLI installs are optional; tests degrade gracefully without them.
- [x] DB portability — SQLite for dev/test, Postgres-compatible SQL.

---

## Out of Scope (deferred to follow-up sessions)

- Phase 4 enterprise items: multi-region HA, SSO/SAML, Squads multisig upgrade authority, compliance audit exports.
- Full implementations of Kamino/Marinade/Drift/Orca/Raydium/Metaplex plugins (scaffolded with stubs; can be filled in plugin-by-plugin).
- React Flow visual canvas / AI Prompt Builder (frontend — separate spec).
- 99.99% SLA infrastructure work.
- Live mainnet deploy of Anchor programs (devnet deploy script included; mainnet requires multisig + audit).
