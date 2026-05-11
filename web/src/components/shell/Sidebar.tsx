"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Icon, type IconName } from "@/components/primitives/Icon";
import { SolhubLogo } from "@/components/primitives/SolhubLogo";
import { cn } from "@/lib/utils/cn";

const NAV: { href: string; label: string; icon: IconName }[] = [
  { href: "/dashboard",   label: "Dashboard",   icon: "dashboard" },
  { href: "/workflows",   label: "Workflows",   icon: "workflows" },
  { href: "/ai",          label: "AI Builder",  icon: "ai" },
  { href: "/runs",        label: "Runs & Logs", icon: "runs" },
  { href: "/marketplace", label: "Marketplace", icon: "marketplace" },
  { href: "/wallet",      label: "Wallet",      icon: "wallet" },
  { href: "/versions",    label: "Versions",    icon: "versions" },
  { href: "/settings",    label: "Settings",    icon: "settings" },
];

export function Sidebar() {
  const pathname = usePathname();
  return (
    <aside className="w-56 shrink-0 h-screen border-r border-ink-200 bg-white flex flex-col">
      <div className="h-14 px-4 flex items-center border-b border-ink-200">
        <SolhubLogo />
      </div>
      <nav className="flex-1 p-2 space-y-0.5 overflow-y-auto scrollbar-thin">
        {NAV.map((n) => {
          const active = pathname?.startsWith(n.href) ?? false;
          return (
            <Link
              key={n.href}
              href={n.href}
              className={cn(
                "flex items-center gap-2.5 px-2.5 h-8 rounded-md text-[13px] font-medium",
                active
                  ? "bg-ink-100 text-ink-900"
                  : "text-ink-600 hover:bg-ink-50 hover:text-ink-900",
              )}
            >
              <Icon name={n.icon} className="w-4 h-4" />
              {n.label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
