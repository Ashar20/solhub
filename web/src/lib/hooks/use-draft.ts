"use client";
import { useCallback, useLayoutEffect, useState } from "react";

export interface DraftState {
  name: string;
  nodes: unknown[];
  edges: unknown[];
  params: Record<string, Record<string, unknown>>;
  updatedAt: string;
}

function storageKey(id: string) {
  return `solhub.draft.${id}`;
}

export interface UseDraftResult {
  /** The draft loaded from storage on mount, or null. Stable reference once loaded. */
  draft: DraftState | null;
  save: (d: Omit<DraftState, "updatedAt">) => void;
  clear: () => void;
}

function readDraft(id: string): DraftState | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(storageKey(id));
  if (!raw) return null;
  try {
    return JSON.parse(raw) as DraftState;
  } catch {
    return null;
  }
}

export function useDraft(id: string): UseDraftResult {
  const [draft, setDraft] = useState<DraftState | null>(null);

  // useLayoutEffect: load before paint so the workflow canvas can measure + fitView on first frame.
  useLayoutEffect(() => {
    setDraft(readDraft(id));
  }, [id]);

  const save = useCallback((d: Omit<DraftState, "updatedAt">) => {
    if (typeof window === "undefined") return;
    const next: DraftState = { ...d, updatedAt: new Date().toISOString() };
    window.localStorage.setItem(storageKey(id), JSON.stringify(next));
    setDraft(next);
  }, [id]);

  const clear = useCallback(() => {
    if (typeof window === "undefined") return;
    window.localStorage.removeItem(storageKey(id));
    setDraft(null);
  }, [id]);

  return { draft, save, clear };
}
