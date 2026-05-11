"use client";
import { z, type ZodTypeAny } from "zod";

function unwrap(schema: ZodTypeAny): ZodTypeAny {
  // Strip ZodDefault, ZodOptional, ZodNullable, ZodEffects to reach the inner kind.
  // (z.coerce.number() is ZodEffects wrapping ZodNumber; ditto bigint.)
  // Each iteration peels one layer.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let s: any = schema;
  while (true) {
    if (s instanceof z.ZodDefault) s = s._def.innerType;
    else if (s instanceof z.ZodOptional) s = s._def.innerType;
    else if (s instanceof z.ZodNullable) s = s._def.innerType;
    else if (s instanceof z.ZodEffects) s = s._def.schema;
    else break;
  }
  return s;
}

type FieldKind = "enum" | "boolean" | "number" | "bigint" | "string";

function kindOf(schema: ZodTypeAny): FieldKind {
  const u = unwrap(schema);
  if (u instanceof z.ZodEnum) return "enum";
  if (u instanceof z.ZodBoolean) return "boolean";
  if (u instanceof z.ZodNumber) return "number";
  if (u instanceof z.ZodBigInt) return "bigint";
  return "string";
}

function enumOptions(schema: ZodTypeAny): readonly string[] {
  const u = unwrap(schema);
  if (u instanceof z.ZodEnum) return u.options as readonly string[];
  return [];
}

export interface ZodFormProps {
  schema: z.ZodObject<z.ZodRawShape>;
  value: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
  /** Keys to skip rendering (used when a custom field like WorkflowPicker is rendered above). */
  skip?: readonly string[];
}

export function ZodForm({ schema, value, onChange, skip }: ZodFormProps) {
  const shape = schema.shape as Record<string, ZodTypeAny>;

  return (
    <div className="space-y-3">
      {Object.entries(shape).filter(([key]) => !skip?.includes(key)).map(([key, raw]) => {
        const kind = kindOf(raw);
        const current = value[key];

        if (kind === "enum") {
          const options = enumOptions(raw);
          const selected = typeof current === "string" ? current : (options[0] ?? "");
          return (
            <label key={key} className="block">
              <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider">{key}</span>
              <select
                value={selected}
                onChange={(e) => onChange({ ...value, [key]: e.target.value })}
                className="mt-1 w-full h-8 px-2 rounded-md border border-ink-200 text-[13px] bg-white"
              >
                {options.map((opt) => <option key={opt} value={opt}>{opt}</option>)}
              </select>
            </label>
          );
        }

        if (kind === "boolean") {
          return (
            <label key={key} className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={!!current}
                onChange={(e) => onChange({ ...value, [key]: e.target.checked })}
              />
              <span className="text-[12px]">{key}</span>
            </label>
          );
        }

        return (
          <label key={key} className="block">
            <span className="text-[11px] uppercase font-mono text-ink-500 tracking-wider">{key}</span>
            <input
              type="text"
              inputMode={kind === "number" || kind === "bigint" ? "numeric" : "text"}
              value={current == null ? "" : String(current)}
              onChange={(e) => onChange({ ...value, [key]: e.target.value })}
              className="mt-1 w-full h-8 px-2 rounded-md border border-ink-200 text-[13px] font-mono"
            />
          </label>
        );
      })}
    </div>
  );
}
