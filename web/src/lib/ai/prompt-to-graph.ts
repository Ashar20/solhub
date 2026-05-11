import type { Edge, Node } from "reactflow";
import { findAction } from "@/lib/plugins/registry";
import type { StepNodeData } from "@/components/workflow/builder/StepNode";

type BareStep = { plugin: string; action: string; params: Record<string, unknown> };

function mergeDefaults(plugin: string, action: string, params: Record<string, unknown>): Record<string, unknown> {
  const found = findAction(plugin, action);
  const defaults = (found?.action.defaults ?? {}) as Record<string, unknown>;
  return { ...defaults, ...params };
}

/**
 * Infer a left-to-right workflow from plain English. Matches the SolHub plugin catalog;
 * steps are linked in execution order with animated edges so the builder shows a connected graph.
 */
export function inferStepsFromPrompt(prompt: string): BareStep[] {
  const p = prompt.toLowerCase();
  const out: BareStep[] = [];

  const push = (plugin: string, action: string, params: Record<string, unknown> = {}) => {
    out.push({ plugin, action, params });
  };

  // ── Data & market signals (typical “scout” front half) ─────────────────
  if (/portfolio|wallet|snapshot|holding|rebalanc|allocat/.test(p)) {
    push("portfolio", "snapshot", { account: "", include_spl: true, price_in: "USDC" });
  }
  if (/drift|deviat|threshold|%\s*from\s*target|off\s*target/.test(p)) {
    push("portfolio", "detect_drift", {
      holdings: "[]",
      targets: '{"SOL":40,"USDC":35,"JUP":25}',
      threshold_pct: 5,
    });
  }
  if (/fear|greed|\bfng\b/.test(p)) {
    push("fear_greed", "current", {});
  }
  if (/cryptopanic|crypto\s*panic/.test(p)) {
    push("news", "crypto_panic", {
      filter: "important",
      currencies: '["SOL","BTC","JUP"]',
      limit: 10,
    });
  } else if (/news|headline|article|oversold|coindesk|dip\b/.test(p)) {
    push("news", "fetch_headlines", { limit: 8 });
  }
  if (/pyth|oracle|price\s*feed/.test(p) || /\$\d/.test(prompt)) {
    push("pyth", "read_price", { feed: "" });
  }
  if (/\bjupiter\b.*\bprice\b|\btoken\s+prices?\b/.test(p)) {
    push("jupiter", "price", {
      ids: '["So11111111111111111111111111111111111111112"]',
    });
  }

  // ── DeFi stubs (still useful in the builder) ──────────────────────────
  if (/kamino/.test(p)) {
    push("kamino", "check_ltv", { obligation: "" });
  }
  if (/\bdrift\b|margin\s*health/.test(p)) {
    push("drift", "check_margin", { sub_account_id: 0 });
  }
  if (/marinade|msol|liquid\s*stake/.test(p)) {
    push("marinade", "check_rewards", { stake_account: "" });
  }

  const wantsLlm =
    /\b(llm|ask|summar|recommend|reason|whether|should|analyz|decide|gpt|claude|model)\b/.test(p) ||
    prompt.trim().length > 100;

  // ── Downstream / execution-ish (after data; LLM inserted later) ───────
  if (/emit\b|trade\s*executor|another\s*workflow|webhook.*workflow|downstream/.test(p)) {
    push("solhub", "emit_webhook", {
      target_workflow_id: "",
      payload: JSON.stringify({
        note: "Fill payload from earlier steps when step templating is available.",
        source_prompt: prompt.slice(0, 280),
      }),
    });
  }

  if (/delta\b|target\s*weight|current\s*weight|allocat.*swap/.test(p)) {
    push("solhub", "delta_calc", {
      current: '{"SOL":50,"JUP":20,"USDC":30}',
      target: '{"SOL":45,"JUP":20,"USDC":35}',
      total_value_usd: 10000,
    });
  }

  if (/\bquote\b|best\s*route|price\s*impact|rout(e|ing)/.test(p)) {
    push("jupiter", "quote", {
      input_mint: "So11111111111111111111111111111111111111112",
      output_mint: "EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v",
      amount: 1_000_000_000,
      slippage_bps: 50,
    });
  }

  if (/guard\s*rail|slippage\s*limit|max\s*swap|risk\s*limit/.test(p)) {
    push("solhub", "guard_rails", {});
  }

  if (/approve\b|human|manual\s+review|confirm\s+before/.test(p)) {
    push("solhub", "require_approval", {
      message: "Review automated actions before continuing.",
      timeout_secs: 600,
    });
  }

  if (/\bswap\b|exchange.*(sol|token)|sell\s+sol|buy\s+(sol|usdc)/.test(p)) {
    push("jupiter", "swap", {
      input_mint: "So11111111111111111111111111111111111111112",
      output_mint: "EPjFWdd5AufqSSqeM2qN1XzybapC8G4wEGGkZwyTDt1v",
      amount: 1_000_000_000,
      slippage_bps: 50,
    });
  }

  if (/telegram/.test(p)) {
    push("notify.telegram", "send_message", {
      chat_id: "",
      text: "SolHub workflow finished — see dashboard for details.",
    });
  }
  if (/discord/.test(p)) {
    push("notify.discord", "send_message", {
      webhook_url: "",
      content: "SolHub workflow finished — see dashboard for details.",
    });
  }

  const llmStep: BareStep = {
    plugin: "llm",
    action: "complete",
    params: {
      prompt,
      system: [
        "You help debug SolHub workflow execution.",
        "Output ONE JSON object with keys:",
        '"summary" (string), "likely_causes" (string[]), "steps_to_verify" (string[]),',
        '"suggested_param_fixes" (object, optional).',
        "Be concrete about plugins, actions, and params.",
      ].join(" "),
      model: "claude-sonnet-4-6",
      max_tokens: 1024,
      temperature: 0.2,
      json_mode: true,
      provider: "anthropic",
    },
  };

  const alreadyHasLlm = out.some((s) => s.plugin === "llm");
  if (alreadyHasLlm) {
    const i = out.findIndex((s) => s.plugin === "llm");
    if (i >= 0) out[i] = { ...out[i]!, params: { ...out[i]!.params, prompt } };
    return out;
  }

  if (out.length === 0) {
    out.push(llmStep);
    return out;
  }

  const onlyNotify =
    out.length > 0 && out.every((s) => s.plugin.startsWith("notify."));
  const shouldAddLlm = wantsLlm || out.length >= 2 || (onlyNotify && prompt.trim().length > 80);
  if (!shouldAddLlm) return out;

  const notifyOrEmitIdx = out.findIndex(
    (s) =>
      s.plugin.startsWith("notify.") ||
      (s.plugin === "solhub" && s.action === "emit_webhook"),
  );
  const swapIdx = out.findIndex((s) => s.plugin === "jupiter" && s.action === "swap");
  const boundIdxs = [notifyOrEmitIdx, swapIdx].filter((i) => i >= 0);
  const insertAt = boundIdxs.length > 0 ? Math.min(...boundIdxs) : out.length;
  out.splice(insertAt, 0, llmStep);

  return out;
}

