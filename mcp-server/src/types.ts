import { z } from "zod";

export const triggerSchema = z.object({
  type: z.enum(["cron", "account_watch", "webhook"]),
  schedule: z.string().optional(),
  account: z.string().optional(),
  condition: z.record(z.unknown()).optional(),
  secret: z.string().optional(),
});

export const stepSchema = z.object({
  id: z.string().optional(),
  plugin: z.string(),
  action: z.string(),
  params: z.record(z.unknown()),
  condition: z.string().optional(),
  on_error: z.union([
    z.literal("Abort"),
    z.literal("Skip"),
    z.object({ Retry: z.object({ max_attempts: z.number() }) }),
  ]).optional(),
});

export type Trigger = z.infer<typeof triggerSchema>;
export type Step = z.infer<typeof stepSchema>;
