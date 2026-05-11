import { z } from "zod";
import type { PluginDef } from "./types";

/** Preprocess helper: parse JSON strings into the inner type for array/object params. */
function jsonParam<T extends z.ZodTypeAny>(inner: T) {
  return z.preprocess(
    (v) =>
      typeof v === "string"
        ? (() => { try { return JSON.parse(v); } catch { return v; } })()
        : v,
    inner,
  );
}

export const REGISTRY: PluginDef[] = [
  // ──────────────────────────────────────────────────────────────────
  // REAL implementations
  // ──────────────────────────────────────────────────────────────────
  {
    id: "jupiter",
    name: "Jupiter",
    category: "swap",
    status: "real",
    actions: [
      {
        id: "swap",
        name: "Swap Tokens",
        description: "Best-route token swap via Jupiter aggregator.",
        type: "transaction",
        schema: z.object({
          input_mint: z.string().min(32),
          output_mint: z.string().min(32),
          amount: z.coerce.number().int().positive(),
          slippage_bps: z.coerce.number().int().min(0).max(10_000).default(50),
        }),
        defaults: { input_mint: "", output_mint: "", amount: 1_000_000, slippage_bps: 50 },
      },
      {
        id: "quote",
        name: "Get Swap Quote",
        description: "Get a Jupiter best-route price quote (no transaction).",
        type: "read",
        schema: z.object({
          input_mint: z.string().min(32),
          output_mint: z.string().min(32),
          amount: z.coerce.number().int().positive(),
          slippage_bps: z.coerce.number().int().min(0).max(10_000).default(50),
        }),
        defaults: { input_mint: "", output_mint: "", amount: 1_000_000, slippage_bps: 50 },
      },
      {
        id: "price",
        name: "Token Prices",
        description: "Fetch USD prices for one or more token mints from Jupiter Price API v3.",
        type: "read",
        schema: z.object({
          ids: jsonParam(z.array(z.string()).min(1)),
        }),
        defaults: { ids: '["So11111111111111111111111111111111111111112"]' },
      },
    ],
  },
  {
    id: "pyth",
    name: "Pyth",
    category: "oracle",
    status: "real",
    actions: [
      {
        id: "read_price",
        name: "Read Price",
        description: "Read current price from a Pyth price feed account.",
        type: "read",
        schema: z.object({
          feed: z.string().min(1),
        }),
        defaults: { feed: "" },
      },
      {
        id: "staleness_check",
        name: "Staleness Check",
        description: "Check if a Pyth price feed is stale.",
        type: "read",
        schema: z.object({
          feed: z.string().min(1),
          max_age_seconds: z.coerce.number().int().positive(),
        }),
        defaults: { feed: "", max_age_seconds: 60 },
      },
    ],
  },
  {
    id: "system",
    name: "System",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "transfer",
        name: "SOL Transfer",
        description: "Transfer native SOL between accounts.",
        type: "transaction",
        schema: z.object({
          to: z.string().min(32),
          lamports: z.coerce.number().int().positive(),
        }),
        defaults: { to: "", lamports: 1_000_000 },
      },
      {
        id: "memo",
        name: "Memo",
        description: "Attach an SPL Memo to a transaction.",
        type: "transaction",
        schema: z.object({
          text: z.string().min(1),
        }),
        defaults: { text: "" },
      },
      {
        id: "get_balance",
        name: "Get Balance",
        description: "Read the SOL balance of any account.",
        type: "read",
        schema: z.object({
          account: z.string().min(32),
        }),
        defaults: { account: "" },
      },
      {
        id: "batch_transfer",
        name: "Batch SOL Transfer",
        description: "Atomically transfer SOL to multiple recipients in one transaction. Params: transfers (JSON array of {to, lamports}).",
        type: "transaction",
        // ZodForm does not yet render nested arrays; this field renders as a
        // plain text input and the JSON is parsed at submit time. UX follow-up: #TODO
        schema: z.object({
          transfers: z.preprocess(
            (v) => {
              if (typeof v === "string") {
                try { return JSON.parse(v); } catch { return v; }
              }
              return v;
            },
            z.array(z.object({ to: z.string(), lamports: z.coerce.number().int().positive() })).min(1).max(15),
          ),
        }),
        defaults: { transfers: '[{"to":"","lamports":1000000}]' },
      },
    ],
  },
  {
    id: "notify.telegram",
    name: "Telegram",
    category: "notify",
    status: "real",
    actions: [
      {
        id: "send_message",
        name: "Send Message",
        description: "Send a Telegram message via bot.",
        type: "notification",
        schema: z.object({
          chat_id: z.string().min(1),
          text: z.string().min(1),
          bot_token: z.string().optional(),
        }),
        defaults: { chat_id: "", text: "" },
      },
    ],
  },
  {
    id: "notify.discord",
    name: "Discord",
    category: "notify",
    status: "real",
    actions: [
      {
        id: "send_message",
        name: "Send Message",
        description: "Send a plain text message to a Discord webhook.",
        type: "notification",
        schema: z.object({
          webhook_url: z.string().url(),
          content: z.string().min(1),
        }),
        defaults: { webhook_url: "", content: "" },
      },
      {
        id: "send_embed",
        name: "Send Embed",
        description: "Send a rich embed message to a Discord webhook.",
        type: "notification",
        schema: z.object({
          webhook_url: z.string().url(),
          title: z.string().min(1),
          description: z.string().min(1),
          color: z.coerce.number().int().optional(),
        }),
        defaults: { webhook_url: "", title: "", description: "" },
      },
    ],
  },
  {
    id: "llm",
    name: "LLM",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "complete",
        name: "Chat Completion",
        description:
          "Send a prompt to the LLM. With json_mode=true the engine parses the reply and adds a \"json\" field for downstream steps (OpenAI: JSON mode; Anthropic: prompt + parse).",
        type: "read",
        schema: z.object({
          prompt: z.string().min(1),
          system: z.string().optional(),
          model: z.string().optional(),
          max_tokens: z.coerce.number().int().positive().default(512),
          temperature: z.coerce.number().min(0).max(2).default(0.2),
          json_mode: z.coerce.boolean().optional().default(false),
          provider: z.enum(["openai", "anthropic"]).optional(),
        }),
        defaults: {
          prompt: "",
          system: "",
          model: "",
          max_tokens: 512,
          temperature: 0.2,
          json_mode: false,
        },
      },
      {
        id: "analyze_sentiment",
        name: "Sentiment Analysis",
        description: "Analyze sentiment of a text input (returns positive, neutral, or negative with score).",
        type: "read",
        schema: z.object({
          text: z.string().min(1),
          context: z.string().optional(),
        }),
        defaults: { text: "", context: "" },
      },
    ],
  },
  {
    id: "news",
    name: "News",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "fetch_headlines",
        name: "Fetch Crypto Headlines",
        description: "Fetch the latest crypto news headlines from the configured RSS source.",
        type: "read",
        schema: z.object({
          limit: z.coerce.number().int().positive().default(5),
          feed_url: z.string().url().optional(),
        }),
        defaults: { limit: 5, feed_url: "" },
      },
      {
        id: "fetch_url",
        name: "Fetch URL Body",
        description: "Fetch the body of an arbitrary URL (text).",
        type: "read",
        schema: z.object({
          url: z.string().url(),
          max_bytes: z.coerce.number().int().positive().default(65536),
        }),
        defaults: { url: "", max_bytes: 65536 },
      },
      {
        id: "crypto_panic",
        name: "CryptoPanic News",
        description: "Fetch crypto news posts from CryptoPanic.",
        type: "read",
        schema: z.object({
          filter: z.enum(["rising", "hot", "bullish", "bearish", "important"]).optional(),
          currencies: jsonParam(z.array(z.string())).optional(),
          limit: z.coerce.number().int().positive().default(10),
        }),
        defaults: { filter: "important", currencies: '["SOL","BTC"]', limit: 10 },
      },
    ],
  },
  {
    id: "solhub",
    name: "SolHub",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "run_workflow",
        name: "Run Sub-Workflow",
        description: "Trigger another workflow and wait for its terminal state. Max depth 3.",
        type: "read",
        schema: z.object({
          workflow_id: z.string().uuid(),
          timeout_secs: z.coerce.number().int().positive().default(60),
        }),
        defaults: { workflow_id: "", timeout_secs: 60 },
      },
      {
        id: "delta_calc",
        name: "Portfolio Delta Calculator",
        description: "Compute rebalancing swaps from current to target weights.",
        type: "read",
        schema: z.object({
          current: jsonParam(z.record(z.unknown())),
          target: jsonParam(z.record(z.unknown())),
          total_value_usd: z.coerce.number().positive(),
        }),
        defaults: {
          current: '{"SOL":50,"USDC":50}',
          target: '{"SOL":45,"USDC":55}',
          total_value_usd: 10000,
        },
      },
      {
        id: "guard_rails",
        name: "Guard Rails",
        description: "Validate proposed swaps against safety rules before execution.",
        type: "read",
        schema: z.object({
          swaps: jsonParam(z.array(z.unknown())),
          total_value_usd: z.coerce.number().positive(),
          confidence_score: z.coerce.number().min(0).max(100),
          quotes: jsonParam(z.array(z.unknown())).optional(),
          rules: jsonParam(z.record(z.unknown())).optional(),
        }),
        defaults: {
          swaps: "[]",
          total_value_usd: 10000,
          confidence_score: 75,
          quotes: "[]",
          rules: '{"max_single_swap_pct":15,"max_slippage_pct":1,"min_confidence":0.6}',
        },
      },
      {
        id: "emit_webhook",
        name: "Emit Webhook",
        description: "POST a payload to another workflow's webhook endpoint.",
        type: "read",
        schema: z.object({
          target_workflow_id: z.string().uuid(),
          payload: jsonParam(z.record(z.unknown())),
          secret: z.string().optional(),
          base_url: z.string().optional(),
        }),
        defaults: { target_workflow_id: "", payload: '{}', secret: "", base_url: "" },
      },
      {
        id: "require_approval",
        name: "Human-In-The-Loop Approval",
        description: "Pauses the workflow run, awaits approval via POST /v1/runs/:id/approve.",
        type: "read",
        schema: z.object({
          message: z.string().optional(),
          timeout_secs: z.coerce.number().int().nonnegative().optional(),
        }),
        defaults: { message: "Please review before proceeding.", timeout_secs: 600 },
      },
    ],
  },
  {
    id: "portfolio",
    name: "Portfolio",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "snapshot",
        name: "Portfolio Snapshot",
        description: "Fetch SOL + SPL token balances with USD values.",
        type: "read",
        schema: z.object({
          account: z.string().min(32),
          include_spl: z.coerce.boolean().default(true),
          price_in: z.string().default("USDC"),
        }),
        defaults: { account: "", include_spl: true, price_in: "USDC" },
      },
      {
        id: "compute_weights",
        name: "Compute Portfolio Weights",
        description: "Compute current vs target allocation weights.",
        type: "read",
        schema: z.object({
          holdings: jsonParam(z.array(z.unknown())),
          targets: jsonParam(z.record(z.unknown())),
        }),
        defaults: { holdings: "[]", targets: '{"SOL":50,"USDC":50}' },
      },
      {
        id: "detect_drift",
        name: "Detect Drift",
        description: "Detect whether any holding has drifted beyond a threshold.",
        type: "read",
        schema: z.object({
          holdings: jsonParam(z.array(z.unknown())),
          targets: jsonParam(z.record(z.unknown())),
          threshold_pct: z.coerce.number().positive().default(5.0),
        }),
        defaults: { holdings: "[]", targets: '{"SOL":50,"USDC":50}', threshold_pct: 5 },
      },
    ],
  },
  {
    id: "fear_greed",
    name: "Fear & Greed Index",
    category: "logic",
    status: "real",
    actions: [
      {
        id: "current",
        name: "Current Fear & Greed",
        description: "Fetch the current crypto Fear & Greed Index (0–100) from alternative.me.",
        type: "read",
        schema: z.object({}),
        defaults: {},
      },
      {
        id: "history",
        name: "Fear & Greed History",
        description: "Fetch historical Fear & Greed Index values.",
        type: "read",
        schema: z.object({
          limit: z.coerce.number().int().positive().default(30),
        }),
        defaults: { limit: 30 },
      },
    ],
  },
  // ──────────────────────────────────────────────────────────────────
  // STUB implementations (engine returns NotImplemented)
  // ──────────────────────────────────────────────────────────────────
  {
    id: "kamino",
    name: "Kamino",
    category: "lend",
    status: "stub",
    actions: [
      {
        id: "deposit",
        name: "Deposit",
        description: "Deposit assets into a Kamino reserve.",
        type: "transaction",
        schema: z.object({
          reserve: z.string().min(1),
          amount: z.coerce.number().int().positive(),
        }),
        defaults: { reserve: "", amount: 1_000_000 },
      },
      {
        id: "withdraw",
        name: "Withdraw",
        description: "Withdraw assets from a Kamino reserve.",
        type: "transaction",
        schema: z.object({
          reserve: z.string().min(1),
          amount: z.coerce.number().int().positive(),
        }),
        defaults: { reserve: "", amount: 1_000_000 },
      },
      {
        id: "claim_rewards",
        name: "Claim Rewards",
        description: "Harvest accrued Kamino rewards.",
        type: "transaction",
        schema: z.object({
          reserve: z.string().min(1),
        }),
        defaults: { reserve: "" },
      },
      {
        id: "check_ltv",
        name: "Check LTV",
        description: "Read current loan-to-value ratio for a position.",
        type: "read",
        schema: z.object({
          obligation: z.string().min(1),
        }),
        defaults: { obligation: "" },
      },
      {
        id: "check_rewards",
        name: "Check Rewards",
        description: "Read pending rewards for a position.",
        type: "read",
        schema: z.object({
          reserve: z.string().min(1),
        }),
        defaults: { reserve: "" },
      },
    ],
  },
  {
    id: "marinade",
    name: "Marinade",
    category: "stake",
    status: "stub",
    actions: [
      {
        id: "stake",
        name: "Stake SOL",
        description: "Stake SOL via Marinade native staking.",
        type: "transaction",
        schema: z.object({
          amount: z.coerce.number().int().positive(),
        }),
        defaults: { amount: 1_000_000_000 },
      },
      {
        id: "liquid_stake",
        name: "Liquid Stake SOL",
        description: "Stake SOL and receive mSOL immediately.",
        type: "transaction",
        schema: z.object({
          amount: z.coerce.number().int().positive(),
        }),
        defaults: { amount: 1_000_000_000 },
      },
      {
        id: "unstake",
        name: "Unstake SOL",
        description: "Begin delayed unstake of SOL from Marinade.",
        type: "transaction",
        schema: z.object({
          msol_amount: z.coerce.number().int().positive(),
        }),
        defaults: { msol_amount: 1_000_000_000 },
      },
      {
        id: "check_rewards",
        name: "Check Staking Rewards",
        description: "Read pending staking rewards for a stake account.",
        type: "read",
        schema: z.object({
          stake_account: z.string().min(1),
        }),
        defaults: { stake_account: "" },
      },
    ],
  },
  {
    id: "drift",
    name: "Drift",
    category: "perps",
    status: "stub",
    actions: [
      {
        id: "open_position",
        name: "Open Position",
        description: "Open a perpetual position on Drift Protocol.",
        type: "transaction",
        schema: z.object({
          market_index: z.coerce.number().int().nonnegative(),
          direction: z.enum(["long", "short"]),
          base_asset_amount: z.coerce.number().int().positive(),
          price: z.coerce.number().int().nonnegative().optional(),
        }),
        defaults: { market_index: 0, direction: "long", base_asset_amount: 100_000_000 },
      },
      {
        id: "close_position",
        name: "Close Position",
        description: "Close an open perpetual position on Drift.",
        type: "transaction",
        schema: z.object({
          market_index: z.coerce.number().int().nonnegative(),
          price: z.coerce.number().int().nonnegative().optional(),
        }),
        defaults: { market_index: 0 },
      },
      {
        id: "check_margin",
        name: "Check Margin",
        description: "Read current margin ratio for a Drift sub-account.",
        type: "read",
        schema: z.object({
          sub_account_id: z.coerce.number().int().nonnegative(),
        }),
        defaults: { sub_account_id: 0 },
      },
      {
        id: "liquidation_guard",
        name: "Liquidation Guard",
        description: "Check liquidation risk and return warning if near threshold.",
        type: "read",
        schema: z.object({
          sub_account_id: z.coerce.number().int().nonnegative(),
          warning_threshold: z.coerce.number().min(0).max(1).default(0.1),
        }),
        defaults: { sub_account_id: 0, warning_threshold: 0.1 },
      },
    ],
  },
];

export function findAction(
  pluginId: string,
  actionId: string,
): { plugin: PluginDef; action: import("./types").PluginAction } | null {
  const p = REGISTRY.find((x) => x.id === pluginId);
  if (!p) return null;
  const a = p.actions.find((x) => x.id === actionId);
  return a ? { plugin: p, action: a } : null;
}

// re-export the types so consumers can `import type { PluginAction } from "@/lib/plugins/registry"`
export type { PluginDef, PluginAction } from "./types";
