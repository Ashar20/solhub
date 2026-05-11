import { describe, it, expect } from "vitest";
import { cn } from "./cn";

describe("cn", () => {
  it("joins truthy classes", () => {
    expect(cn("a", "b")).toBe("a b");
  });
  it("drops falsy values", () => {
    expect(cn("a", false, undefined, null, 0, "b")).toBe("a b");
  });
  it("merges conflicting Tailwind classes (later wins)", () => {
    expect(cn("px-2", "px-4")).toBe("px-4");
  });
});
