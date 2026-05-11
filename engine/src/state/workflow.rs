use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub trigger: TriggerConfig,
    pub steps: Vec<WorkflowStep>,
    pub is_active: bool,
    pub onchain_pda: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Workflow {
    pub fn new(
        org_id: Uuid,
        name: String,
        trigger: TriggerConfig,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            org_id,
            name,
            trigger,
            steps,
            is_active: true,
            onchain_pda: None,
            created_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerConfig {
    Cron { schedule: String },
    AccountWatch {
        account: String,
        condition: WatchCondition,
    },
    Webhook { secret: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchCondition {
    BalanceAbove { lamports: u64 },
    BalanceBelow { lamports: u64 },
    DataChanges,
    ProgramLog { pattern: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub plugin: String,
    pub action: String,
    pub params: serde_json::Value,
    pub condition: Option<String>,
    pub on_error: OnError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnError {
    Abort,
    Skip,
    Retry { max_attempts: u8 },
}
