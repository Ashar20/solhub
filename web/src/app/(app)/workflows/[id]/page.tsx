"use client";
import { Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { BuilderShell } from "@/components/workflow/builder/BuilderShell";

function BuilderWithRemountKey({ id }: { id: string }) {
  const sp = useSearchParams();
  const from = sp.get("from");
  const key = from ? `${id}:${from}` : id;
  return <BuilderShell key={key} id={id} />;
}

export default function BuilderPage({ params }: { params: { id: string } }) {
  return (
    <Suspense
      fallback={
        <div className="flex flex-1 items-center justify-center p-6 text-[13px] text-ink-500">
          Loading builder…
        </div>
      }
    >
      <BuilderWithRemountKey id={params.id} />
    </Suspense>
  );
}
