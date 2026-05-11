"use client";
import { use } from "react";
import { BuilderShell } from "@/components/workflow/builder/BuilderShell";

export default function BuilderPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <BuilderShell id={id} />;
}
