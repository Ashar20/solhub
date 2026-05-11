"use client";
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { getRun } from "@/lib/api/runs";
import { RunLogEventSchema, type RunLogEvent } from "@/lib/api/schemas";

export type RunStreamState = "idle" | "streaming" | "polling" | "closed" | "error";

export interface UseRunStreamResult {
  events: RunLogEvent[];
  state: RunStreamState;
  reset: () => void;
}

function bearer(): string | null {
  if (typeof window === "undefined") return null;
  return window.localStorage.getItem("solhub.bearer");
}

function proxyUrl(run_id: string): string {
  const token = bearer() ?? "";
  return `/api/runs/${encodeURIComponent(run_id)}/logs?token=${encodeURIComponent(token)}`;
}

export function useRunStream(run_id: string | undefined): UseRunStreamResult {
  const [events, setEvents] = useState<RunLogEvent[]>([]);
  const [state, setState] = useState<RunStreamState>("idle");
  const esRef = useRef<EventSource | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const qc = useQueryClient();

  function clearPolling() {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }

  function close() {
    esRef.current?.close();
    esRef.current = null;
    clearPolling();
  }

  function reset() {
    close();
    setEvents([]);
    setState("idle");
  }

  useEffect(() => {
    if (!run_id) return;
    setEvents([]);
    setState("streaming");

    const es = new EventSource(proxyUrl(run_id));
    esRef.current = es;

    es.onmessage = (e) => {
      try {
        const evt = RunLogEventSchema.parse(JSON.parse(e.data as string));
        setEvents((prev) => [...prev, evt]);
        if (evt.event === "run_complete") {
          es.close();
          setState("closed");
          void qc.invalidateQueries({ queryKey: ["run", run_id] });
        }
      } catch {
        // Drop malformed events silently
      }
    };

    es.onerror = () => {
      es.close();
      esRef.current = null;
      setState("polling");
      pollRef.current = setInterval(() => {
        void (async () => {
          try {
            const r = await getRun(run_id);
            if (
              r.status === "Confirmed" ||
              r.status === "Failed" ||
              r.status === "Skipped"
            ) {
              clearPolling();
              setState("closed");
              void qc.invalidateQueries({ queryKey: ["run", run_id] });
            }
          } catch {
            clearPolling();
            setState("error");
          }
        })();
      }, 1000);
    };

    return close;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run_id, qc]);

  return { events, state, reset };
}
