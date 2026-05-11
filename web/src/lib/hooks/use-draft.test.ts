import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useDraft } from "./use-draft";

beforeEach(() => { window.localStorage.clear(); });

describe("useDraft", () => {
  it("returns null initially when no draft exists", () => {
    const { result } = renderHook(() => useDraft("wf-1"));
    expect(result.current.draft).toBe(null);
  });

  it("save() writes to localStorage", () => {
    const { result } = renderHook(() => useDraft("wf-1"));
    act(() => {
      result.current.save({ name: "x", nodes: [], edges: [], params: {} });
    });
    const raw = window.localStorage.getItem("solhub.draft.wf-1");
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw!).name).toBe("x");
    expect(result.current.draft?.name).toBe("x");
  });

  it("loads existing draft on mount", () => {
    window.localStorage.setItem(
      "solhub.draft.wf-2",
      JSON.stringify({ name: "pre", nodes: [], edges: [], params: {}, updatedAt: "2026-05-11T00:00:00Z" }),
    );
    const { result } = renderHook(() => useDraft("wf-2"));
    expect(result.current.draft?.name).toBe("pre");
  });

  it("clear() removes from localStorage and sets draft to null", () => {
    window.localStorage.setItem(
      "solhub.draft.wf-3",
      JSON.stringify({ name: "x", nodes: [], edges: [], params: {}, updatedAt: "z" }),
    );
    const { result } = renderHook(() => useDraft("wf-3"));
    expect(result.current.draft?.name).toBe("x");
    act(() => result.current.clear());
    expect(window.localStorage.getItem("solhub.draft.wf-3")).toBe(null);
    expect(result.current.draft).toBe(null);
  });

  it("ignores malformed JSON in storage", () => {
    window.localStorage.setItem("solhub.draft.wf-4", "{not json");
    const { result } = renderHook(() => useDraft("wf-4"));
    expect(result.current.draft).toBe(null);
  });
});
