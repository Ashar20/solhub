# SolHub — Backend Specification
### Claude Code Handoff · v1.0 · May 2026

> **Status:** READY FOR DEVELOPMENT  
> **Stack:** Rust · Anchor · TypeScript · PostgreSQL · Redis · Jito · Geyser gRPC  
> **Owner:** Engineering Team

---

## Table of Contents

1. [Project Context](#1-project-context)
2. [Repository Structure](#2-repository-structure)
3. [Onchain Programs (Anchor)](#3-onchain-programs-anchor)
4. [Workflow Execution Engine](#4-workflow-execution-engine)
5. [Data Ingestion Layer (Geyser + ShredStream)](#5-data-ingestion-layer-geyser--shredstream)
6. [Jito Transaction Execution](#6-jito-transaction-execution)
7. [Protocol Plugin System](#7-protocol-plugin-system)
8. [REST API (Axum)](#8-rest-api-axum)
9. [MCP Server](#9-mcp-server)
10. [Wallet & Key Infrastructure](#10-wallet--key-infrastructure)
11. [Database Schema](#11-database-schema)
12. [CLI (`skh`)](#12-cli-skh)
13. [Environment Configuration](#13-environment-configuration)
14. [Testing Requirements](#14-testing-requirements)
15. [Non-Negotiable Rules](#15-non-negotiable-rules)
16. [Build Priority Order](#16-build-priority-order)

---

## 1. Project Context

SolanaKeeper is a **blockchain automation and execution reliability platform** built natively for Solana. Think KeeperHub (EVM) but rebuilt from scratch for Solana's architecture: 400ms blocks, Proof of History, Sealevel parallel execution, and Jito MEV infrastructure.

**Core value proposition:**
- Users and AI agents define automation workflows (triggers → conditions → actions)
- The platform guarantees onchain execution via Jito bundles — no dropped transactions
- No infrastructure management needed by the user

**What makes this hard on Solana (vs EVM):**
- No public mempool — transactions go through gossip or private relays (Jito)
- 400ms block windows mean reaction time is everything
- Compute budget must be estimated per-transaction, not globally
- Account-based model (not contract storage) requires understanding ALTs and PDAs

---

## 2. Repository Structure

```
solanakeeper/
├── programs/                        # Anchor programs (Rust)
│   ├── workflow-registry/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── execution-vault/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   └── condition-oracle/
│       ├── src/lib.rs
│       └── Cargo.toml
│
├── engine/                          # Core workflow execution engine (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── trigger/
│       │   ├── mod.rs
│       │   ├── cron.rs              # Cron-based triggers
│       │   ├── account_watch.rs     # Geyser-backed account monitors
│       │   └── webhook.rs           # Inbound HTTP trigger receiver
│       ├── executor/
│       │   ├── mod.rs
│       │   ├── bundle_builder.rs    # Jito bundle construction
│       │   ├── tip_calculator.rs    # Dynamic tip from live telemetry
│       │   ├── retry.rs             # Exponential backoff + nonce mgmt
│       │   └── simulator.rs         # Pre-flight transaction simulation
│       ├── plugins/
│       │   ├── mod.rs               # Plugin registry + trait definition
│       │   ├── jupiter.rs
│       │   ├── kamino.rs
│       │   ├── marinade.rs
│       │   ├── drift.rs
│       │   ├── orca.rs
│       │   ├── raydium.rs
│       │   ├── pyth.rs
│       │   └── notifications/
│       │       ├── telegram.rs
│       │       ├── discord.rs
│       │       └── sendgrid.rs
│       ├── geyser/
│       │   ├── mod.rs
│       │   ├── yellowstone.rs       # Yellowstone gRPC client
│       │   ├── shredstream.rs       # Jito ShredStream client
│       │   └── router.rs            # Dual-feed fan-in + dedup
│       ├── wallet/
│       │   ├── mod.rs
│       │   ├── turnkey.rs           # Turnkey HSM signing API
│       │   └── agentic.rs           # Scoped ephemeral signer generation
│       ├── state/
│       │   ├── mod.rs
│       │   ├── workflow.rs          # Workflow state machine
│       │   └── run.rs               # Run log + status tracking
│       └── config.rs
│
├── api/                             # REST API server (Rust, Axum)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── routes/
│       │   ├── workflows.rs
│       │   ├── runs.rs
│       │   ├── analytics.rs
│       │   ├── webhooks.rs
│       │   └── orgs.rs
│       ├── middleware/
│       │   ├── auth.rs              # API key validation
│       │   └── rate_limit.rs
│       └── types.rs
│
├── mcp-server/                      # MCP server for AI agent integration (TypeScript)
│   ├── package.json
│   └── src/
│       ├── index.ts
│       ├── tools/
│       │   ├── create_workflow.ts
│       │   ├── trigger_workflow.ts
│       │   ├── get_run_status.ts
│       │   ├── list_workflows.ts
│       │   ├── get_balance.ts
│       │   ├── call_program.ts
│       │   └── publish_to_hub.ts
│       └── types.ts
│
├── cli/                             # `skh` CLI binary (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── commands/
│           ├── workflow.rs
│           ├── run.rs
│           ├── execute.rs
│           └── auth.rs
│
├── migrations/                      # PostgreSQL migrations (sqlx)
│   ├── 001_workflows.sql
│   ├── 002_runs.sql
│   ├── 003_orgs.sql
│   └── 004_marketplace.sql
│
├── tests/
│   ├── anchor/                      # Anchor test suite
│   ├── integration/                 # Full workflow integration tests
│   └── fixtures/
│
├── Anchor.toml
├── Cargo.toml                       # Workspace manifest
└── .env.example
```

---

## 3. Onchain Programs (Anchor)

### 3.1 WorkflowRegistry Program

**Purpose:** Stores workflow metadata onchain for trustlessness and Marketplace discoverability. The offchain engine reads this to validate that a workflow is legitimately registered.

**Program ID:** Deploy to devnet first; mainnet address TBD.

```rust
// programs/workflow-registry/src/lib.rs

use anchor_lang::prelude::*;

declare_id!("REGISTRY_PROGRAM_ID_PLACEHOLDER");

#[program]
pub mod workflow_registry {
    use super::*;

    /// Register a new workflow on-chain.
    /// Owner signs. Steps are stored off-chain (IPFS/DB); only hash stored here.
    pub fn register_workflow(
        ctx: Context<RegisterWorkflow>,
        params: RegisterParams,
    ) -> Result<()> {
        let workflow = &mut ctx.accounts.workflow;
        workflow.owner = ctx.accounts.owner.key();
        workflow.name = params.name;
        workflow.trigger_type = params.trigger_type;
        workflow.steps_hash = params.steps_hash;    // SHA-256 of JSON steps
        workflow.steps_cid = params.steps_cid;      // IPFS CID for encrypted steps
        workflow.is_active = true;
        workflow.execution_count = 0;
        workflow.created_at = Clock::get()?.unix_timestamp;
        workflow.bump = ctx.bumps.workflow;
        Ok(())
    }

    /// Toggle workflow active/inactive. Owner only.
    pub fn set_workflow_status(
        ctx: Context<SetStatus>,
        is_active: bool,
    ) -> Result<()> {
        ctx.accounts.workflow.is_active = is_active;
        Ok(())
    }

    /// Called by platform authority after each successful execution.
    pub fn record_execution(ctx: Context<RecordExecution>) -> Result<()> {
        let workflow = &mut ctx.accounts.workflow;
        workflow.execution_count = workflow.execution_count.checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        workflow.last_executed_at = Clock::get()?.unix_timestamp;
        Ok(())
    }
}

#[account]
pub struct WorkflowAccount {
    pub owner: Pubkey,                  // 32
    pub name: String,                   // 4 + 64
    pub trigger_type: u8,               // 1  (0=cron, 1=account_watch, 2=webhook)
    pub steps_hash: [u8; 32],           // 32 (SHA-256)
    pub steps_cid: String,              // 4 + 64 (IPFS CID)
    pub is_active: bool,                // 1
    pub execution_count: u64,           // 8
    pub created_at: i64,                // 8
    pub last_executed_at: i64,          // 8
    pub bump: u8,                       // 1
}
// Space: 8 + 32 + 68 + 1 + 64 + 68 + 1 + 8 + 8 + 8 + 1 = ~267 bytes → allocate 300

#[derive(Accounts)]
#[instruction(params: RegisterParams)]
pub struct RegisterWorkflow<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 300,
        seeds = [b"workflow", owner.key().as_ref(), params.name.as_bytes()],
        bump
    )]
    pub workflow: Account<'info, WorkflowAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterParams {
    pub name: String,
    pub trigger_type: u8,
    pub steps_hash: [u8; 32],
    pub steps_cid: String,
}
```

---

### 3.2 ExecutionVault Program

**Purpose:** Handles per-execution USDC billing. Callers pre-deposit credits; the platform deducts per run. Creators earn 80% of fees.

```rust
// programs/execution-vault/src/lib.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("VAULT_PROGRAM_ID_PLACEHOLDER");

#[program]
pub mod execution_vault {
    use super::*;

    /// Caller deposits USDC credits into their org vault.
    pub fn deposit_credits(ctx: Context<DepositCredits>, amount: u64) -> Result<()> {
        token::transfer(ctx.accounts.into_transfer_context(), amount)?;
        ctx.accounts.vault.credits = ctx.accounts.vault.credits
            .checked_add(amount).ok_or(ErrorCode::Overflow)?;
        Ok(())
    }

    /// Platform deducts fee per execution.
    /// 80% goes to creator_account, 20% stays in treasury.
    pub fn debit_execution(
        ctx: Context<DebitExecution>,
        fee_usdc: u64,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.caller_vault;
        require!(vault.credits >= fee_usdc, ErrorCode::InsufficientCredits);
        vault.credits -= fee_usdc;

        let creator_share = fee_usdc * 80 / 100;
        let _treasury_share = fee_usdc - creator_share;

        ctx.accounts.creator_account.balance = ctx.accounts.creator_account.balance
            .checked_add(creator_share).ok_or(ErrorCode::Overflow)?;

        emit!(ExecutionBilled {
            workflow: ctx.accounts.workflow.key(),
            fee_usdc,
            creator_share,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    /// Creator withdraws their accumulated balance.
    pub fn withdraw_creator(ctx: Context<WithdrawCreator>, amount: u64) -> Result<()> {
        let creator = &mut ctx.accounts.creator_account;
        require!(creator.balance >= amount, ErrorCode::InsufficientBalance);
        creator.balance -= amount;
        token::transfer(ctx.accounts.into_transfer_context(), amount)?;
        Ok(())
    }
}

#[account]
pub struct VaultAccount {
    pub org_id: Pubkey,
    pub credits: u64,           // USDC in lamports (6 decimals)
    pub total_spent: u64,
    pub bump: u8,
}

#[account]
pub struct CreatorAccount {
    pub owner: Pubkey,
    pub balance: u64,
    pub total_earned: u64,
    pub bump: u8,
}

#[event]
pub struct ExecutionBilled {
    pub workflow: Pubkey,
    pub fee_usdc: u64,
    pub creator_share: u64,
    pub timestamp: i64,
}
```

---

### 3.3 ConditionOracle Interface

**Purpose:** Standardised CPI interface that external protocols implement. Allows their state to be consumed as a workflow condition natively onchain without platform changes.

```rust
// programs/condition-oracle/src/lib.rs

use anchor_lang::prelude::*;

declare_id!("ORACLE_INTERFACE_ID_PLACEHOLDER");

/// Any protocol implements this trait to become a native SolanaKeeper condition.
/// SolanaKeeper calls evaluate() via CPI before firing the action step.
#[program]
pub mod condition_oracle {
    use super::*;

    /// Returns true if the condition is met, false otherwise.
    /// params: protocol-specific encoded condition parameters (borsh).
    pub fn evaluate(ctx: Context<Evaluate>, params: Vec<u8>) -> Result<bool> {
        // Default implementation: always true (protocols override this)
        let _ = (ctx, params);
        Ok(true)
    }
}

#[derive(Accounts)]
pub struct Evaluate<'info> {
    /// CHECK: protocol's state account — validated by the implementing program
    pub state_account: UncheckedAccount<'info>,
    pub caller: Signer<'info>,
}
```

---

## 4. Workflow Execution Engine

### 4.1 Workflow State Machine

Each workflow run transitions through these states:

```
PENDING → TRIGGERED → SIMULATING → BUNDLING → SUBMITTED → CONFIRMED
                                                    ↓
                                               RETRYING (up to 3x, exponential backoff)
                                                    ↓
                                               FAILED (logged with full error context)
```

### 4.2 Core Engine Types

```rust
// engine/src/state/workflow.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub trigger: TriggerConfig,
    pub steps: Vec<WorkflowStep>,
    pub is_active: bool,
    pub onchain_pda: Option<String>,   // WorkflowAccount PDA address
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerConfig {
    Cron { schedule: String },         // cron expression: "*/5 * * * *"
    AccountWatch {
        account: String,               // base58 pubkey
        condition: WatchCondition,
    },
    Webhook { secret: String },        // HMAC secret for validation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchCondition {
    BalanceAbove { lamports: u64 },
    BalanceBelow { lamports: u64 },
    DataChanges,                       // any account data mutation
    ProgramLog { pattern: String },    // regex match on program log
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub plugin: String,                // e.g. "kamino", "jupiter", "notify.telegram"
    pub action: String,                // e.g. "claim_rewards", "swap"
    pub params: serde_json::Value,     // plugin-specific params
    pub condition: Option<String>,     // expression evaluated against prev step output
    pub on_error: OnError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnError {
    Abort,                             // stop workflow, mark FAILED
    Skip,                              // skip this step, continue
    Retry { max_attempts: u8 },
}
```

### 4.3 Run Log Structure

```rust
// engine/src/state/run.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub triggered_by: TriggerSource,
    pub status: RunStatus,
    pub steps: Vec<StepLog>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub slot: Option<u64>,
    pub signature: Option<String>,     // Solana tx signature
    pub fee_lamports: Option<u64>,
    pub jito_tip_lamports: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepLog {
    pub step_id: String,
    pub status: StepStatus,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunStatus {
    Pending, Triggered, Simulating, Bundling,
    Submitted, Confirmed, Retrying, Failed, Skipped,
}
```

---

## 5. Data Ingestion Layer (Geyser + ShredStream)

### 5.1 Architecture

The data layer uses **two parallel feeds**. A smart router fires whichever delivers first, with a deduplication window to prevent double-triggering.

```
Yellowstone gRPC ──────┐
                        ├──→ DualFeedRouter ──→ TriggerEngine
Jito ShredStream ───────┘      (dedup: 500ms window)
```

**Why both?**
- Yellowstone: filtered, structured account/tx streams — good for account-watch triggers
- ShredStream: raw shreds from block leaders — arrives ~2 slots earlier, critical for latency-sensitive actions

### 5.2 Yellowstone gRPC Client

```rust
// engine/src/geyser/yellowstone.rs

use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;

pub struct YellowstoneClient {
    client: GeyserGrpcClient<impl Interceptor>,
    endpoint: String,
    x_token: String,
}

impl YellowstoneClient {
    pub async fn new(endpoint: &str, x_token: &str) -> Result<Self> {
        let client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?
            .x_token(Some(x_token.to_string()))?
            .connect()
            .await?;
        Ok(Self { client, endpoint: endpoint.to_string(), x_token: x_token.to_string() })
    }

    /// Subscribe to account updates for a list of pubkeys.
    pub async fn subscribe_accounts(
        &mut self,
        accounts: Vec<String>,
        tx: mpsc::Sender<AccountUpdate>,
    ) -> Result<()> {
        let mut filters = HashMap::new();
        filters.insert(
            "account_monitor".to_string(),
            SubscribeRequestFilterAccounts {
                account: accounts,
                owner: vec![],
                filters: vec![],
            },
        );

        let request = SubscribeRequest {
            accounts: filters,
            ..Default::default()
        };

        let (_, mut stream) = self.client.subscribe_with_request(Some(request)).await?;

        while let Some(msg) = stream.next().await {
            match msg?.update_oneof {
                Some(UpdateOneof::Account(account_update)) => {
                    let _ = tx.send(account_update).await;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```

### 5.3 Dual-Feed Router

```rust
// engine/src/geyser/router.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct DualFeedRouter {
    /// Deduplication window — ignore duplicate events within this duration
    dedup_window: Duration,
    /// Recent event hashes with their first-seen timestamp
    seen: HashMap<[u8; 32], Instant>,
    output: mpsc::Sender<CanonicalEvent>,
}

impl DualFeedRouter {
    pub fn new(output: mpsc::Sender<CanonicalEvent>) -> Self {
        Self {
            dedup_window: Duration::from_millis(500),
            seen: HashMap::new(),
            output,
        }
    }

    /// Accept event from either Yellowstone or ShredStream.
    /// First arrival wins; duplicates within dedup_window are dropped.
    pub async fn ingest(&mut self, event: RawEvent) {
        let key = event.content_hash();
        let now = Instant::now();

        // Cleanup old entries
        self.seen.retain(|_, ts| now.duration_since(*ts) < self.dedup_window);

        if self.seen.contains_key(&key) {
            return; // duplicate — drop
        }
        self.seen.insert(key, now);
        let _ = self.output.send(event.into_canonical()).await;
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalEvent {
    pub event_type: EventType,
    pub account: String,
    pub slot: u64,
    pub data: serde_json::Value,
    pub received_at: std::time::SystemTime,
}
```

---

## 6. Jito Transaction Execution

### 6.1 Bundle Builder

```rust
// engine/src/executor/bundle_builder.rs

use jito_searcher_client::{get_searcher_client, SearcherClient};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    transaction::VersionedTransaction,
    message::VersionedMessage,
};

pub struct BundleBuilder {
    searcher_client: SearcherClient,
    rpc_client: Arc<RpcClient>,
    tip_calculator: TipCalculator,
}

impl BundleBuilder {
    /// Build a Jito bundle from workflow step transactions.
    /// Max 5 transactions per bundle.
    pub async fn build_and_submit(
        &self,
        transactions: Vec<VersionedTransaction>,
        wallet: &Arc<dyn Signer>,
    ) -> Result<BundleResult> {
        assert!(transactions.len() <= 5, "Jito bundles max 5 transactions");

        // 1. Simulate all transactions first
        for tx in &transactions {
            self.simulate(tx).await?;
        }

        // 2. Calculate dynamic tip based on current network conditions
        let tip_lamports = self.tip_calculator.calculate().await?;

        // 3. Append tip transaction (transfer to Jito tip account)
        let tip_ix = system_instruction::transfer(
            &wallet.pubkey(),
            &self.tip_calculator.tip_account(),
            tip_lamports,
        );
        let tip_tx = self.build_tip_transaction(tip_ix, wallet).await?;

        // 4. Build bundle: [workflow_txs..., tip_tx]
        let mut bundle_txs = transactions;
        bundle_txs.push(tip_tx);

        // 5. Submit to Jito block engine
        let bundle_id = self.searcher_client
            .send_bundle(bundle_txs)
            .await?;

        Ok(BundleResult { bundle_id, tip_lamports })
    }

    async fn simulate(&self, tx: &VersionedTransaction) -> Result<SimulationResult> {
        let result = self.rpc_client
            .simulate_transaction(tx)
            .await?;

        if let Some(err) = result.err {
            return Err(anyhow::anyhow!("Simulation failed: {:?}", err));
        }
        Ok(SimulationResult {
            units_consumed: result.units_consumed.unwrap_or(200_000),
        })
    }
}
```

### 6.2 Dynamic Tip Calculator

```rust
// engine/src/executor/tip_calculator.rs

/// Calculates optimal Jito tip based on recent block telemetry.
/// Target: land in top 25% of bundles without overpaying.
pub struct TipCalculator {
    /// Rolling window of recent successful bundle tips (last 50 blocks)
    recent_tips: VecDeque<u64>,
    /// Minimum tip floor in lamports
    min_tip: u64,
}

impl TipCalculator {
    pub fn new() -> Self {
        Self {
            recent_tips: VecDeque::with_capacity(50),
            min_tip: 1_000,       // 0.000001 SOL minimum
        }
    }

    /// Returns tip in lamports.
    /// Strategy: 75th percentile of recent tips, capped at 0.01 SOL.
    pub async fn calculate(&self) -> Result<u64> {
        if self.recent_tips.is_empty() {
            return Ok(self.min_tip);
        }

        let mut sorted: Vec<u64> = self.recent_tips.iter().copied().collect();
        sorted.sort_unstable();
        let p75_index = sorted.len() * 75 / 100;
        let p75_tip = sorted[p75_index];

        Ok(p75_tip
            .max(self.min_tip)
            .min(10_000_000))  // cap: 0.01 SOL
    }

    pub fn record_successful_tip(&mut self, tip: u64) {
        if self.recent_tips.len() == 50 {
            self.recent_tips.pop_front();
        }
        self.recent_tips.push_back(tip);
    }
}
```

### 6.3 Retry Logic

```rust
// engine/src/executor/retry.rs

pub struct RetryPolicy {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    pub fn default() -> Self {
        Self { max_attempts: 3, base_delay_ms: 500, max_delay_ms: 8_000 }
    }

    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;
        loop {
            match f().await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.max_attempts {
                        return Err(e);
                    }
                    // Exponential backoff: 500ms, 1000ms, 2000ms...
                    let delay = (self.base_delay_ms * 2u64.pow(attempt as u32 - 1))
                        .min(self.max_delay_ms);
                    tracing::warn!("Attempt {} failed: {:?}. Retrying in {}ms", attempt, e, delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}
```

---

## 7. Protocol Plugin System

### 7.1 Plugin Trait

```rust
// engine/src/plugins/mod.rs

use async_trait::async_trait;

#[async_trait]
pub trait SolanaKeeperPlugin: Send + Sync {
    /// Unique plugin identifier, e.g. "kamino", "jupiter"
    fn id(&self) -> &'static str;

    /// Human-readable name for UI display
    fn name(&self) -> &'static str;

    /// All actions this plugin supports
    fn actions(&self) -> Vec<ActionDefinition>;

    /// Execute an action. Returns the transaction(s) to include in the bundle.
    /// DO NOT submit transactions here — return them for the bundle builder.
    async fn build_transactions(
        &self,
        action: &str,
        params: &serde_json::Value,
        wallet_pubkey: &Pubkey,
        rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError>;

    /// Read-only call — returns data without creating a transaction.
    async fn read(
        &self,
        action: &str,
        params: &serde_json::Value,
        rpc: &RpcClient,
    ) -> Result<serde_json::Value, PluginError>;
}

#[derive(Debug, Clone)]
pub struct ActionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action_type: ActionType,
    pub params_schema: serde_json::Value,   // JSON Schema for param validation
    pub returns_schema: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    ReadOnly,        // No transaction — only returns data
    Transaction,     // Produces one or more transactions
    Notification,    // Sends external notification; no onchain tx
}
```

### 7.2 Jupiter Plugin (Reference Implementation)

```rust
// engine/src/plugins/jupiter.rs

use super::{SolanaKeeperPlugin, ActionDefinition, ActionType, PluginError};
use jupiter_swap_api_client::{JupiterSwapApiClient, QuoteRequest, SwapRequest};

pub struct JupiterPlugin {
    api_client: JupiterSwapApiClient,
}

impl JupiterPlugin {
    pub fn new() -> Self {
        Self {
            api_client: JupiterSwapApiClient::new("https://quote-api.jup.ag/v6".to_string()),
        }
    }
}

#[async_trait]
impl SolanaKeeperPlugin for JupiterPlugin {
    fn id(&self) -> &'static str { "jupiter" }
    fn name(&self) -> &'static str { "Jupiter" }

    fn actions(&self) -> Vec<ActionDefinition> {
        vec![
            ActionDefinition {
                id: "swap".to_string(),
                name: "Swap Tokens".to_string(),
                description: "Best-route token swap via Jupiter aggregator".to_string(),
                action_type: ActionType::Transaction,
                params_schema: serde_json::json!({
                    "type": "object",
                    "required": ["input_mint", "output_mint", "amount"],
                    "properties": {
                        "input_mint":   { "type": "string", "description": "Input token mint address" },
                        "output_mint":  { "type": "string", "description": "Output token mint address" },
                        "amount":       { "type": "integer", "description": "Amount in input token base units" },
                        "slippage_bps": { "type": "integer", "default": 50, "description": "Slippage in bps (50 = 0.5%)" }
                    }
                }),
                returns_schema: serde_json::json!({
                    "output_amount": "integer",
                    "price_impact_pct": "number",
                    "route": "array"
                }),
            },
        ]
    }

    async fn build_transactions(
        &self,
        action: &str,
        params: &serde_json::Value,
        wallet_pubkey: &Pubkey,
        _rpc: &RpcClient,
    ) -> Result<Vec<VersionedTransaction>, PluginError> {
        match action {
            "swap" => {
                let input_mint = params["input_mint"].as_str()
                    .ok_or(PluginError::InvalidParam("input_mint".into()))?;
                let output_mint = params["output_mint"].as_str()
                    .ok_or(PluginError::InvalidParam("output_mint".into()))?;
                let amount = params["amount"].as_u64()
                    .ok_or(PluginError::InvalidParam("amount".into()))?;
                let slippage_bps = params["slippage_bps"].as_u64().unwrap_or(50) as u16;

                // 1. Get quote
                let quote = self.api_client.quote(&QuoteRequest {
                    input_mint: Pubkey::from_str(input_mint)?,
                    output_mint: Pubkey::from_str(output_mint)?,
                    amount,
                    slippage_bps,
                    ..Default::default()
                }).await?;

                // 2. Get swap transaction
                let swap_response = self.api_client.swap(&SwapRequest {
                    user_public_key: *wallet_pubkey,
                    quote_response: quote,
                    ..Default::default()
                }).await?;

                let tx = bincode::deserialize::<VersionedTransaction>(
                    &swap_response.swap_transaction
                )?;

                Ok(vec![tx])
            }
            _ => Err(PluginError::UnknownAction(action.to_string())),
        }
    }

    async fn read(&self, _action: &str, _params: &serde_json::Value, _rpc: &RpcClient)
        -> Result<serde_json::Value, PluginError> {
        Err(PluginError::NotSupported)
    }
}
```

### 7.3 Protocol Plugin Matrix

| Plugin | Actions | Type | Priority |
|--------|---------|------|---------|
| `jupiter` | `swap`, `limit_order`, `dca_create` | Transaction | P0 |
| `kamino` | `deposit`, `withdraw`, `claim_rewards`, `check_ltv`, `check_rewards` | Tx + Read | P0 |
| `marinade` | `stake`, `unstake`, `liquid_stake`, `check_rewards` | Tx + Read | P0 |
| `drift` | `open_position`, `close_position`, `check_margin`, `liquidation_guard` | Tx + Read | P0 |
| `pyth` | `read_price`, `price_deviation_alert`, `staleness_check` | Read only | P0 |
| `orca` | `add_liquidity`, `remove_liquidity`, `collect_fees`, `rebalance_range` | Transaction | P1 |
| `raydium` | `swap`, `add_liquidity`, `harvest_yield` | Transaction | P1 |
| `metaplex` | `mint_nft`, `transfer_nft`, `update_metadata` | Transaction | P2 |
| `notify.telegram` | `send_message` | Notification | P0 |
| `notify.discord` | `send_message`, `send_embed` | Notification | P0 |
| `notify.sendgrid` | `send_email` | Notification | P1 |
| `system` | `http_request`, `condition`, `foreach`, `collect`, `template` | Logic | P0 |
| `math` | `sum`, `average`, `min`, `max`, `median` | Logic | P1 |

---

## 8. REST API (Axum)

### 8.1 Server Setup

```rust
// api/src/main.rs

use axum::{Router, middleware};

#[tokio::main]
async fn main() {
    let app = Router::new()
        // Workflows
        .route("/v1/workflows",                 post(create_workflow).get(list_workflows))
        .route("/v1/workflows/:id",             get(get_workflow).patch(update_workflow).delete(delete_workflow))
        .route("/v1/workflows/:id/trigger",     post(trigger_workflow))
        // Runs
        .route("/v1/runs",                      get(list_runs))
        .route("/v1/runs/:run_id",              get(get_run))
        .route("/v1/runs/:run_id/logs",         get(stream_run_logs))   // SSE
        // Webhooks (no auth — HMAC signature validation instead)
        .route("/v1/webhooks/:workflow_id",     post(receive_webhook))
        // Analytics
        .route("/v1/analytics",                 get(get_analytics))
        // Orgs
        .route("/v1/orgs/me",                   get(get_org))
        .route("/v1/orgs/me/api_keys",          post(create_api_key).get(list_api_keys))
        .route("/v1/orgs/me/api_keys/:key_id",  delete(revoke_api_key))
        // Marketplace
        .route("/v1/hub",                       get(list_hub_workflows))
        .route("/v1/hub/:workflow_id/call",     post(call_hub_workflow))
        // Layer middleware
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(AppState::new().await);

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### 8.2 Key Request/Response Types

```rust
// api/src/types.rs

/// POST /v1/workflows
#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub trigger: TriggerConfig,
    pub steps: Vec<WorkflowStep>,
    pub fee_per_execution_usdc: Option<f64>,  // if publishing to Hub
    pub is_public: Option<bool>,
}

#[derive(Serialize)]
pub struct CreateWorkflowResponse {
    pub workflow_id: String,
    pub status: String,
    pub next_run: Option<String>,   // ISO 8601 timestamp for cron triggers
    pub onchain_pda: Option<String>,
}

/// GET /v1/runs/:run_id/logs — Server-Sent Events
#[derive(Serialize)]
pub struct RunLogEvent {
    pub event: String,              // "step_start" | "step_complete" | "run_complete" | "error"
    pub step_id: Option<String>,
    pub data: serde_json::Value,
    pub timestamp: String,
}

/// POST /v1/webhooks/:workflow_id
#[derive(Deserialize)]
pub struct WebhookPayload {
    pub trigger_data: serde_json::Value,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub run_id: String,
    pub status: String,
    pub estimated_slot: Option<u64>,
}
```

### 8.3 Authentication Middleware

```rust
// api/src/middleware/auth.rs

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for webhook endpoints (they use HMAC)
    if req.uri().path().starts_with("/v1/webhooks/") {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let org = state.db
        .get_org_by_api_key(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(org);
    Ok(next.run(req).await)
}
```

### 8.4 Full Endpoint Reference

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/v1/workflows` | Bearer | Create workflow; returns `workflow_id` + `next_run` |
| `GET` | `/v1/workflows` | Bearer | List org workflows; filter: `status`, `trigger_type` |
| `GET` | `/v1/workflows/:id` | Bearer | Get config, status, execution stats |
| `PATCH` | `/v1/workflows/:id` | Bearer | Update config or toggle `is_active` |
| `DELETE` | `/v1/workflows/:id` | Bearer | Archive; run history preserved |
| `POST` | `/v1/workflows/:id/trigger` | Bearer | Manually fire; body: param overrides |
| `GET` | `/v1/runs` | Bearer | List runs; filter: `workflow_id`, `status`, `from`, `to` |
| `GET` | `/v1/runs/:run_id` | Bearer | Full run detail: steps, signature, fee, outcome |
| `GET` | `/v1/runs/:run_id/logs` | Bearer | SSE stream of real-time step events |
| `POST` | `/v1/webhooks/:workflow_id` | HMAC | Inbound trigger; `X-SK-Signature` header required |
| `GET` | `/v1/analytics` | Bearer | Aggregate: executions, success rate, fee spend |
| `GET` | `/v1/hub` | None | Public Marketplace listings |
| `POST` | `/v1/hub/:id/call` | Bearer | Call a published workflow; pay via ExecutionVault credits |

---

## 9. MCP Server

The MCP server exposes SolanaKeeper as native tools for AI agents. Built in TypeScript using the `@modelcontextprotocol/sdk` package.

### 9.1 Tool Definitions

```typescript
// mcp-server/src/index.ts

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new Server({
    name: "solanakeeper",
    version: "1.0.0",
}, {
    capabilities: { tools: {} }
});

server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [
        {
            name: "sk.create_workflow",
            description: "Create a new SolanaKeeper automation workflow on Solana",
            inputSchema: {
                type: "object",
                required: ["name", "trigger", "steps"],
                properties: {
                    name: { type: "string" },
                    trigger: {
                        type: "object",
                        properties: {
                            type: { enum: ["cron", "account_watch", "webhook"] },
                            schedule: { type: "string" },      // for cron
                            account: { type: "string" },       // for account_watch
                            condition: { type: "object" },
                        }
                    },
                    steps: {
                        type: "array",
                        items: {
                            type: "object",
                            properties: {
                                plugin: { type: "string" },
                                action: { type: "string" },
                                params: { type: "object" },
                                condition: { type: "string" },
                            }
                        }
                    }
                }
            }
        },
        {
            name: "sk.trigger_workflow",
            description: "Manually trigger a workflow by ID, optionally overriding parameters",
            inputSchema: {
                type: "object",
                required: ["workflow_id"],
                properties: {
                    workflow_id: { type: "string" },
                    param_overrides: { type: "object" },
                }
            }
        },
        {
            name: "sk.get_run_status",
            description: "Get execution status and logs for a workflow run",
            inputSchema: {
                type: "object",
                required: ["run_id"],
                properties: { run_id: { type: "string" } }
            }
        },
        {
            name: "sk.list_workflows",
            description: "List all workflows for the authenticated org",
            inputSchema: {
                type: "object",
                properties: {
                    status: { enum: ["active", "inactive", "all"] },
                    limit: { type: "integer", default: 20 }
                }
            }
        },
        {
            name: "sk.get_balance",
            description: "Read SOL or SPL token balance for any Solana account",
            inputSchema: {
                type: "object",
                required: ["account"],
                properties: {
                    account: { type: "string" },
                    token_mint: { type: "string" },    // optional; omit for SOL
                }
            }
        },
        {
            name: "sk.call_program",
            description: "Execute any Solana program instruction via the platform wallet",
            inputSchema: {
                type: "object",
                required: ["program_id", "instruction_data"],
                properties: {
                    program_id: { type: "string" },
                    instruction_data: { type: "string" },  // base64 encoded
                    accounts: { type: "array" },
                }
            }
        },
        {
            name: "sk.publish_to_hub",
            description: "Publish a workflow to the public SolanaKeeper Marketplace",
            inputSchema: {
                type: "object",
                required: ["workflow_id", "fee_per_execution_usdc"],
                properties: {
                    workflow_id: { type: "string" },
                    fee_per_execution_usdc: { type: "number" },
                    description: { type: "string" },
                    tags: { type: "array", items: { type: "string" } },
                }
            }
        },
    ]
}));
```

---

## 10. Wallet & Key Infrastructure

### 10.1 Turnkey HSM Integration

```rust
// engine/src/wallet/turnkey.rs

use reqwest::Client;
use serde_json::json;

pub struct TurnkeyWallet {
    api_key: String,
    organization_id: String,
    wallet_id: String,
    http: Client,
    base_url: String,
}

impl TurnkeyWallet {
    /// Sign a transaction using Turnkey's secure enclave.
    /// The private key NEVER leaves the enclave.
    pub async fn sign_transaction(
        &self,
        tx: &VersionedTransaction,
    ) -> Result<VersionedTransaction> {
        let msg_bytes = bincode::serialize(&tx.message)?;
        let encoded = base64::encode(&msg_bytes);

        let response = self.http
            .post(format!("{}/public/v1/submit/sign_raw_payload", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "organizationId": self.organization_id,
                "parameters": {
                    "signWith": self.wallet_id,
                    "payload": encoded,
                    "encoding": "PAYLOAD_ENCODING_HEXADECIMAL",
                    "hashFunction": "HASH_FUNCTION_NOT_APPLICABLE"
                }
            }))
            .send()
            .await?;

        let signed: SignatureResponse = response.json().await?;
        let sig_bytes = hex::decode(&signed.signature)?;
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&sig_bytes);

        let mut signed_tx = tx.clone();
        signed_tx.signatures[0] = Signature::from(sig_array);
        Ok(signed_tx)
    }
}
```

### 10.2 Agentic Wallet (Scoped Ephemeral Signer)

```rust
// engine/src/wallet/agentic.rs

/// A scoped signer for AI agent workflows with per-execution spend limits.
pub struct AgenticWallet {
    keypair: Keypair,
    spend_limit_lamports: u64,
    spent_lamports: u64,
    expires_at: std::time::SystemTime,
}

impl AgenticWallet {
    /// Create an ephemeral signer valid for a single workflow run.
    /// Funded from org vault; unused balance returned after run.
    pub fn new_for_run(budget_lamports: u64) -> Self {
        Self {
            keypair: Keypair::new(),
            spend_limit_lamports: budget_lamports,
            spent_lamports: 0,
            expires_at: std::time::SystemTime::now()
                + std::time::Duration::from_secs(300),  // 5 min max
        }
    }

    pub fn check_spend(&self, lamports: u64) -> Result<()> {
        if self.spent_lamports + lamports > self.spend_limit_lamports {
            return Err(anyhow::anyhow!("Spend limit exceeded"));
        }
        if std::time::SystemTime::now() > self.expires_at {
            return Err(anyhow::anyhow!("Agentic wallet expired"));
        }
        Ok(())
    }
}
```

---

## 11. Database Schema

```sql
-- migrations/001_workflows.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE organizations (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name            TEXT NOT NULL,
    wallet_address  TEXT,                   -- Turnkey-managed Solana pubkey
    credits_usdc    BIGINT DEFAULT 0,       -- USDC credits in base units (6 decimals)
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id          UUID REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash        TEXT NOT NULL UNIQUE,   -- SHA-256 of raw key; never store raw
    name            TEXT,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    revoked_at      TIMESTAMPTZ
);

CREATE TABLE workflows (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id          UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    trigger_type    TEXT NOT NULL,          -- 'cron' | 'account_watch' | 'webhook'
    trigger_config  JSONB NOT NULL,
    steps           JSONB NOT NULL,         -- encrypted for Hub workflows
    is_active       BOOLEAN DEFAULT TRUE,
    is_public       BOOLEAN DEFAULT FALSE,  -- Hub listing
    onchain_pda     TEXT,                   -- WorkflowRegistry PDA address
    fee_per_exec_usdc BIGINT,              -- for Hub workflows
    execution_count BIGINT DEFAULT 0,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- migrations/002_runs.sql

CREATE TABLE workflow_runs (
    run_id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workflow_id     UUID REFERENCES workflows(id),
    org_id          UUID REFERENCES organizations(id),
    status          TEXT NOT NULL,          -- RunStatus enum values
    triggered_by    TEXT NOT NULL,          -- 'cron' | 'account_watch' | 'webhook' | 'manual' | 'mcp'
    steps_log       JSONB DEFAULT '[]',
    slot            BIGINT,
    signature       TEXT,                   -- Solana tx signature
    fee_lamports    BIGINT,
    jito_tip_lamports BIGINT,
    error_message   TEXT,
    started_at      TIMESTAMPTZ DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_runs_workflow_id ON workflow_runs(workflow_id);
CREATE INDEX idx_runs_org_id ON workflow_runs(org_id);
CREATE INDEX idx_runs_status ON workflow_runs(status);
CREATE INDEX idx_runs_started_at ON workflow_runs(started_at DESC);
```

---

## 12. CLI (`skh`)

### 12.1 Commands

```bash
# Authentication
skh auth login                          # Opens browser for API key setup
skh auth status                         # Show current auth + org info

# Workflow management
skh workflow list                        # List all workflows
skh workflow create -f workflow.json    # Create from JSON file
skh workflow deploy -f workflow.json    # Create + register onchain
skh workflow enable <id>
skh workflow disable <id>
skh workflow delete <id>

# Run management
skh run list --workflow <id>            # List recent runs
skh run status <run_id>                 # Poll run status
skh run logs <run_id>                   # Stream run logs (SSE)
skh run logs <run_id> --follow          # Follow live logs
skh run cancel <run_id>

# Direct execution (no workflow required)
skh execute transfer \
    --to <pubkey> \
    --amount 1.5 \
    --token SOL

skh execute contract-call \
    --program <program_id> \
    --instruction <base64_data>

# Billing
skh billing status                      # Show credit balance
skh billing deposit --amount 50         # Deposit 50 USDC credits

# Config
skh config set rpc_url <url>
skh config set api_key <key>
skh config list
```

---

## 13. Environment Configuration

```bash
# .env.example

# Solana Network
SOLANA_RPC_URL=https://your-dedicated-node.chainstack.com/xxx
SOLANA_WS_URL=wss://your-dedicated-node.chainstack.com/xxx
SOLANA_NETWORK=mainnet-beta                # devnet | mainnet-beta

# Geyser / Data Feeds
YELLOWSTONE_ENDPOINT=https://your-yellowstone-endpoint:10000
YELLOWSTONE_X_TOKEN=your_yellowstone_token
JITO_SHREDSTREAM_URL=https://slc.mainnet.block-engine.jito.wtf
JITO_SHREDSTREAM_AUTH=your_jito_auth

# Jito Block Engine
JITO_BLOCK_ENGINE_URL=https://slc.mainnet.block-engine.jito.wtf
JITO_AUTH_KEYPAIR_PATH=./jito_auth.json   # Block engine API keypair

# Wallet / Key Management
TURNKEY_API_KEY=your_turnkey_api_key
TURNKEY_ORG_ID=your_turnkey_org_id
TURNKEY_BASE_URL=https://api.turnkey.com

# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/solanakeeper
REDIS_URL=redis://localhost:6379

# API Server
API_PORT=8080
API_SECRET=your_jwt_secret_min_32_chars

# Anthropic (for AI Prompt Builder)
ANTHROPIC_API_KEY=your_anthropic_api_key
ANTHROPIC_MODEL=claude-sonnet-4-20250514

# Notifications
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
DISCORD_BOT_TOKEN=your_discord_bot_token
SENDGRID_API_KEY=your_sendgrid_api_key

# Onchain Program IDs (update after deploy)
WORKFLOW_REGISTRY_PROGRAM_ID=REGISTRY_PROGRAM_ID_PLACEHOLDER
EXECUTION_VAULT_PROGRAM_ID=VAULT_PROGRAM_ID_PLACEHOLDER

# IPFS (for encrypted workflow step storage)
IPFS_API_URL=https://api.pinata.cloud
IPFS_API_KEY=your_pinata_key
```

---

## 14. Testing Requirements

### 14.1 Anchor Program Tests

```typescript
// tests/anchor/workflow-registry.ts

import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";

describe("WorkflowRegistry", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.WorkflowRegistry;

    it("registers a workflow and verifies PDA state", async () => {
        const owner = provider.wallet;
        const [workflowPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("workflow"), owner.publicKey.toBytes(), Buffer.from("test-workflow")],
            program.programId
        );

        await program.methods.registerWorkflow({
            name: "test-workflow",
            triggerType: 0,
            stepsHash: Array(32).fill(0),
            stepsCid: "QmTestCID",
        }).accounts({
            workflow: workflowPda,
            owner: owner.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
        }).rpc();

        const account = await program.account.workflowAccount.fetch(workflowPda);
        assert.equal(account.name, "test-workflow");
        assert.equal(account.isActive, true);
        assert.equal(account.executionCount.toNumber(), 0);
    });

    it("rejects registration from non-owner", async () => {
        // ... test unauthorized access
    });

    it("increments execution count on record_execution", async () => {
        // ... test counter increment
    });
});
```

### 14.2 Plugin Integration Tests

```rust
// tests/integration/jupiter_plugin.rs
// Run against devnet

#[tokio::test]
async fn test_jupiter_swap_builds_valid_transaction() {
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
    let wallet = Keypair::new();
    let plugin = JupiterPlugin::new();

    let txs = plugin.build_transactions(
        "swap",
        &serde_json::json!({
            "input_mint": "So11111111111111111111111111111111111111112",  // SOL
            "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",  // USDC
            "amount": 1_000_000,    // 0.001 SOL
            "slippage_bps": 100
        }),
        &wallet.pubkey(),
        &rpc,
    ).await.unwrap();

    assert_eq!(txs.len(), 1);
    // Simulate but don't submit
    let sim_result = rpc.simulate_transaction(&txs[0]).await.unwrap();
    assert!(sim_result.err.is_none(), "Simulation failed: {:?}", sim_result.err);
}
```

### 14.3 Test Coverage Requirements

| Component | Test Type | Coverage Target |
|-----------|-----------|----------------|
| Anchor programs | Unit + integration | All instructions, happy + error paths |
| Protocol plugins | Integration (devnet) | All actions simulate successfully |
| Jito bundle builder | Unit + devnet | Bundle constructs, tips calculate |
| Geyser router | Unit | Dedup window, dual-feed fan-in |
| REST API | Integration | All endpoints, auth, error responses |
| MCP tools | Unit | All `sk.*` tools return correct schemas |
| E2E workflow | Full system (devnet) | cron→execute→log→notify |

---

## 15. Non-Negotiable Rules

These are hard constraints. **Do not deviate without explicit approval.**

1. **NEVER use public shared RPC for production execution.** All write operations go through dedicated nodes (Chainstack/Helius configured in `.env`).

2. **ALL transaction submissions MUST go through Jito bundles.** No raw `sendTransaction` for any write operation.

3. **Plugin `build_transactions()` MUST simulate before the bundle builder submits.** Zero unsimulated transactions to mainnet.

4. **Private keys MUST NOT appear in application memory.** All signing goes through the Turnkey enclave API. The only exception is the Jito auth keypair, which is only used for block engine authentication, not for holding user funds.

5. **Every workflow run MUST produce a fully structured `WorkflowRun` log entry** with: slot, signature, fee_lamports, jito_tip_lamports, outcome, all step logs, started_at, completed_at.

6. **All Anchor programs MUST use PDA authority** — no raw keypair upgrade authorities on mainnet. Use Squads Protocol multisig for upgrade authority.

7. **Geyser stream failures MUST trigger automatic failover within 5 seconds.** The router must attempt reconnection to the secondary feed and alert via internal monitoring. Never silently drop events.

8. **API keys are stored as SHA-256 hashes only.** The raw key is shown once at creation; never stored in plaintext.

9. **Webhook endpoints MUST validate `X-SK-Signature` HMAC.** Reject any webhook without valid signature.

10. **Database queries MUST use parameterised statements** via `sqlx`. No string interpolation in SQL.

---

## 16. Build Priority Order

Implement in this exact sequence. Each phase unblocks the next.

### Phase 1 — Foundation (Weeks 1–6)
- [ ] Anchor workspace setup; deploy `WorkflowRegistry` to devnet
- [ ] PostgreSQL schema + migrations (sqlx)
- [ ] Turnkey wallet integration + signing flow
- [ ] Yellowstone gRPC client + ShredStream client + DualFeedRouter
- [ ] Cron trigger engine
- [ ] Account-watch trigger (Geyser-backed)
- [ ] Jito bundle builder + TipCalculator + RetryPolicy
- [ ] Jupiter plugin (proves plugin interface)
- [ ] Axum REST API — CRUD for workflows + run log endpoints
- [ ] Basic `skh` CLI — auth, workflow deploy, run logs

### Phase 2 — Protocol Depth (Weeks 7–12)
- [ ] Kamino plugin (highest priority — largest Solana TVL)
- [ ] Marinade plugin
- [ ] Drift plugin
- [ ] Pyth price feed read plugin
- [ ] Notification plugins: Telegram, Discord, SendGrid
- [ ] Webhook trigger support
- [ ] `ExecutionVault` Anchor program + devnet deploy
- [ ] SSE streaming for run logs
- [ ] Full `skh` CLI — all commands

### Phase 3 — Intelligence Layer (Weeks 13–20)
- [ ] MCP server — all `sk.*` tools
- [ ] AI Prompt Builder (natural language → workflow JSON via Anthropic API)
- [ ] React Flow visual canvas (see Frontend Spec)
- [ ] Marketplace Hub API endpoints
- [ ] `WorkflowRegistry` onchain publishing flow
- [ ] Orca + Raydium plugins

### Phase 4 — Enterprise (Weeks 21+)
- [ ] 99.99% SLA infrastructure (multi-region, automatic failover)
- [ ] Squads multisig integration for Anchor program upgrade authority
- [ ] SSO/SAML via org management API
- [ ] Compliance audit log exports
- [ ] Custom protocol integration framework (community plugins)

---

*SolanaKeeper Backend Spec · v1.0 · Claude Code Handoff*  
*Stack: Rust · Anchor · TypeScript · PostgreSQL · Redis · Jito · Geyser gRPC*