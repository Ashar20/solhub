"use client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { orgs } from "@/lib/api";

export const useApiKeys = () =>
  useQuery({
    queryKey: ["org", "me", "api_keys"] as const,
    queryFn: orgs.listApiKeys,
  });

export function useCreateApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => orgs.createApiKey(name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["org", "me", "api_keys"] }),
  });
}

export function useRevokeApiKey() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => orgs.revokeApiKey(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["org", "me", "api_keys"] }),
  });
}
