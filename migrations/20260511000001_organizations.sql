CREATE TABLE organizations (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    wallet_address  TEXT,
    credits_usdc    INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL
);
