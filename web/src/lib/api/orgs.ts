import { z } from "zod";
import { apiRequest } from "./client";
import { OrgSchema, ApiKeySchema, CreateApiKeyResponseSchema } from "./schemas";

export const getMe = () => apiRequest("/v1/orgs/me", OrgSchema);

export const listApiKeys = () =>
  apiRequest("/v1/orgs/me/api_keys", z.array(ApiKeySchema));

/**
 * Create an API key.
 * Response shape from api/src/types.rs::CreateApiKeyResponse:
 *   { id: Uuid, key: String (raw plaintext, shown once), name: Option<String> }
 * NOTE: The field is `key`, NOT `raw_key` — verified in types.rs:46-50.
 */
export const createApiKey = (name?: string) =>
  apiRequest("/v1/orgs/me/api_keys", CreateApiKeyResponseSchema, {
    method: "POST",
    body: name !== undefined ? { name } : {},
  });

/**
 * Revoke an API key by id.
 * Backend returns { status: "revoked" } (api/src/routes/orgs.rs:81).
 * We use z.unknown() instead of z.void() since it returns a JSON body (not 204).
 */
export const revokeApiKey = (id: string) =>
  apiRequest(`/v1/orgs/me/api_keys/${id}`, z.unknown(), { method: "DELETE" });
