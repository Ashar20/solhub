import { z } from "zod";
import type { PluginDef } from "./types";

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
