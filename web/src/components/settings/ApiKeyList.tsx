"use client";
import { useApiKeys, useRevokeApiKey } from "@/lib/hooks/use-api-keys";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";
import { Pill } from "@/components/primitives/Pill";
import { formatRelativeTime } from "@/lib/utils/format";

export function ApiKeyList() {
  const { data, isLoading } = useApiKeys();
  const revoke = useRevokeApiKey();

  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card overflow-hidden">
      <div className="grid grid-cols-[1fr_140px_140px_100px] items-center px-4 h-9 border-b border-ink-200 text-[11px] uppercase tracking-wider font-mono text-ink-500">
        <div>Name</div>
        <div>Last used</div>
        <div>Created</div>
        <div className="text-right">Action</div>
      </div>
      {isLoading && <div className="p-6 text-[12px] text-ink-500">Loading…</div>}
      {!isLoading && (data ?? []).length === 0 && (
        <div className="p-6 text-[12px] text-ink-500">No API keys yet.</div>
      )}
      {(data ?? []).map((k) => (
        <div
          key={k.id}
          className="grid grid-cols-[1fr_140px_140px_100px] items-center px-4 h-11 border-b border-ink-100 last:border-b-0 text-[13px]"
        >
          <div className="flex items-center gap-2 min-w-0">
            <span className="font-medium text-ink-900 truncate">{k.name ?? "(unnamed)"}</span>
            {k.revoked_at && <Pill tone="rose">revoked</Pill>}
          </div>
          <div className="text-[12px] text-ink-500 font-mono">
            {k.last_used_at ? formatRelativeTime(k.last_used_at) : "—"}
          </div>
          <div className="text-[12px] text-ink-500 font-mono">
            {formatRelativeTime(k.created_at)}
          </div>
          <div className="flex justify-end">
            {!k.revoked_at && (
              <Btn
                variant="danger"
                size="sm"
                icon={<Icon name="trash" className="w-3.5 h-3.5" />}
                onClick={() => {
                  if (confirm(`Revoke key "${k.name ?? "(unnamed)"}"?`))
                    revoke.mutate(k.id);
                }}
              >
                Revoke
              </Btn>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
