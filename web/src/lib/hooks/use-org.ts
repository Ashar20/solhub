"use client";
import { useQuery } from "@tanstack/react-query";
import { orgs } from "@/lib/api";

export const useMe = () =>
  useQuery({
    queryKey: ["org", "me"] as const,
    queryFn: orgs.getMe,
  });
