import { describe, it, expect, beforeEach, vi } from "vitest";
import { z } from "zod";
import { apiRequest, ApiError, setToken, clearToken } from "./client";

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  vi.stubGlobal("fetch", F);
  clearToken();
});

const Ok = z.object({ ok: z.boolean() });

describe("apiRequest", () => {
  it("attaches Bearer token", async () => {
    setToken("test-key");
    F.mockResolvedValueOnce({
      ok: true, status: 200, statusText: "OK",
      text: async () => JSON.stringify({ ok: true }),
    } as Response);
    await apiRequest("/v1/ping", Ok);
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).headers).toMatchObject({ Authorization: "Bearer test-key" });
  });

  it("omits Bearer when anonymous: true", async () => {
    setToken("test-key");
    F.mockResolvedValueOnce({
      ok: true, status: 200, statusText: "OK",
      text: async () => JSON.stringify({ ok: true }),
    } as Response);
    await apiRequest("/v1/hub", Ok, { anonymous: true });
    const [, init] = F.mock.calls[0]!;
    expect((init as RequestInit).headers).not.toHaveProperty("Authorization");
  });

  it("encodes query params", async () => {
    F.mockResolvedValueOnce({
      ok: true, status: 200, statusText: "OK",
      text: async () => JSON.stringify({ ok: true }),
    } as Response);
    await apiRequest("/v1/runs", Ok, { query: { workflow_id: "abc", limit: 10 } });
    const [url] = F.mock.calls[0]!;
    expect(String(url)).toContain("workflow_id=abc");
    expect(String(url)).toContain("limit=10");
  });

  it("skips undefined/null query values", async () => {
    F.mockResolvedValueOnce({
      ok: true, status: 200, statusText: "OK",
      text: async () => JSON.stringify({ ok: true }),
    } as Response);
    await apiRequest("/v1/runs", Ok, { query: { workflow_id: undefined, status: null, limit: 10 } });
    const [url] = F.mock.calls[0]!;
    const u = String(url);
    expect(u).not.toContain("workflow_id");
    expect(u).not.toContain("status");
    expect(u).toContain("limit=10");
  });

  it("throws ApiError on non-2xx with JSON body", async () => {
    F.mockResolvedValueOnce({
      ok: false, status: 404, statusText: "Not Found",
      text: async () => JSON.stringify({ code: "not_found", message: "missing" }),
    } as Response);
    await expect(apiRequest("/v1/x", Ok)).rejects.toBeInstanceOf(ApiError);
  });

  it("throws ApiError on non-2xx with non-JSON body", async () => {
    F.mockResolvedValueOnce({
      ok: false, status: 500, statusText: "Internal Server Error",
      text: async () => "<html>500</html>",
    } as Response);
    await expect(apiRequest("/v1/x", Ok)).rejects.toBeInstanceOf(ApiError);
  });

  it("returns undefined for 204 (with z.void())", async () => {
    F.mockResolvedValueOnce({
      ok: true, status: 204, statusText: "No Content",
      text: async () => "",
    } as Response);
    const r = await apiRequest("/v1/x", z.void());
    expect(r).toBeUndefined();
  });

  it("clears token + throws on 401", async () => {
    setToken("test-key");
    // Mock window.location to avoid jsdom navigation errors
    Object.defineProperty(window, "location", {
      value: { href: "" },
      writable: true,
      configurable: true,
    });
    F.mockResolvedValueOnce({
      ok: false, status: 401, statusText: "Unauthorized",
      text: async () => JSON.stringify({ code: "unauthorized", message: "bad token" }),
    } as Response);
    await expect(apiRequest("/v1/x", Ok)).rejects.toBeInstanceOf(ApiError);
    expect(window.localStorage.getItem("solhub.bearer")).toBeNull();
  });
});
