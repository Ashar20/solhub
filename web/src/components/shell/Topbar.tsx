"use client";
import { Breadcrumb } from "./Breadcrumb";
import { Icon } from "@/components/primitives/Icon";
import { Kbd } from "@/components/primitives/Kbd";
import { NetworkChip } from "@/components/wallet/NetworkChip";
import { ConnectButton } from "@/components/wallet/ConnectButton";

export function Topbar({ crumbs, right }: { crumbs: string[]; right?: React.ReactNode }) {
  return (
    <header className="h-14 px-6 border-b border-ink-200 bg-white flex items-center justify-between">
      <Breadcrumb items={crumbs} />
      <div className="flex items-center gap-2">
        {right}
        <NetworkChip />
        <ConnectButton />
        <div className="flex items-center h-8 rounded-md border border-ink-200 bg-ink-50">
          <Icon name="search" className="w-3.5 h-3.5 text-ink-400 ml-2.5" />
          <input
            placeholder="Search workspace…"
            className="px-2 w-64 text-[12px] focus:outline-none bg-transparent"
          />
          <span className="mr-2"><Kbd>⌘K</Kbd></span>
        </div>
      </div>
    </header>
  );
}
