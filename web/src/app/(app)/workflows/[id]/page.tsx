"use client";
import { BuilderShell } from "@/components/workflow/builder/BuilderShell";

export default function BuilderPage({ params }: { params: { id: string } }) {
  return <BuilderShell id={params.id} />;
}
