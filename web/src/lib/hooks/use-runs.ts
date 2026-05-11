"use client";
import { useQuery } from "@tanstack/react-query";
import { runs } from "@/lib/api";
import type { ListRunsParams } from "@/lib/api/runs";

export const useRuns = (params: ListRunsParams = {}) =>
  useQuery({
    queryKey: ["runs", params] as const,
    queryFn: () => runs.listRuns(params),
    refetchInterval: () => {
      if (typeof document === "undefined") return false;
      return document.visibilityState === "visible" ? 5000 : false;
    },
  });

export const useRun = (run_id: string | undefined, pollMs?: number | false) =>
  useQuery({
    queryKey: ["run", run_id] as const,
    queryFn: () => runs.getRun(run_id!),
    enabled: !!run_id,
    refetchInterval: pollMs ?? false,
  });
