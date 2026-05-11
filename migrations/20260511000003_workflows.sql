CREATE TABLE workflows (
    id                  TEXT PRIMARY KEY,
    org_id              TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    trigger_type        TEXT NOT NULL,
    trigger_config      TEXT NOT NULL,
    steps               TEXT NOT NULL,
    is_active           INTEGER NOT NULL DEFAULT 1,
    is_public           INTEGER NOT NULL DEFAULT 0,
    onchain_pda         TEXT,
    fee_per_exec_usdc   INTEGER,
    execution_count     INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);
CREATE INDEX idx_workflows_org_id ON workflows(org_id);
CREATE INDEX idx_workflows_is_active ON workflows(is_active);