export function buildAiBuilderSeed(prompt: string): {
  name: string;
  nodes: Node<StepNodeData>[];
  edges: Edge[];
  params: Record<string, Record<string, unknown>>;
} {
  const trimmed = prompt.trim();
  const raw = inferStepsFromPrompt(trimmed);
  const withIds = raw.map((s, i) => ({
    id: `step_ai_${i + 1}`,
    plugin: s.plugin,
    action: s.action,
    params: mergeDefaults(s.plugin, s.action, s.params),
  }));

  const nodes: Node<StepNodeData>[] = withIds.map((s, i) => ({
    id: s.id,
    type: "step",
    position: { x: 80 + i * 240, y: 160 },
    data: { label: `${s.plugin}.${s.action}`, plugin: s.plugin, action: s.action },
  }));

  const edges: Edge[] = [];
  for (let i = 1; i < withIds.length; i++) {
    edges.push({
      id: `${withIds[i - 1]!.id}->${withIds[i]!.id}`,
      source: withIds[i - 1]!.id,
      target: withIds[i]!.id,
      animated: true,
    });
  }

  const params: Record<string, Record<string, unknown>> = {};
  for (const s of withIds) params[s.id] = s.params;

  return {
    name: `AI draft — ${trimmed.slice(0, 40)}${trimmed.length > 40 ? "…" : ""}`,
    nodes,
    edges,
    params,
  };
}
