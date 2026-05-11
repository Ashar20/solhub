import { apiRequest } from "./client";
import { AnalyticsSchema } from "./schemas";

/**
 * Fetch aggregated analytics for the current org.
 * GET /v1/analytics — no query params; backend computes totals over all runs
 * (api/src/routes/analytics.rs). No range/from/to filtering exists in current impl.
 */
export const getAnalytics = () =>
  apiRequest("/v1/analytics", AnalyticsSchema);
