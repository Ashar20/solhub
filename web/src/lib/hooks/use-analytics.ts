"use client";
import { useQuery } from "@tanstack/react-query";
import { analytics } from "@/lib/api";

export const useAnalytics = () =>
  useQuery({
    queryKey: ["analytics"] as const,
    queryFn: analytics.getAnalytics,
  });
