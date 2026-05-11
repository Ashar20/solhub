"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { Topbar } from "@/components/shell/Topbar";
import { Btn } from "@/components/primitives/Btn";
import { Pill } from "@/components/primitives/Pill";
import { Icon } from "@/components/primitives/Icon";

const EXAMPLES = [
  "Every 6 hours, snapshot my Solana portfolio, check the Crypto Fear & Greed Index, and ask the LLM whether I should rebalance. Emit the recommendation to my Trade Executor workflow.",
  "When the SOL price drops below $140 via Pyth, fetch crypto news headlines and ask the LLM whether the dip looks oversold. If yes, alert me on Telegram.",
  "Every hour, fetch my Kamino vault status and Drift margin health, summarize them, and post to my Discord.",
];

const SEED_KEY = "solhub.draft.new";

interface DraftSeed {
  name: string;
  nodes: unknown[];
  edges: unknown[];
  params: Record<string, Record<string, unknown>>;
  updatedAt: string;
}

function buildSeed(prompt: string): DraftSeed {
  const id = "step_llm_seed";
  return {
    name: "AI-built workflow",
    nodes: [
      {
        id,
        type: "step",
        position: { x: 80, y: 160 },
        data: { label: "llm.complete", plugin: "llm", action: "complete" },
      },
    ],
    edges: [],
    params: {
      [id]: {
        prompt,
        system: "Respond with structured JSON.",
        model: "claude-sonnet-4-6",
        max_tokens: 512,
        temperature: 0.2,
      },
    },
    updatedAt: new Date().toISOString(),
  };
}

export default function AiBuilderPage() {
  const [prompt, setPrompt] = useState("");
  const router = useRouter();

  function openInBuilder() {
    if (!prompt.trim()) return;
    window.localStorage.setItem(SEED_KEY, JSON.stringify(buildSeed(prompt.trim())));
    router.push("/workflows/new");
  }

  return (
    <>
      <Topbar crumbs={["Workspace", "solhub-prod", "AI Builder"]} />
      <main className="flex-1 p-6 overflow-y-auto grid grid-cols-[1fr_360px] gap-6">
        <section>
          <h1 className="text-[22px] font-semibold tracking-tight mb-1">Describe a workflow</h1>
          <p className="text-[13px] text-ink-500 mb-4">
            Sketch what you want in plain English. We&apos;ll seed the builder with an{" "}
            <Pill tone="violet">llm.complete</Pill> node containing your prompt — refine the rest
            visually.
          </p>
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Every 6 hours, check Kamino LTV. If above 0.7, withdraw 10% collateral and swap to USDC."
            rows={8}
            className="w-full rounded-lg border border-ink-200 p-3 text-[13px] focus:outline-none focus:ring-2 focus:ring-violet-500/30 font-mono leading-relaxed"
          />
          <div className="mt-2">
            <Btn
              variant="primary"
              onClick={openInBuilder}
              disabled={prompt.trim() === ""}
              icon={<Icon name="arrow" className="w-3.5 h-3.5" />}
            >
              Open in builder
            </Btn>
          </div>
          <p className="text-[11px] text-ink-500 mt-3 leading-relaxed">
            Note: dedicated AI scaffolding (auto-generating multi-step workflows from the prompt) is
            on the roadmap. For now the prompt is seeded into a single LLM node so you can extend it
            with other plugin steps.
          </p>
        </section>
        <aside>
          <div className="text-[11px] uppercase tracking-wider font-mono text-ink-500 mb-2">
            Examples
          </div>
          <ul className="space-y-1.5">
            {EXAMPLES.map((ex) => (
              <li key={ex}>
                <button
                  type="button"
                  onClick={() => setPrompt(ex)}
                  className="w-full text-left text-[12px] p-3 rounded-lg border border-ink-200 hover:bg-ink-50 leading-relaxed"
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
