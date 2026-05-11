import type { Node, Edge } from "reactflow";
import type { StepNodeData } from "@/components/workflow/builder/StepNode";

export interface WorkflowExample {
  id: string;
  name: string;
  description: string;
  triggerType: "cron" | "webhook" | "manual" | "price_alert" | "on_chain";
  triggerConfig: Record<string, unknown>;
  /** Step nodes laid out left-to-right at y=160. */
  steps: Array<{
    id: string;
    plugin: string;
    action: string;
    params: Record<string, unknown>;
  }>;
}

export const SIGNAL_SCOUT: WorkflowExample = {
  id: "example-signal-scout",
  name: "Signal Scout — Rebalance reasoning",
  description:
    "Every 6h (or on portfolio drift), snapshot the wallet, pull market signals, ask the LLM for target weights, emit to Trade Executor.",
  triggerType: "cron",
  triggerConfig: { schedule: "0 */6 * * *" },
  steps: [
    {
      id: "step_snapshot",
      plugin: "portfolio",
      action: "snapshot",
      params: { account: "", include_spl: true, price_in: "USDC" },
    },
    {
      id: "step_drift",
      plugin: "portfolio",
      action: "detect_drift",
      params: { holdings: "[]", targets: '{"SOL":50,"JUP":20,"USDC":30}', threshold_pct: 5 },
    },
    {
      id: "step_news",
      plugin: "news",
      action: "crypto_panic",
      params: { filter: "important", currencies: '["SOL","BTC","JUP"]', limit: 10 },
    },
    {
      id: "step_fg",
      plugin: "fear_greed",
      action: "current",
      params: {},
    },
    {
      id: "step_llm",
      plugin: "llm",
      action: "complete",
      params: {
        prompt:
          "Given the portfolio snapshot, market news, and Fear & Greed index, output target_weights, confidence (0-100), and a one-line reasoning summary as JSON.",
        system: "You are a conservative crypto portfolio manager. Always return JSON.",
        model: "claude-sonnet-4-6",
        max_tokens: 512,
        temperature: 0.2,
      },
    },
    {
      id: "step_emit",
      plugin: "solhub",
      action: "emit_webhook",
      params: {
        target_workflow_id: "",
        payload: '{"target_weights":{},"confidence":0,"reasoning":"","triggered_by":"drift"}',
      },
    },
  ],
};

export const TRADE_EXECUTOR: WorkflowExample = {
  id: "example-trade-executor",
  name: "Trade Executor — Guarded swaps",
  description:
    "Webhook-triggered: receive a target allocation, run delta + guard rails, get Jupiter quotes, require Telegram approval for large swaps, execute.",
  triggerType: "webhook",
  triggerConfig: { secret: "" },
  steps: [
    {
      id: "step_delta",
      plugin: "solhub",
      action: "delta_calc",
      params: {
        current: '{"SOL":50,"JUP":20,"USDC":30}',
        target: '{"SOL":45,"JUP":20,"USDC":35}',
        total_value_usd: 10000,
      },
    },
    {
      id: "step_guard",
      plugin: "solhub",
      action: "guard_rails",
      params: {
        swaps: "[]",
        total_value_usd: 10000,
        confidence_score: 75,
        rules: '{"max_single_swap_pct":15,"max_slippage_pct":1,"min_confidence":70}',
      },
    },
    {
      id: "step_quote",
      plugin: "jupiter",
      action: "quote",
      params: {
        input_mint: "So11111111111111111111111111111111111111112",
        output_mint: "EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v",
        amount: 1_000_000_000,
        slippage_bps: 50,
      },
    },
    {
      id: "step_approval",
      plugin: "solhub",
      action: "require_approval",
      params: {
        message: "Approve rebalance: confirm proposed swaps before execution.",
        timeout_secs: 600,
      },
    },
    {
      id: "step_telegram",
      plugin: "notify.telegram",
      action: "send_message",
      params: { chat_id: "", text: "Rebalance proposal — see workflow run for details" },
    },
    {
      id: "step_swap",
      plugin: "jupiter",
      action: "swap",
      params: {
        input_mint: "So11111111111111111111111111111111111111112",
        output_mint: "EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v",
        amount: 1_000_000_000,
        slippage_bps: 50,
      },
    },
  ],
};

export const EXAMPLES: WorkflowExample[] = [SIGNAL_SCOUT, TRADE_EXECUTOR];

export function exampleToGraph(ex: WorkflowExample): {
  name: string;
  nodes: Node<StepNodeData>[];
  edges: Edge[];
  params: Record<string, Record<string, unknown>>;
} {
  const nodes: Node<StepNodeData>[] = ex.steps.map((s, i) => ({
    id: s.id,
    type: "step",
    position: { x: 80 + i * 240, y: 160 },
    data: { label: `${s.plugin}.${s.action}`, plugin: s.plugin, action: s.action },
  }));
  const edges: Edge[] = [];
  for (let i = 1; i < ex.steps.length; i++) {
    edges.push({
      id: `${ex.steps[i - 1]!.id}->${ex.steps[i]!.id}`,
      source: ex.steps[i - 1]!.id,
      target: ex.steps[i]!.id,
      animated: true,
    });
  }
  const params: Record<string, Record<string, unknown>> = {};
  for (const s of ex.steps) params[s.id] = s.params;
  return { name: ex.name, nodes, edges, params };
}
