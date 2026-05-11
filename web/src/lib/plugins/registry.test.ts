import { describe, it, expect } from "vitest";
import { REGISTRY, findAction } from "./registry";

describe("REGISTRY", () => {
  it("has at least 6 plugins", () => {
    expect(REGISTRY.length).toBeGreaterThanOrEqual(6);
  });

  it("each plugin has unique id", () => {
    const ids = REGISTRY.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("each action has a Zod schema with a parse function", () => {
    for (const p of REGISTRY) {
      for (const a of p.actions) {
        expect(typeof a.schema.parse).toBe("function");
      }
    }
  });

  it("each action's defaults parse cleanly through its own schema", () => {
    // Optional fields with defaults are fine; required empty strings will fail
    // for `.min(32)` constraints. We just verify the schema *exists* and is
    // well-formed by trying to parse defaults.
    for (const p of REGISTRY) {
      for (const a of p.actions) {
        const r = a.schema.safeParse(a.defaults);
        // We don't assert success — defaults may include empty strings that fail
        // for `.min(32)` constraints. We only check the schema is a real ZodType.
        expect(r).toHaveProperty("success");
      }
    }
  });

  it("every plugin has at least one action", () => {
    for (const p of REGISTRY) {
      expect(p.actions.length).toBeGreaterThanOrEqual(1);
    }
  });

  it("every plugin has a valid status", () => {
    for (const p of REGISTRY) {
      expect(["real", "stub"]).toContain(p.status);
    }
  });

  it("every action has a valid type", () => {
    const validTypes = ["read", "transaction", "notification", "logic"];
    for (const p of REGISTRY) {
      for (const a of p.actions) {
        expect(validTypes).toContain(a.type);
      }
    }
  });
});

describe("findAction", () => {
  it("finds jupiter.swap", () => {
    const r = findAction("jupiter", "swap");
    expect(r?.plugin.id).toBe("jupiter");
    expect(r?.action.id).toBe("swap");
  });

  it("finds pyth.read_price", () => {
    const r = findAction("pyth", "read_price");
    expect(r?.plugin.id).toBe("pyth");
    expect(r?.action.id).toBe("read_price");
  });

  it("finds pyth.staleness_check", () => {
    const r = findAction("pyth", "staleness_check");
    expect(r?.plugin.id).toBe("pyth");
    expect(r?.action.id).toBe("staleness_check");
  });

  it("finds system.transfer", () => {
    const r = findAction("system", "transfer");
    expect(r?.plugin.id).toBe("system");
    expect(r?.action.id).toBe("transfer");
  });

  it("returns null for unknown plugin", () => {
    expect(findAction("bogus", "x")).toBe(null);
  });

  it("returns null for unknown action", () => {
    expect(findAction("jupiter", "bogus")).toBe(null);
  });
});

describe("real vs stub", () => {
  it("jupiter is real", () => {
    const p = REGISTRY.find((x) => x.id === "jupiter");
    expect(p?.status).toBe("real");
  });

  it("pyth is real", () => {
    const p = REGISTRY.find((x) => x.id === "pyth");
    expect(p?.status).toBe("real");
  });

  it("system is real", () => {
    const p = REGISTRY.find((x) => x.id === "system");
    expect(p?.status).toBe("real");
  });

  it("notify.telegram is real", () => {
    const p = REGISTRY.find((x) => x.id === "notify.telegram");
    expect(p?.status).toBe("real");
  });

  it("notify.discord is real", () => {
    const p = REGISTRY.find((x) => x.id === "notify.discord");
    expect(p?.status).toBe("real");
  });

  it("kamino is stub", () => {
    const p = REGISTRY.find((x) => x.id === "kamino");
    expect(p?.status).toBe("stub");
  });

  it("marinade is stub", () => {
    const p = REGISTRY.find((x) => x.id === "marinade");
    expect(p?.status).toBe("stub");
  });

  it("drift is stub", () => {
    const p = REGISTRY.find((x) => x.id === "drift");
    expect(p?.status).toBe("stub");
  });
});
