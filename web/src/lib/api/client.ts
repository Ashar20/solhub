import { z } from "zod";

export class ApiError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

const BEARER_KEY = "solhub.bearer";

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return window.localStorage.getItem(BEARER_KEY);
}
export function setToken(token: string): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(BEARER_KEY, token);
}
export function clearToken(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(BEARER_KEY);
}

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";

export interface RequestOpts {
  method?: "GET" | "POST" | "PATCH" | "DELETE" | "PUT";
  body?: unknown;
  query?: Record<string, string | number | boolean | undefined | null>;
  /** When true, do NOT attach the Bearer token (used for public Hub endpoints). */
  anonymous?: boolean;
  /** Optional override of the base URL — used for tests. */
  baseUrl?: string;
}

export async function apiRequest<T>(
  path: string,
  schema: z.ZodSchema<T>,
  opts: RequestOpts = {},
): Promise<T> {
  const base = opts.baseUrl ?? API_BASE;
  const url = new URL(path, base);
  if (opts.query) {
    for (const [k, v] of Object.entries(opts.query)) {
      if (v !== undefined && v !== null) url.searchParams.set(k, String(v));
    }
  }

  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (!opts.anonymous) {
    const tok = getToken();
    if (tok) headers["Authorization"] = `Bearer ${tok}`;
  }

  const res = await fetch(url.toString(), {
    method: opts.method ?? "GET",
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
  });

  if (res.status === 401 && !opts.anonymous) {
    clearToken();
    if (typeof window !== "undefined") {
      try {
        // Soft redirect; component-level boundaries will handle the navigation.
        window.location.href = "/login";
      } catch {
        // jsdom may throw on location assignment — ignore, throw ApiError below
      }
    }
    throw new ApiError(401, "unauthorized", "Session expired");
  }

  if (!res.ok) {
    let code = "http_error";
    let message = res.statusText || `HTTP ${res.status}`;
    try {
      const j = await res.json();
      if (typeof j?.code === "string") code = j.code;
      if (typeof j?.message === "string") message = j.message;
      else if (typeof j?.error === "string") message = j.error;
    } catch {
      // response was not JSON — keep statusText
    }
    throw new ApiError(res.status, code, message);
  }

  if (res.status === 204) {
    // 204 No Content — schema must accept undefined (use z.void() at call sites).
    return schema.parse(undefined);
  }

  const text = await res.text();
  if (text.length === 0) {
    return schema.parse(undefined);
  }
  const body = JSON.parse(text);
  return schema.parse(body);
}
