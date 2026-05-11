"use client";
import { useEffect, useState } from "react";
import { getToken, setToken as setStorageToken, clearToken as clearStorageToken } from "@/lib/api/client";

export interface UseAuthResult {
  token: string | null;
  isAuthenticated: boolean;
  ready: boolean;
  signIn: (t: string) => void;
  signOut: () => void;
}

export function useAuth(): UseAuthResult {
  const [token, setTokenState] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setTokenState(getToken());
    setReady(true);
    const onStorage = (e: StorageEvent) => {
      if (e.key === "solhub.bearer") setTokenState(e.newValue);
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  return {
    token,
    isAuthenticated: !!token,
    ready,
    signIn: (t: string) => { setStorageToken(t); setTokenState(t); },
    signOut: () => { clearStorageToken(); setTokenState(null); },
  };
}
