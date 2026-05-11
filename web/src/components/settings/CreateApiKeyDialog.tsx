"use client";
import { useState } from "react";
import { useCreateApiKey } from "@/lib/hooks/use-api-keys";
import { Btn } from "@/components/primitives/Btn";
import { Icon } from "@/components/primitives/Icon";

export interface CreateApiKeyDialogProps {
  onClose: () => void;
}

export function CreateApiKeyDialog({ onClose }: CreateApiKeyDialogProps) {
  const [name, setName] = useState("");
  const [raw, setRaw] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const create = useCreateApiKey();

  async function submit() {
    const trimmed = name.trim() || "untitled";
    const r = await create.mutateAsync(trimmed);
    setRaw(r.key);
  }

  async function copy() {
    if (!raw) return;
    await navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/30 flex items-center justify-center">
      <div className="w-[480px] rounded-xl bg-white shadow-pop border border-ink-200 p-5 space-y-4">
        {!raw ? (
          <>
            <div className="flex items-start justify-between">
              <h2 className="text-[16px] font-semibold tracking-tight">New API key</h2>
              <button onClick={onClose} className="text-ink-500 hover:text-ink-900">
                <Icon name="x" className="w-4 h-4" />
              </button>
            </div>
            <label className="block">
              <span className="text-[12px] font-medium">Name</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="server-backend"
                disabled={create.isPending}
                className="mt-1 w-full h-9 px-3 rounded-md border border-ink-200 text-[13px]"
              />
            </label>
            {create.isError && (
              <p className="text-[12px] text-rose-600">
                {create.error instanceof Error ? create.error.message : "Failed"}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <Btn variant="ghost" onClick={onClose} disabled={create.isPending}>
                Cancel
              </Btn>
              <Btn variant="primary" onClick={submit} disabled={create.isPending}>
                {create.isPending ? "Creating…" : "Create"}
              </Btn>
            </div>
          </>
        ) : (
          <>
            <h2 className="text-[16px] font-semibold tracking-tight">Save this key now</h2>
            <p className="text-[12px] text-ink-500">It will not be shown again.</p>
            <div className="flex items-center gap-2 rounded-md border border-ink-200 bg-ink-50 px-3 py-2">
              <code className="flex-1 text-[12px] font-mono break-all">{raw}</code>
              <button onClick={copy} className="text-ink-700 hover:text-ink-900" aria-label="Copy">
                <Icon name={copied ? "check" : "copy"} className="w-4 h-4" />
              </button>
            </div>
            <div className="flex justify-end">
              <Btn variant="primary" onClick={onClose}>
                Done
              </Btn>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
