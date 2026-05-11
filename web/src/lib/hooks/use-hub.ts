"use client";
import { useQuery } from "@tanstack/react-query";
import { hub } from "@/lib/api";

export const useHub = () =>
  useQuery({
    queryKey: ["hub"] as const,
    queryFn: hub.listHub,
  });
