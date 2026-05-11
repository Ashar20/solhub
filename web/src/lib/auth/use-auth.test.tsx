import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAuth } from "./use-auth";

beforeEach(() => { window.localStorage.clear(); });

describe("useAuth", () => {
  it("starts unauthenticated when no token in storage", () => {
    const { result } = renderHook(() => useAuth());
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.token).toBe(null);
  });

  it("ready becomes true after mount", async () => {
    const { result } = renderHook(() => useAuth());
    // After useEffect runs, ready should flip true
    await new Promise((r) => setTimeout(r, 0));
    expect(result.current.ready).toBe(true);
  });

  it("signIn persists token to localStorage and updates state", () => {
    const { result } = renderHook(() => useAuth());
    act(() => result.current.signIn("abc123"));
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.token).toBe("abc123");
    expect(window.localStorage.getItem("solhub.bearer")).toBe("abc123");
  });

  it("signOut clears token", () => {
    window.localStorage.setItem("solhub.bearer", "preexisting");
    const { result } = renderHook(() => useAuth());
    act(() => result.current.signOut());
    expect(result.current.isAuthenticated).toBe(false);
    expect(window.localStorage.getItem("solhub.bearer")).toBe(null);
  });

  it("reacts to storage events from other tabs", async () => {
    const { result } = renderHook(() => useAuth());
    act(() => {
      window.dispatchEvent(new StorageEvent("storage", {
        key: "solhub.bearer", newValue: "from-other-tab",
      }));
    });
    expect(result.current.token).toBe("from-other-tab");
  });
});
