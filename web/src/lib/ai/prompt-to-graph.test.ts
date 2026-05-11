import { describe, expect, it } from "vitest";
import { buildAiBuilderSeed, inferStepsFromPrompt } from "./prompt-to-graph";

describe("buildAiBuilderSeed", () => {
  it("connects multiple inferred steps with edges", () => {
    const prompt =
      "Every 6 hours, snapshot my Solana portfolio, check Fear & Greed, and ask the LLM what to do. Emit to Trade Executor.";
    const g = buildAiBuilderSeed(prompt);
    expect(g.nodes.length).toBeGreaterThanOrEqual(3);
    expect(g.edges.length).toBe(g.nodes.length - 1);
    for (let i = 1; i < g.nodes.length; i++) {
      expect(g.edges[i - 1]!.source).toBe(g.nodes[i - 1]!.id);
      expect(g.edges[i - 1]!.target).toBe(g.nodes[i]!.id);
    }
  });

  it("places LLM before emit_webhook when both match", () => {
    const g = buildAiBuilderSeed(
      "Snapshot portfolio, fear greed, LLM decide, emit webhook downstream.",
    );
    const plugins = g.nodes.map((n) => n.data.plugin);
    const emitIdx = plugins.lastIndexOf("solhub");
    const llmIdx = plugins.indexOf("llm");
    expect(llmIdx).toBeGreaterThanOrEqual(0);
    expect(emitIdx).toBeGreaterThan(llmIdx);
  });

  it("falls back to a single LLM node for vague prompts", () => {
    const g = buildAiBuilderSeed("hello");
    expect(g.nodes).toHaveLength(1);
    expect(g.edges).toHaveLength(0);
    expect(g.nodes[0]!.data.plugin).toBe("llm");
  });
});

describe("inferStepsFromPrompt", () => {
  it("does not force LLM for a short telegram-only line", () => {
    const steps = inferStepsFromPrompt("ping telegram");
    expect(steps.every((s) => s.plugin === "notify.telegram")).toBe(true);
  });
});
