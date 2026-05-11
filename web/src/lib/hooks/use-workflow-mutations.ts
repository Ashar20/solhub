"use client";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { workflows } from "@/lib/api";
import type { CreateWorkflowBody, UpdateWorkflowBody } from "@/lib/api/workflows";

export function useCreateWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateWorkflowBody) => workflows.createWorkflow(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
}

export function useUpdateWorkflow(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateWorkflowBody) =>
      workflows.updateWorkflow(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["workflows"] });
      qc.invalidateQueries({ queryKey: ["workflow", id] });
    },
  });
}

export function useDeleteWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => workflows.deleteWorkflow(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["workflows"] }),
  });
}

export function useTriggerWorkflow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: { id: string; overrides?: Record<string, unknown> }) =>
      workflows.triggerWorkflow(vars.id, vars.overrides),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["runs"] }),
  });
}
