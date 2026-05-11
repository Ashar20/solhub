use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Timestamp helpers
// ---------------------------------------------------------------------------

/// Convert a unix-epoch i64 stored in SQLite to a UTC DateTime.
pub(crate) fn ts_to_dt(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
}

/// Current time as unix epoch i64.
pub(crate) fn now_ts() -> i64 {
    Utc::now().timestamp()
}

// ---------------------------------------------------------------------------
// Public row structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub wallet_address: Option<String>,
    pub credits_usdc: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub org_id: Uuid,
    pub key_hash: String,
    pub name: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: Value,
    pub steps: Value,
    pub is_active: bool,
    pub is_public: bool,
    pub onchain_pda: Option<String>,
    pub fee_per_exec_usdc: Option<i64>,
    pub execution_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub org_id: Uuid,
    pub status: String,
    pub triggered_by: String,
    pub steps_log: Value,
    pub slot: Option<i64>,
    pub signature: Option<String>,
    pub fee_lamports: Option<i64>,
    pub jito_tip_lamports: Option<i64>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflow {
    pub org_id: Uuid,
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: Value,
    pub steps: Value,
    pub is_public: bool,
    pub fee_per_exec_usdc: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRun {
    pub workflow_id: Uuid,
    pub org_id: Uuid,
    pub triggered_by: String,
}
