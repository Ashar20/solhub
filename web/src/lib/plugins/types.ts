import { z } from "zod";

export type ActionType = "read" | "transaction" | "notification" | "logic";
export type PluginCategory = "swap" | "lend" | "stake" | "perps" | "oracle" | "lp" | "nft" | "notify" | "logic";
export type PluginStatus = "real" | "stub";

export interface PluginAction {
  /** Action id — used as `step.action` in the workflow JSON. */
  id: string;
  name: string;
  description: string;
  type: ActionType;
  /** Zod schema for the `params` object. */
  schema: z.ZodTypeAny;
  /** Default values used to seed the inspector form when a node is added. */
  defaults: Record<string, unknown>;
}

export interface PluginDef {
  /** Plugin id — used as `step.plugin` in the workflow JSON. */
  id: string;
  name: string;
  category: PluginCategory;
  /** Whether the engine implementation is real or returns NotImplemented. */
  status: PluginStatus;
  actions: PluginAction[];
}
