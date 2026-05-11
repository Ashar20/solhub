import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import LoginPage from "./page";

// Prevent jsdom location-assignment warnings/errors from the 401 handler in client.ts
Object.defineProperty(window, "location", {
  writable: true,
  configurable: true,
  value: { ...window.location, href: "" },
});

const mockReplace = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: mockReplace, push: vi.fn(), back: vi.fn() }),
}));

const F = vi.fn();
beforeEach(() => {
  F.mockReset();
  mockReplace.mockReset();
  vi.stubGlobal("fetch", F);
  window.localStorage.clear();
});

function wrap(children: React.ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

function jsonRes(body: unknown, status = 200) {
  return {
    ok: status < 400,
    status,
    statusText: status === 200 ? "OK" : "Unauthorized",
    text: async () => JSON.stringify(body),
  } as Response;
}

describe("LoginPage", () => {
  it("renders the form", () => {
    render(wrap(<LoginPage />));
    expect(screen.getByRole("heading", { name: "Sign in" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /sign in/i })).toBeInTheDocument();
    expect(screen.getByPlaceholderText("sk_live_…")).toBeInTheDocument();
  });

  it("submits valid key, stores token, redirects", async () => {
    F.mockResolvedValueOnce(
      jsonRes({
        id: "11111111-1111-1111-1111-111111111111",
        name: "test",
        wallet_address: null,
        credits_usdc: 0,
        created_at: "2026-05-11T00:00:00Z",
      }),
    );
    render(wrap(<LoginPage />));
    fireEvent.change(screen.getByPlaceholderText("sk_live_…"), { target: { value: "good-key" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in/i }));
    await waitFor(() => expect(mockReplace).toHaveBeenCalledWith("/dashboard"));
    expect(window.localStorage.getItem("solhub.bearer")).toBe("good-key");
  });

  it("shows error and clears token on 401", async () => {
    F.mockResolvedValueOnce(jsonRes({ code: "unauthorized", message: "bad" }, 401));
    render(wrap(<LoginPage />));
    fireEvent.change(screen.getByPlaceholderText("sk_live_…"), { target: { value: "bad-key" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in/i }));
    await waitFor(() => expect(screen.getByText(/Invalid API key/i)).toBeInTheDocument());
    expect(window.localStorage.getItem("solhub.bearer")).toBe(null);
    expect(mockReplace).not.toHaveBeenCalled();
  });

  it("disables submit when empty", () => {
    render(wrap(<LoginPage />));
    expect(screen.getByRole("button", { name: /sign in/i })).toBeDisabled();
  });
});
