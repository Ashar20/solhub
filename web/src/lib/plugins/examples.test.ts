import { describe, it, expect } from "vitest";
import { SIGNAL_SCOUT, TRADE_EXECUTOR, exampleToGraph } from "./examples";
import { findAction } from "./registry";

describe("WorkflowExample", () => {
  it("Signal Scout steps all resolve to known plugin/actions", () => {
    for (const s of SIGNAL_SCOUT.steps) {
      expect(
        findAction(s.plugin, s.action),
        `${s.plugin}.${s.action} not found`,
      ).not.toBe(null);
    }
  });
  it("Trade Executor steps all resolve to known plugin/actions", () => {
    for (const s of TRADE_EXECUTOR.steps) {
      expect(
        findAction(s.plugin, s.action),
        `${s.plugin}.${s.action} not found`,
      ).not.toBe(null);
    }
  });
  it("exampleToGraph produces linear edges between sequential steps", () => {
    const g = exampleToGraph(SIGNAL_SCOUT);
    expect(g.nodes).toHaveLength(SIGNAL_SCOUT.steps.length);
    expect(g.edges).toHaveLength(SIGNAL_SCOUT.steps.length - 1);
  });
  it("exampleToGraph params map contains each step id", () => {
    const g = exampleToGraph(TRADE_EXECUTOR);
    for (const s of TRADE_EXECUTOR.steps) {
      expect(g.params[s.id]).toBeDefined();
    }
  });
  it("exampleToGraph nodes have correct plugin/action in data", () => {
    const g = exampleToGraph(SIGNAL_SCOUT);
    for (let i = 0; i < SIGNAL_SCOUT.steps.length; i++) {
      const s = SIGNAL_SCOUT.steps[i]!;
      const n = g.nodes[i]!;
      expect(n.data.plugin).toBe(s.plugin);
      expect(n.data.action).toBe(s.action);
    }
  });
  it("edges connect sequential steps with animated=true", () => {
    const g = exampleToGraph(TRADE_EXECUTOR);
    for (let i = 0; i < g.edges.length; i++) {
      expect(g.edges[i]!.source).toBe(TRADE_EXECUTOR.steps[i]!.id);
      expect(g.edges[i]!.target).toBe(TRADE_EXECUTOR.steps[i + 1]!.id);
      expect(g.edges[i]!.animated).toBe(true);
    }
  });
});
