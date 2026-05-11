CREATE TABLE IF NOT EXISTS credit_ledger (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id),
    delta           INTEGER NOT NULL,
    reason          TEXT NOT NULL,
    run_id          TEXT,
    payment_id      TEXT,
    balance_after   INTEGER NOT NULL,
    created_at      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_credit_ledger_org ON credit_ledger(org_id, created_at DESC);
