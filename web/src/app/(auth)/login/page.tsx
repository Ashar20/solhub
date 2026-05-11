"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/auth/use-auth";
import { orgs } from "@/lib/api";
import { ApiError, setToken, clearToken } from "@/lib/api/client";
import { Btn } from "@/components/primitives/Btn";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";

function devLoginPrefill(): string {
  if (process.env.NODE_ENV !== "development") return "";
  return (process.env.NEXT_PUBLIC_DEV_LOGIN_API_KEY ?? "").trim();
}

export default function LoginPage() {
  const router = useRouter();
  const { signIn } = useAuth();
  const [value, setValue] = useState(devLoginPrefill);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!value.trim()) return;
    setError(null);
    setBusy(true);
    // Probe the proposed token by writing it then calling getMe()
    setToken(value);
    try {
      await orgs.getMe();
      signIn(value);
      router.replace("/dashboard");
    } catch (err) {
      // Clean up the bad token regardless of error type
      clearToken();
      if (err instanceof ApiError) {
        setError(err.status === 401 ? "Invalid API key" : `Request failed (${err.status})`);
      } else {
        setError("Network error — is the backend running?");
      }
      setBusy(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      className="w-[400px] rounded-xl border border-ink-200 bg-white shadow-card p-8 space-y-4"
    >
      <SolhubLogo />
      <div>
        <h1 className="text-[20px] font-semibold tracking-tight">Sign in</h1>
        <p className="text-[12px] text-ink-500 mt-1">
          Paste your API key from the SolHub backend. Stored locally in your browser.
        </p>
      </div>
      <label className="block">
        <span className="text-[12px] font-medium text-ink-700">API key</span>
        <input
          type="password"
          autoFocus
          autoComplete="off"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px] font-mono focus:outline-none focus:ring-2 focus:ring-violet-500/30"
          placeholder="sk_live_…"
        />
      </label>
      {error && <p className="text-[12px] text-rose-600">{error}</p>}
      <Btn
        type="submit"
        variant="primary"
        size="lg"
        disabled={busy || !value.trim()}
        className="w-full justify-center"
      >
        {busy ? "Verifying…" : "Sign in"}
      </Btn>
    </form>
  );
}
