"use client";
import { Topbar } from "@/components/shell/Topbar";
import { useHub } from "@/lib/hooks/use-hub";
import { PlaybookCard } from "@/components/marketplace/PlaybookCard";

export default function MarketplacePage() {
  const { data, isLoading } = useHub();
  return (
    <>
      <Topbar crumbs={["Hub", "Marketplace"]} />
      <main className="flex-1 p-6 overflow-y-auto">
        {isLoading && <div className="text-[12px] text-ink-500">Loading…</div>}
        {!isLoading && (data ?? []).length === 0 && (
          <div className="text-[12px] text-ink-500">No published workflows yet.</div>
        )}
        <div className="grid grid-cols-3 gap-3">
          {(data ?? []).map((w) => <PlaybookCard key={w.id} w={w} />)}
        </div>
      </main>
    </>
  );
}
