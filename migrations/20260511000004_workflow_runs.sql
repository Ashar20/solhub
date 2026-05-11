CREATE TABLE workflow_runs (
    run_id              TEXT PRIMARY KEY,
    workflow_id         TEXT NOT NULL REFERENCES workflows(id),
    org_id              TEXT NOT NULL REFERENCES organizations(id),
    status              TEXT NOT NULL,
    triggered_by        TEXT NOT NULL,
    steps_log           TEXT NOT NULL DEFAULT '[]',
    slot                INTEGER,
    signature           TEXT,
    fee_lamports        INTEGER,
    jito_tip_lamports   INTEGER,
    error_message       TEXT,
    started_at          INTEGER NOT NULL,
    completed_at        INTEGER
);
CREATE INDEX idx_runs_workflow_id ON workflow_runs(workflow_id);
CREATE INDEX idx_runs_org_id ON workflow_runs(org_id);
CREATE INDEX idx_runs_status ON workflow_runs(status);
CREATE INDEX idx_runs_started_at ON workflow_runs(started_at);
