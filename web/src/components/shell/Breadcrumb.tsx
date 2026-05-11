import { Icon } from "@/components/primitives/Icon";

export function Breadcrumb({ items }: { items: string[] }) {
  return (
    <div className="flex items-center text-[12px] text-ink-500 gap-1.5">
      {items.map((item, i) => (
        <span key={i} className="flex items-center gap-1.5">
          {i > 0 && <Icon name="chevron" className="w-3 h-3 text-ink-300" />}
          <span className={i === items.length - 1 ? "text-ink-900 font-medium" : ""}>
            {item}
          </span>
        </span>
      ))}
    </div>
  );
}
