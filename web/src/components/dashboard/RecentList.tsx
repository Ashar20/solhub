import Link from "next/link";

export function RecentList({ title, items, emptyText }: {
  title: string;
  items: { id: string; primary: React.ReactNode; secondary?: React.ReactNode; href: string }[];
  emptyText: string;
}) {
  return (
    <div className="rounded-xl border border-ink-200 bg-white shadow-card">
      <div className="px-4 h-10 border-b border-ink-200 flex items-center text-[12px] font-medium text-ink-700">
        {title}
      </div>
      <ul>
        {items.length === 0 && (
          <li className="px-4 py-6 text-[12px] text-ink-500">{emptyText}</li>
        )}
        {items.map((it) => (
          <li key={it.id} className="border-b border-ink-100 last:border-b-0">
            <Link href={it.href} className="block px-4 py-2.5 hover:bg-ink-50 text-[13px]">
              <div className="text-ink-900 font-medium truncate">{it.primary}</div>
              {it.secondary && <div className="text-[11px] text-ink-500 mt-0.5">{it.secondary}</div>}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
