CREATE TABLE payments (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL REFERENCES workflows(id),
    payer_pubkey    TEXT NOT NULL,
    recipient       TEXT NOT NULL,
    network         TEXT NOT NULL,
    amount_lamports INTEGER NOT NULL,
    signature       TEXT NOT NULL UNIQUE,
    status          TEXT NOT NULL,         -- 'pending' | 'verified' | 'rejected'
    run_id          TEXT,                  -- set when the workflow is triggered after payment
    error           TEXT,
    created_at      INTEGER NOT NULL,
    verified_at     INTEGER
);
CREATE INDEX idx_payments_workflow_id ON payments(workflow_id);
CREATE INDEX idx_payments_signature ON payments(signature);
