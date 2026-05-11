CREATE TABLE api_keys (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash        TEXT NOT NULL UNIQUE,
    name            TEXT,
    last_used_at    INTEGER,
    created_at      INTEGER NOT NULL,
    revoked_at      INTEGER
);
CREATE INDEX idx_api_keys_org_id ON api_keys(org_id);
