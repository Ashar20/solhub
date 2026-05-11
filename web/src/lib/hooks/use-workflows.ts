"use client";
import { useQuery } from "@tanstack/react-query";
import { workflows } from "@/lib/api";
import type { ListWorkflowsParams } from "@/lib/api/workflows";

export const useWorkflows = (params: ListWorkflowsParams = {}) =>
  useQuery({
    queryKey: ["workflows", params] as const,
    queryFn: () => workflows.listWorkflows(params),
  });

export const useWorkflow = (id: string | undefined) =>
  useQuery({
    queryKey: ["workflow", id] as const,
    queryFn: () => workflows.getWorkflow(id!),
    enabled: !!id && id !== "new",
  });
