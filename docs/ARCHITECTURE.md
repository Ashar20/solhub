# SolHub Backend — Architecture & Operator Guide

> Solana-native automation platform. Workflows define **triggers → conditions → actions**.
> The platform reliably executes them on-chain via Jito bundles (or direct RPC submission
> in MVP), with a plugin-based node system, sub-workflows, human-in-the-loop approval
> gates, credit accounting, and x402-gated public hub.

This document is the single-source operator guide. Read top-to-bottom on first contact;
then return to specific sections as a reference.

---

## Table of Contents

1. [Big picture](#1-big-picture)
2. [Repository layout](#2-repository-layout)
3. [Toolchain & prerequisites](#3-toolchain--prerequisites)
4. [Configuration (env vars)](#4-configuration-env-vars)
5. [Build & start](#5-build--start)
6. [Component deep-dives](#6-component-deep-dives)
   - 6.1 [Anchor programs](#61-anchor-programs)
   - 6.2 [Database (`db` crate)](#62-database-db-crate)
   - 6.3 [Engine (`engine` crate)](#63-engine-engine-crate)
   - 6.4 [REST API (`api` crate)](#64-rest-api-api-crate)
   - 6.5 [CLI (`cli` crate)](#65-cli-cli-crate)
   - 6.6 [MCP server (`mcp-server/`)](#66-mcp-server-mcp-server)
7. [Plugin & node catalog](#7-plugin--node-catalog)
8. [State machine](#8-state-machine)
9. [Workflow lifecycle (sequence)](#9-workflow-lifecycle-sequence)
10. [Approval gate](#10-approval-gate)
11. [Sub-workflows](#11-sub-workflows)
12. [Credit system](#12-credit-system)
13. [x402 payment-gated hub](#13-x402-payment-gated-hub)
14. [Webhook trigger (HMAC)](#14-webhook-trigger-hmac)
15. [REST API reference](#15-rest-api-reference)
16. [MCP tool reference](#16-mcp-tool-reference)
17. [E2E test scripts](#17-e2e-test-scripts)
18. [Devnet deployment](#18-devnet-deployment)
19. [Security & operational notes](#19-security--operational-notes)
20. [Known limitations & follow-ups](#20-known-limitations--follow-ups)

---

## 1. Big picture

```
                           ┌──────────────────────┐
                           │   solhub-api (Axum)  │ ──► clients (curl / SDK / MCP / web)
                           │   REST + SSE         │
                           └──────────┬───────────┘
                                      │ writes runs / reads workflows
                                      ▼
                           ┌──────────────────────┐
                           │   SQLite (sqlx)      │
                           │   workflows · runs · │
                           │   payments · credits │
                           └──────────┬───────────┘
                                      ▲ polls Pending / Resumed
                                      │
   cron schedule ──────────► ┌────────┴────────────┐
   webhook payload ─────────►│ solhub-engine        │
                             │ Scheduler + Executor│
                             │   Plugin Registry   │
                             └────────┬────────────┘
                                      │ build_transactions / read / notify
                                      ▼
              ┌───────────┬───────────┴────────────┬───────────┐
              │           │                        │           │
              ▼           ▼                        ▼           ▼
        Solana RPC   Jupiter API           OpenAI/Anthropic   Telegram/Discord
        (devnet)    (lite-api.jup.ag)      (LLM reasoning)    (notifications)
              │
              ▼
    ┌────────────────────────────────────────────────────┐
    │ Anchor programs on devnet:                         │
    │   workflow_registry  Eemnq9Fv55B2TNi5z…JuFB        │
    │   execution_vault    4CFgDzuLnfdTThgNX…oMnBHJn     │
    │   condition_oracle   JwYqHkFc9w3bwZuK8…Gey7h       │
    └────────────────────────────────────────────────────┘
```

**Separation of concerns:**
- The **API** is a thin REST layer over the DB. It creates run records, validates
  payments, gates with auth + credits, but never touches Solana directly.
- The **Engine** polls the DB for `Pending` / `Resumed` runs, executes them through
  the plugin registry, and submits transactions to Solana.
- The **DB** is the single source of truth for workflows, runs, credits, payments.
- **Anchor programs** provide on-chain trustlessness for workflow registration,
  execution billing, and standardised condition oracles. The MVP engine works fine
  without them — they're the trust anchor for production.

---

## 2. Repository layout

```
solhub/
├── programs/                 # Anchor programs (Rust, deployed to devnet)
│   ├── workflow-registry/
│   ├── execution-vault/
│   └── condition-oracle/
├── db/                       # sqlx data layer (SQLite + Postgres-compatible)
│   ├── src/
│   │   ├── lib.rs            # Db struct, connect_in_memory, migrate
│   │   ├── models.rs         # Organization, ApiKey, Workflow, WorkflowRun, LedgerEntry…
│   │   ├── orgs.rs           # org + api_key CRUD
│   │   ├── workflows.rs      # workflow CRUD + status helpers
│   │   ├── runs.rs           # run CRUD, list_runs_to_execute, set_resume_index
│   │   ├── payments.rs       # x402 payment records
│   │   ├── credits.rs        # debit_credit_for_run, grant_credits, ledger
│   │   └── error.rs          # DbError enum
│   └── Cargo.toml
├── engine/                   # Long-running worker
│   ├── src/
│   │   ├── main.rs           # bootstrap: load env, signer, plugins, scheduler
│   │   ├── lib.rs            # module declarations
│   │   ├── config.rs         # envy-backed Config
│   │   ├── state/
│   │   │   ├── workflow.rs   # Workflow, TriggerConfig, WorkflowStep, OnError
│   │   │   └── run.rs        # WorkflowRun, RunStatus, transition_to, TriggerSource
│   │   ├── plugins/          # see Section 7 for the full catalog
│   │   │   ├── mod.rs        # PluginRegistry, SolanaKeeperPlugin trait, ActionDef
│   │   │   ├── system.rs     # transfer, batch_transfer, memo, get_balance
│   │   │   ├── jupiter.rs    # swap, quote, price (lite-api.jup.ag)
│   │   │   ├── pyth.rs       # read_price, staleness_check
│   │   │   ├── portfolio.rs  # snapshot, detect_drift, current_weights_from_holdings
│   │   │   ├── fear_greed.rs # alternative.me Fear & Greed Index
│   │   │   ├── news.rs       # fetch_headlines (CoinDesk RSS), crypto_panic, fetch_url
│   │   │   ├── llm.rs        # complete, analyze_sentiment, recommend_rebalance
│   │   │   │                 #   (provider: "openai" | "anthropic")
│   │   │   ├── solhub.rs     # run_workflow, delta_calc, guard_rails,
│   │   │   │                 #   emit_webhook, require_approval
│   │   │   ├── kamino.rs, marinade.rs, drift.rs, orca.rs, raydium.rs  # stubs
│   │   │   ├── notifications/telegram.rs, discord.rs
│   │   │   └── test_plugin.rs   # EchoPlugin for testing
│   │   ├── executor/
│   │   │   ├── worker.rs        # ExecutorWorker.execute_run (the main loop)
│   │   │   ├── bundle_builder/  # BundleBuilder trait + Mock + Jito (feature-gated)
│   │   │   ├── rpc_submit.rs    # RpcSubmitBuilder — devnet-friendly direct submit
│   │   │   ├── simulator.rs     # Simulator trait + Rpc + Mock impls
│   │   │   ├── tip_calculator.rs
│   │   │   └── retry.rs         # exponential backoff
│   │   ├── trigger/
│   │   │   ├── cron.rs          # CronTriggers (tokio-cron-scheduler)
│   │   │   ├── scheduler.rs     # Polls Pending+Resumed, dispatches to executor
│   │   │   ├── webhook.rs       # HMAC verification helper
│   │   │   └── account_watch.rs # Geyser-backed (feature-gated)
│   │   ├── wallet/
│   │   │   ├── local.rs         # LocalKeypairSigner (reads .json keypair)
│   │   │   ├── agentic.rs       # Ephemeral signer with spend limit + TTL
│   │   │   └── turnkey.rs       # HSM signer (feature = "live-net")
│   │   └── geyser/
│   │       ├── router.rs        # DualFeedRouter (dedup window)
│   │       ├── yellowstone.rs   # gRPC client (feature-gated)
│   │       └── shredstream.rs   # Jito ShredStream (feature-gated)
│   └── Cargo.toml
├── api/                      # Axum REST + SSE
│   ├── src/
│   │   ├── main.rs           # build_router, axum::serve
│   │   ├── app.rs            # router with auth_middleware + rate_limit_middleware
│   │   ├── state.rs          # AppState { db, plugins, payment_verifier, treasury }
│   │   ├── error.rs          # AppError + IntoResponse (incl. PaymentRequired)
│   │   ├── types.rs          # request/response DTOs
│   │   ├── payment.rs        # PaymentVerifier (on-chain x402 verification)
│   │   ├── middleware/
│   │   │   ├── auth.rs       # Bearer SHA-256 hash → Organization
│   │   │   └── rate_limit.rs # token bucket per org (60/burst, 5/sec)
│   │   └── routes/
│   │       ├── workflows.rs  # CRUD + trigger
│   │       ├── runs.rs       # list, get, SSE logs, approve, reject
│   │       ├── webhooks.rs   # HMAC-validated trigger
│   │       ├── analytics.rs
│   │       ├── orgs.rs       # /me + api_keys CRUD
│   │       ├── credits.rs    # balance, topup_info, topup (x402), admin grant
│   │       ├── hub.rs        # public list, publish, payment_info, call (x402)
│   │       └── execute.rs    # one-shot transfer/program (MVP stubs)
│   └── Cargo.toml
├── cli/                      # `skh` binary
│   ├── src/
│   │   ├── main.rs
│   │   ├── client.rs         # ApiClient (reqwest + SSE streaming)
│   │   ├── config.rs         # ~/.config/skh/config.toml
│   │   └── commands/
│   │       ├── auth.rs       # login | status
│   │       ├── workflow.rs   # list | create | deploy | enable | disable | delete
│   │       ├── run.rs        # list | status | logs | cancel
│   │       ├── execute.rs    # transfer | contract-call
│   │       ├── billing.rs    # balance | topup
│   │       ├── x402.rs       # pay --workflow --keypair
│   │       └── config_cmd.rs # set | list
│   └── Cargo.toml
├── mcp-server/               # TypeScript MCP server (stdio)
│   ├── src/
│   │   ├── index.ts          # Server, ListTools, CallTool handlers
│   │   ├── api.ts            # ApiClient (HTTP)
│   │   ├── types.ts          # zod schemas
│   │   └── tools/            # 7 sk.* tools
│   └── package.json
├── migrations/               # sqlx (SQLite-compatible, Postgres-friendly)
├── idl/                      # Anchor IDL artifacts
├── deployments/devnet.json   # Devnet program IDs + deployer pubkey
├── tests/
│   ├── anchor/               # TS tests for Anchor programs (ts-mocha)
│   └── e2e/                  # End-to-end shell scripts (see Section 17)
├── docs/                     # this file, planning docs
├── Anchor.toml               # cluster + wallet + program IDs
├── Cargo.toml                # workspace manifest
├── package.json              # workspace-level npm (anchor TS tests)
└── .env / .env.example
```

---

## 3. Toolchain & prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.95 (pinned via `rust-toolchain.toml`) | `cargo` ships with it |
| Solana CLI / Agave | 3.1 | `~/.local/share/solana/install/active_release/bin` |
| Anchor (via avm) | 0.31.1 | `cargo install --git github.com/coral-xyz/anchor avm --locked` |
| `cargo-build-sbf` | 3.1 | Bundled with Solana / Agave install |
| Node.js | ≥ 20 | for MCP server + anchor TS tests |
| sqlite3 | any | for E2E DB seeding |
| Optional: PostgreSQL | 14+ | the schema is portable; tests use SQLite |

For dev you only need: **Rust + Node + sqlite3**. Anchor/Solana CLIs are only required
to redeploy programs.

---

## 4. Configuration (env vars)

`.env` (gitignored; copy from `.env.example` and fill in):

```bash
# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
SOLANA_NETWORK=devnet                   # devnet | mainnet-beta

# Wallet (for engine signer; keypair file path)
SOLHUB_KEYPAIR=./solhub-dev.json

# Database
DATABASE_URL=sqlite:./solhub-dev.db?mode=rwc

# API
API_PORT=8080
SOLHUB_TREASURY=FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb   # x402 recipient
SOLHUB_LAMPORTS_PER_CREDIT=10000                                # default
SOLHUB_ADMIN_TOKEN=<set-something-strong>                       # for admin grants

# Plugin keys (all optional; plugins fail gracefully if missing)
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
CRYPTOPANIC_TOKEN=...                   # without this, crypto_panic returns 404
TELEGRAM_BOT_TOKEN=...
DISCORD_BOT_TOKEN=...
SENDGRID_API_KEY=...

# Logging
RUST_LOG=info,solhub=debug
```

---

## 5. Build & start

### Quick start (dev)

```bash
# 1. Build everything
cargo build --workspace
(cd mcp-server && npm install && npm run build)

# 2. Run tests
cargo test --workspace --no-fail-fast      # ~199 Rust tests
(cd mcp-server && npm test)                # MCP unit tests

# 3. Start the stack
source .env
./target/debug/solhub-api &                # API on :8080
./target/debug/solhub-engine &             # Worker

# 4. Seed an org + API key (via sqlite3) — or via your admin path
sqlite3 ./solhub-dev.db <<SQL
INSERT INTO organizations (id, name, credits_usdc, created_at)
VALUES ('00000000-0000-0000-0000-000000000001', 'my-org', 100, strftime('%s','now'));
INSERT INTO api_keys (id, org_id, key_hash, created_at)
VALUES ('00000000-0000-0000-0000-000000000002',
        '00000000-0000-0000-0000-000000000001',
        '$(printf 'my-key' | sha256sum | awk '{print $1}')',
        strftime('%s','now'));
SQL

# 5. Drive the API
curl -H "Authorization: Bearer my-key" http://localhost:8080/v1/orgs/me
```

### Release build

```bash
cargo build --release -p api -p engine -p cli
./target/release/solhub-api &
./target/release/solhub-engine &
```

### Run all E2E demos

```bash
source .env
bash tests/e2e/api_full_suite.sh          # 43 endpoint assertions
bash tests/e2e/complex_workflow.sh        # multi-step bundled tx
bash tests/e2e/batch_transfer_e2e.sh      # batch_transfer action
bash tests/e2e/subworkflow_e2e.sh         # parent → child
bash tests/e2e/x402_full_e2e.sh           # paid hub call
bash tests/e2e/rebalancer_nodes.sh        # all rebalancer nodes
bash tests/e2e/rebalancer_combined.sh     # Signal Scout → Trade Executor
```

---

## 6. Component deep-dives

### 6.1 Anchor programs

Three programs in `programs/`; all deployed to devnet (see `deployments/devnet.json`).

**`workflow_registry`** — On-chain registration of workflows for trustlessness.
Stores: `owner, name, trigger_type, steps_hash (sha256), steps_cid (IPFS),
is_active, execution_count, platform_authority, timestamps`. PDA seed:
`["workflow", owner, name]`.

Instructions:
- `register_workflow(params)` — owner-signed; creates the PDA.
- `set_workflow_status(is_active)` — owner-only toggle.
- `record_execution()` — platform_authority-only; increments counter.

**`execution_vault`** — USDC-denominated credits with 80/20 creator/treasury split.
Accounts: `VaultAccount` per org, `CreatorAccount` per creator.

Instructions:
- `deposit_credits(amount)` — depositor signs; SPL transfer in.
- `debit_execution(fee_usdc)` — platform; emits `ExecutionBilled` event.
- `withdraw_creator(amount)` — creator-signed.

**`condition_oracle`** — Standardised CPI interface for protocols to expose
on-chain conditions. The default impl emits `ConditionEvaluated { met: true }`;
each protocol overrides.

### 6.2 Database (`db` crate)

sqlx over SQLite (Postgres-compatible SQL). Tables:

| Table | Purpose |
|---|---|
| `organizations` | tenant + `credits_usdc` balance |
| `api_keys` | SHA-256 hash, never raw |
| `workflows` | JSON-encoded `trigger_config` + `steps`, `is_active`, `is_public`, `fee_per_exec_usdc`, `onchain_pda` |
| `workflow_runs` | every execution; status, signature, fee, tip, JSON step_log, `resume_from_step_index` |
| `payments` | x402 payment records (sig, payer, recipient, amount, status) |
| `credit_ledger` | every grant/debit, append-only, with `balance_after` snapshot |

Run statuses: see [Section 8](#8-state-machine).

Key methods:
- `Db.connect(url)`, `connect_in_memory()`, `migrate()`
- Workflow: `create_workflow`, `get_workflow`, `list_workflows`, `update_workflow`, `delete_workflow` (soft), `set_workflow_pda`, `increment_execution_count`
- Run: `create_run`, `get_run`, `list_runs`, `list_runs_to_execute` (Pending+Resumed), `update_run_status`, `record_run_outcome`, `append_step_log`, `set_resume_index`
- Credits: `debit_credit_for_run`, `grant_credits`, `list_ledger`
- Payments: `create_payment`, `get_payment_by_signature`, `mark_payment_verified`

All queries are parameterised. The schema avoids Postgres-specific functions
(`uuid_generate_v4`, `JSONB`, `EXTRACT EPOCH`) — UUIDs are generated in Rust
and timestamps are stored as `INTEGER` unix-seconds.

### 6.3 Engine (`engine` crate)

The long-running worker. Bootstrap (`main.rs`):

1. Load `.env` via shell; read `DATABASE_URL`, `SOLANA_RPC_URL`, `SOLHUB_KEYPAIR`.
2. Connect to DB, run migrations.
3. Load keypair file → `LocalKeypairSigner`.
4. Build RPC client + `PluginRegistry::with_default_plugins()`.
   `register_solhub(db)` is also called to give the `solhub` plugin DB access.
5. Construct `ExecutorWorker` with `RpcSubmitBuilder` (direct devnet submit) and
   `RpcSimulator` (real `simulate_transaction` call).
6. Start `CronTriggers` — loads workflows with `trigger_type="cron"` from DB and
   schedules them via `tokio-cron-scheduler`. **NOTE:** loaded once at boot; new
   cron workflows require an engine restart to be scheduled.
7. Start `Scheduler` — every 500 ms calls `db.list_runs_to_execute(50)` and spawns
   `executor.execute_run(id)` for each.

**`ExecutorWorker.execute_run`** (core loop, in `executor/worker.rs`):

```
Pending → Triggered → Simulating
  for step in steps[resume_from..]:
    plugin = registry.get(step.plugin)
    out = match action_type:
      ReadOnly      => plugin.read(...)
      Notification  => plugin.notify(...)
      Transaction   => plugin.build_transactions(...) → append to queue
    if out.__pause__:
      append_step_log(WaitingApproval)
      set_resume_index(i + 1)
      update_run_status(WaitingApproval)
      return early
    append_step_log(Completed)
if queue non-empty:
  → Bundling → Submitted
  for tx in queue: signer.sign_transaction(tx)
  bundle_builder.build_and_submit(txs, signer)
  record_run_outcome(slot, signature, fee, tip)
→ Confirmed
```

**`BundleBuilder` trait** has three implementations:
- `MockBundleBuilder` — for tests; returns a fake bundle_id.
- `RpcSubmitBuilder` — calls `RpcClient::send_and_confirm_transaction_with_spinner_and_commitment`. The default in `main.rs`.
- `JitoBundleBuilder` (feature `live-net`) — stubbed; production wiring TBD.

### 6.4 REST API (`api` crate)

Axum 0.7. Routes are grouped:
- **Auth'd routes** — middleware chain: `auth_middleware` → `rate_limit_middleware`.
- **Public routes** — `/health`, `GET /v1/hub`, `GET /v1/hub/:id/payment_info`, `POST /v1/webhooks/:id`.

`auth_middleware` extracts `Bearer <key>`, SHA-256 hashes, looks up the org, attaches it to request extensions. Webhook routes skip auth (HMAC instead).

`rate_limit_middleware` is a hand-rolled token bucket per org (60 burst, 5/sec refill).

See [Section 15](#15-rest-api-reference) for the full route table.

### 6.5 CLI (`cli` crate)

`skh` binary, clap-derive command tree. Reads config from `~/.config/skh/config.toml`
(set via `skh config set` or interactively via `skh auth login`).

Top-level: `auth | workflow | run | execute | billing | x402 | config`. All
sub-commands go through `ApiClient` (reqwest). SSE log streaming via
`reqwest::Response::bytes_stream()`.

### 6.6 MCP server (`mcp-server/`)

TypeScript, `@modelcontextprotocol/sdk` over stdio. Exposes 7 tools that proxy
to the REST API:

| Tool | Maps to |
|---|---|
| `sk.create_workflow` | POST /v1/workflows |
| `sk.trigger_workflow` | POST /v1/workflows/:id/trigger |
| `sk.get_run_status` | GET /v1/runs/:id |
| `sk.list_workflows` | GET /v1/workflows |
| `sk.get_balance` | direct Solana JSON-RPC (not via API) |
| `sk.call_program` | POST /v1/execute/program (MVP stub) |
| `sk.publish_to_hub` | POST /v1/hub/publish |

Inputs are zod-validated. Auth via `SOLHUB_API_KEY` env var.

---

## 7. Plugin & node catalog

Every workflow step is `{plugin, action, params}`. The plugin contributes one of three
action types:

- **`ReadOnly`** — calls `plugin.read(...)`; no transaction. Used for data fetch,
  logic, approval gates.
- **`Transaction`** — calls `plugin.build_transactions(...)`. Returned txs are
  accumulated across all steps and submitted as a single bundle at the end.
- **`Notification`** — calls `plugin.notify(...)`; off-chain side effect (HTTP POST).

### Production-ready plugins

| Plugin | Actions | Type | Notes |
|---|---|---|---|
| **`system`** | `transfer` | Transaction | Native SOL transfer (system_instruction::transfer) |
| | `batch_transfer` | Transaction | 1-15 transfers in ONE tx |
| | `memo` | Transaction | SPL Memo program write |
| | `get_balance` | ReadOnly | SOL lamports via RPC |
| **`jupiter`** | `swap` | Transaction | Builds swap tx (lite-api.jup.ag/swap/v1) |
| | `quote` | ReadOnly | Best-route quote (price impact, route plan) |
| | `price` | ReadOnly | Jupiter Price v3 (multi-mint lookup) |
| **`portfolio`** | `snapshot` | ReadOnly | SOL+SPL balances + Jupiter price → USD value + weights |
| | `detect_drift` | ReadOnly | Flags when any holding drifts > threshold from target |
| | `current_weights_from_holdings` | ReadOnly | Helper: holdings array → `{sym: fraction}` |
| **`fear_greed`** | `current` | ReadOnly | alternative.me FNG (value 0-100 + classification) |
| | `history` | ReadOnly | Historical FNG array |
| **`news`** | `fetch_headlines` | ReadOnly | CoinDesk RSS (no auth needed) |
| | `crypto_panic` | ReadOnly | Requires `CRYPTOPANIC_TOKEN` env var |
| | `fetch_url` | ReadOnly | Generic GET (`max_bytes` truncation) |
| **`llm`** | `complete` | ReadOnly | Chat completion (provider: openai|anthropic) |
| | `analyze_sentiment` | ReadOnly | Structured JSON sentiment output |
| | `recommend_rebalance` | ReadOnly | Returns `{confidence, target_weights, reasoning}` |
| **`solhub`** | `run_workflow` | ReadOnly | Triggers + waits on child workflow |
| | `delta_calc` | ReadOnly | Current vs target weights → swap deltas |
| | `guard_rails` | ReadOnly | Slippage/size/confidence/no-trade checks |
| | `emit_webhook` | ReadOnly | POST `{trigger_data}` to another workflow's webhook (HMAC signed) |
| | `require_approval` | ReadOnly | Returns `__pause__` sentinel; executor pauses run |
| **`pyth`** | `read_price` | ReadOnly | Pyth feed parse via pyth-sdk-solana |
| | `staleness_check` | ReadOnly | Reject feeds older than `max_age_seconds` |
| **`notify.telegram`** | `send_message` | Notification | Telegram Bot API |
| **`notify.discord`** | `send_message`, `send_embed` | Notification | Webhook URL |

### Stub plugins (schemas advertised, `build_transactions` returns `NotImplemented`)

- `kamino`: `deposit`, `withdraw`, `claim_rewards`, `check_ltv`, `check_rewards`
- `marinade`: `stake`, `unstake`, `liquid_stake`, `check_rewards`
- `drift`: `open_position`, `close_position`, `check_margin`, `liquidation_guard`
- `orca`: `add_liquidity`, `remove_liquidity`, `collect_fees`, `rebalance_range`
- `raydium`: `swap`, `add_liquidity`, `harvest_yield`

These exist so a UI can render them; fill in `build_transactions` per protocol when needed.

---

## 8. State machine

```
Pending ──────► Triggered ──────► Simulating ──────► Bundling ──────► Submitted ──────► Confirmed
   │              │                  │                                    │
   ▼              ▼                  ▼                                    ▼
WaitingApproval  Failed           Failed                              Retrying
   │                                                                     │
   ▼                                                                     ▼
Resumed ──► Triggered (re-execution starts at resume_from_step_index)  Failed
   │
   ▼
Failed | Skipped (terminal)
```

- `Pending` — newly created.
- `Triggered` — picked up by scheduler.
- `Simulating` / `Bundling` / `Submitted` — engine in-flight.
- `Confirmed` — terminal success.
- `Failed` / `Skipped` — terminal failure.
- `WaitingApproval` — paused at an approval gate (see Section 10).
- `Resumed` — set by `/approve`; scheduler re-picks-up and executor restarts at
  `resume_from_step_index`.

Transition validation is in `engine/src/state/run.rs::transition_to()`. Illegal
transitions return `TransitionError`.

---

## 9. Workflow lifecycle (sequence)

```
client                API                  DB                Engine
  │                    │                    │                  │
  ├ POST /workflows ──►│                    │                  │
  │                    ├ create_workflow ──►│                  │
  │◄── 200, wf_id ─────┤                    │                  │
  │                    │                    │                  │
  ├ POST /trigger ────►│                    │                  │
  │                    ├ debit_credit(1) ──►│                  │
  │                    ├ create_run ───────►│                  │
  │◄── 200, run_id ────┤                    │                  │
  │                    │                    │  poll Pending ───┤
  │                    │                    │◄── run_id ───────┤
  │                    │                    │                  │
  │                    │                    │   for step in steps:
  │                    │                    │     read/build/notify
  │                    │                    │     append_step_log
  │                    │                    │   sign + submit txs
  │                    │                    │   record_outcome
  │                    │                    │   status=Confirmed
  │                    │                    │                  │
  ├ GET /runs/:id ────►│                    │                  │
  │                    ├ get_run ──────────►│                  │
  │◄── full run JSON ──┤                    │                  │
```

---

## 10. Approval gate

**Used for:** any step that demands explicit human confirmation before
continuing (Trade Executor sizes above threshold, irreversible actions, etc.).

**Mechanics:**

1. Workflow step is `{plugin: "solhub", action: "require_approval", params: {message: "…"}}`.
2. The executor calls `plugin.read("require_approval", …)` which returns
   `{"__pause__": true, "approval_required": true, "message": "…"}`.
3. The executor sees `__pause__: true` →
   - Appends the step to `step_log` with `status: "WaitingApproval"`.
   - Sets `resume_from_step_index = i + 1`.
   - Sets run status to `WaitingApproval`.
   - Returns early (no further steps run).
4. Operator calls `POST /v1/runs/:id/approve` → status becomes `Resumed`.
5. Scheduler picks up Resumed runs (`list_runs_to_execute` includes them).
6. Executor reads `resume_from_step_index` and continues from there. The completed
   steps' outputs remain in `step_log`; the new steps are appended.

**Rejection path:** `POST /v1/runs/:id/reject` with `{"reason": "…"}` →
status becomes `Failed` with `error_message="rejected: …"`.

---

## 11. Sub-workflows

`solhub.run_workflow` creates a child run, polls until terminal, returns the
child's `{child_run_id, status, steps_log, signature}` as the parent step's output.

Constraints:
- Max depth 3 (configurable on `SolhubPlugin`).
- Default timeout 60 s (`params.timeout_secs`).
- Cycle detection via `params.parent_run_id` (set by executor on nested calls).
- Child runs are normal runs with `triggered_by = "parent:<parent_run_id>"`.

Pattern:
```json
{
  "id": "delegate",
  "plugin": "solhub",
  "action": "run_workflow",
  "params": {"workflow_id": "<uuid>", "timeout_secs": 30}
}
```

---

## 12. Credit system

Every workflow trigger consumes **1 credit** from the org's `credits_usdc` field
(misnomer: the column stores an integer credit count). Behaviour:

| Trigger source | Insufficient credits behaviour |
|---|---|
| Manual (`POST /v1/workflows/:id/trigger`) | 402, run marked `Skipped` |
| Webhook (`POST /v1/webhooks/:id`) | run created + immediately marked `Skipped` (audit trail) |
| Cron | run created + marked `Skipped`, scheduler logs warning |

Every grant + debit is recorded in `credit_ledger` with `balance_after`. Inspect via
`GET /v1/orgs/me/credits`.

**Grant credits:**
- Admin: `POST /v1/orgs/me/credits/grant` with `X-Admin-Token: $SOLHUB_ADMIN_TOKEN`.
- Self-serve x402 top-up: `POST /v1/orgs/me/credits/topup` with `X-PAYMENT: solana:devnet:tx:<sig>` header (Section 13).

Rate: `lamports_per_credit` defaults to 10,000 (i.e. 0.00001 SOL = 1 credit;
1 SOL = 100,000 credits). Tunable via env `SOLHUB_LAMPORTS_PER_CREDIT`.

---

## 13. x402 payment-gated hub

HTTP 402 + a `X-PAYMENT` header carrying an on-chain proof. The platform verifies
the on-chain transaction post-hoc and grants access.

**Flow (`POST /v1/hub/:id/call` for a workflow with `fee_per_execution_usdc > 0`):**

1. Client `POST /v1/hub/:id/call` with no `X-PAYMENT` header.
2. Server responds `402 Payment Required` with body:
   ```json
   {
     "x402": "1",
     "payment": {
       "network": "solana-devnet",
       "asset": "SOL",
       "amount_lamports": <fee>,
       "recipient": "<treasury>",
       "memo": "hub-call:<workflow_id>"
     }
   }
   ```
3. Client transfers `amount_lamports` of SOL to `recipient` on devnet, gets `signature`.
4. Client re-`POST`s with header `X-PAYMENT: solana:devnet:tx:<signature>`.
5. Server's `PaymentVerifier.verify()`:
   - Fetches the tx via `get_transaction_with_config`.
   - Checks block_time freshness (≤ 600s).
   - Decodes the `VersionedTransaction`.
   - Verifies recipient balance delta ≥ required lamports.
6. If valid → records `payments` row, creates run, returns `200 {run_id, …}`.
7. Replay protection: same signature → `409 Conflict`.

**Topup uses the same protocol** at `POST /v1/orgs/me/credits/topup`, granting
`lamports / lamports_per_credit` credits.

---

## 14. Webhook trigger (HMAC)

`POST /v1/webhooks/:workflow_id` is public (no bearer). Validation:

1. Workflow must exist and `trigger_type == "webhook"`.
2. The workflow's `trigger_config.secret` is the HMAC key.
3. Request must include `X-SK-Signature: sha256=<hex>`.
4. Server computes `HMAC-SHA256(secret, raw_body)` and compares with constant-time
   equality. Mismatch → `401`.
5. On success: creates a run with `triggered_by = "webhook"`.

The payload (`{"trigger_data": <anything>}`) is stored on the run; **note**:
step params do not yet templated-interpolate this payload — that's a known
limitation (Section 20).

---

## 15. REST API reference

| Method | Path | Auth | Description |
|---|---|---|---|
| GET | `/health` | public | Liveness probe |
| GET | `/v1/orgs/me` | Bearer | Current org details |
| POST | `/v1/orgs/me/api_keys` | Bearer | Create key (raw returned ONCE) |
| GET | `/v1/orgs/me/api_keys` | Bearer | List keys (no raw values) |
| DELETE | `/v1/orgs/me/api_keys/:id` | Bearer | Revoke |
| GET | `/v1/orgs/me/credits` | Bearer | Balance + recent ledger |
| GET | `/v1/orgs/me/credits/topup_info` | Bearer | x402 reqs for credit topup |
| POST | `/v1/orgs/me/credits/topup` | Bearer + X-PAYMENT | Grant credits after on-chain payment |
| POST | `/v1/orgs/me/credits/grant` | Bearer + X-Admin-Token | Admin grant |
| POST | `/v1/workflows` | Bearer | Create workflow |
| GET | `/v1/workflows` | Bearer | List |
| GET | `/v1/workflows/:id` | Bearer | Get |
| PATCH | `/v1/workflows/:id` | Bearer | Update (is_active, trigger, steps) |
| DELETE | `/v1/workflows/:id` | Bearer | Soft delete (is_active=false) |
| POST | `/v1/workflows/:id/trigger` | Bearer | Manual fire (consumes 1 credit) |
| GET | `/v1/runs` | Bearer | List (filters: workflow_id, status, limit) |
| GET | `/v1/runs/:run_id` | Bearer | Full run detail |
| GET | `/v1/runs/:run_id/logs` | Bearer | Server-Sent Events stream |
| POST | `/v1/runs/:run_id/approve` | Bearer | Resume WaitingApproval run |
| POST | `/v1/runs/:run_id/reject` | Bearer | Mark Failed with reason |
| POST | `/v1/webhooks/:workflow_id` | HMAC | Inbound trigger |
| GET | `/v1/analytics` | Bearer | Aggregate execution stats |
| GET | `/v1/hub` | public | List public workflows |
| POST | `/v1/hub/publish` | Bearer | Mark workflow public + set fee |
| GET | `/v1/hub/:id/payment_info` | public | x402 payment requirements |
| POST | `/v1/hub/:id/call` | Bearer + X-PAYMENT | Trigger paid hub workflow |
| POST | `/v1/execute/program` | Bearer | One-shot program call (MVP stub) |
| POST | `/v1/execute/transfer` | Bearer | One-shot transfer (MVP stub) |

**HTTP code conventions:**
- `401` — missing/invalid bearer or HMAC.
- `402` — payment required (hub call, optionally trigger when out of credits).
- `403` — admin token mismatch.
- `404` — resource not found OR wrong org.
- `409` — payment signature replay.
- `429` — rate limit exceeded.
- `400` — invalid body (e.g. unknown trigger type).

---

## 16. MCP tool reference

Run: `node mcp-server/dist/index.js` (stdio). Configure clients with
`SOLHUB_API_URL` + `SOLHUB_API_KEY`.

7 tools listed in [Section 6.6](#66-mcp-server-mcp-server). Use `tools/list` to
get the JSON schemas; use `tools/call` with `{name, arguments}`.

---

## 17. E2E test scripts

All under `tests/e2e/`. Run with the API+engine binaries already built
(`cargo build --release -p api -p engine`).

| Script | What it verifies |
|---|---|
| `api_full_suite.sh` | **43 endpoint assertions** — auth, CRUD, runs, SSE, approval, webhook, analytics, credits, hub, x402, soft-delete. Hermetic (own in-memory DB). |
| `complex_workflow.sh` | 4-step workflow: Read → Transaction → Transaction → Read. Asserts 2 transfers bundled into 1 on-chain signature. |
| `batch_transfer_e2e.sh` | `system.batch_transfer` — 3 recipients in one tx. Validates on-chain via `solana confirm -v`. |
| `subworkflow_e2e.sh` | Parent workflow uses `solhub.run_workflow` to invoke a child; verifies parent's step output includes the child's run record + signature. |
| `devnet_e2e.sh` | Original full-stack: workflow CRUD, trigger, real SOL transfer, webhook good/bad, analytics, MCP smoke. |
| `mcp_and_sse.sh` | MCP `tools/list` returns 7 tools; `tools/call sk.list_workflows` works; SSE stream delivers `run_complete`. |
| `x402_full_e2e.sh` | Mints a fresh payer wallet, funds from treasury, runs the full x402 demo: 402 without payment → real SOL transfer → 200 with X-PAYMENT → 409 on replay. |
| `rebalancer_nodes.sh` | All 13 rebalancer-related plugin actions in single-step workflows. |
| `rebalancer_combined.sh` | **Full Signal Scout → Trade Executor flow:** portfolio.snapshot → fear_greed → news → jupiter.price → llm.recommend_rebalance → solhub.emit_webhook → (webhook triggers Trade Executor) → delta_calc → jupiter.quote → guard_rails → require_approval → /approve → on-chain transfer. |

---

## 18. Devnet deployment

Current devnet program IDs (from `deployments/devnet.json`):

```
workflow_registry  Eemnq9Fv55B2TNi5zKNSQyQDd6CKFBUJMfcgtJUiJuFB
execution_vault    4CFgDzuLnfdTThgNXTknhXyshzsidDQFtNCxsoMnBHJn
condition_oracle   JwYqHkFc9w3bwZuK87FEE2jviVsVCDkp7RdBXQGey7h
Deployer wallet    FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb
```

Deploying afresh (e.g. mainnet upgrade):

```bash
# 1. Generate / sync program-id keypairs
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
anchor build           # generates target/deploy/*.so + *-keypair.json on first run
anchor keys list       # shows the newly generated program IDs
anchor keys sync       # rewrites declare_id! in source + Anchor.toml
anchor build           # rebuild with corrected IDs

# 2. Deploy each program
solana program deploy --program-id target/deploy/workflow_registry-keypair.json \
  --url devnet --keypair ./solhub-dev.json \
  target/deploy/workflow_registry.so
# ... repeat for execution_vault, condition_oracle

# 3. Update deployments/devnet.json
# 4. Run the Anchor TS test suite against devnet:
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
  ANCHOR_WALLET=./solhub-dev.json \
  npx ts-mocha -p ./tsconfig.json -t 300000 'tests/anchor/**/*.ts'
```

Expected: 11/11 anchor tests pass on devnet.

---

## 19. Security & operational notes

- **API keys** are stored as SHA-256 hashes only; the raw key is returned ONCE at
  creation. Lost keys cannot be recovered.
- **Wallet keypairs** (`solhub-dev.json`) are gitignored. For production, swap
  `LocalKeypairSigner` for `TurnkeyWallet` (feature `live-net`). Private keys never
  appear in memory when using Turnkey.
- **Webhook HMAC**: constant-time comparison via `constant_time_eq`. Reject any
  webhook without valid `X-SK-Signature`.
- **SQL injection**: all queries use sqlx parameter binding. No string concat.
- **Rate limiting**: 60 burst / 5 per second per org. Drops the request at 429.
- **Replay protection** on x402: every signature is unique-indexed in `payments`.
- **Admin endpoints** gated by `SOLHUB_ADMIN_TOKEN` env (don't use the default
  in production).
- **Logs** should never contain raw API keys or payment signatures in production
  (current `tracing` config logs at `info` — review before mainnet).

---

## 20. Known limitations & follow-ups

1. **Step output → next step params templating.** Today, every step's `params`
   are static JSON in the workflow definition. The Trade Executor demo hardcodes
   what would normally be templated from Signal Scout's output. Implementing
   `${steps.<id>.output.<path>}` and `${trigger.<field>}` is the next big feature.

2. **Cron hot-reload.** `CronTriggers::load_and_start` runs once at engine boot.
   New cron workflows require an engine restart. Fix: have the API publish to
   a channel the engine subscribes to; or re-load periodically.

3. **Slot / fee fields.** `record_run_outcome` is called with `slot=0, fee=0`
   because `RpcSubmitBuilder` doesn't fetch them post-confirmation. Cosmetic;
   easy follow-up.

4. **Jito live submission.** `JitoBundleBuilder` is a stub. Production needs
   the real searcher client + tip account selection + bundle-status polling.

5. **Stub plugins** (`kamino`, `marinade`, `drift`, `orca`, `raydium`) advertise
   schemas but return `NotImplemented` for `build_transactions`. Fill in
   protocol-by-protocol when needed.

6. **Geyser feeds** behind `feature = "live-net"` only — Yellowstone gRPC +
   ShredStream stubs exist but aren't wired. The `DualFeedRouter` (with dedup)
   is implemented and unit-tested.

7. **Drift-based trigger** ("rebalance when any holding drifts > 5%") not yet
   wired as a first-class trigger type. The `portfolio.detect_drift` action
   exists — combine with cron to poll, or build a dedicated trigger.

8. **Approval timeouts** are accepted as a param on `require_approval` but not
   currently enforced. Implementing: a periodic sweep that flips
   `WaitingApproval` runs older than `timeout_secs` to `Skipped`.

9. **Templating with HMAC re-sign** for `emit_webhook`: each cross-workflow
   payload is HMAC-signed using the target workflow's secret. The emitting
   workflow must know the secret (currently in the step params; safer would be
   to store per-org webhook signing keys).

10. **Mainnet readiness** requires: real Jito wiring, Turnkey wallet integration,
    multi-region HA, Squads multisig for program upgrade authority, and a
    security audit of the Anchor programs.

---

*SolHub Backend · Operator Guide*
