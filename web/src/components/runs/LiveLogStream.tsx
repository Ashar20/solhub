"use client";
import { useEffect, useRef } from "react";
import type { RunLogEvent } from "@/lib/api/schemas";
import { Pill } from "@/components/primitives/Pill";
import type { RunStreamState } from "@/lib/hooks/use-run-stream";

export function LiveLogStream({ events, state }: {
  events: RunLogEvent[];
  state: RunStreamState;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.scrollTo({ top: ref.current.scrollHeight });
  }, [events.length]);

  const tone =
    state === "streaming" ? "emerald"
    : state === "polling" ? "amber"
    : state === "error" ? "rose"
    : "ink";

  return (
    <div className="rounded-xl border border-ink-200 bg-ink-950 text-ink-100 h-full flex flex-col">
      <div className="h-9 px-3 border-b border-ink-800 flex items-center justify-between">
        <div className="text-[11px] font-mono uppercase tracking-wider text-ink-400">Live log</div>
        <Pill tone={tone}>{state}</Pill>
      </div>
      <div ref={ref} className="flex-1 overflow-y-auto p-3 font-mono text-[11px] leading-relaxed scrollbar-thin">
        {events.length === 0 && <div className="text-ink-500">Waiting for first event…</div>}
        {events.map((e, i) => (
          <div key={i} className="whitespace-pre-wrap">
            <span className={
              e.event === "run_complete" ? "text-emerald-400" : "text-violet-300"
            }>{e.event}</span>
            {" "}
            <span className="text-ink-200">{JSON.stringify(e.data)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
