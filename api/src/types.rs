use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub trigger: Value,
    pub steps: Vec<Value>,
    pub fee_per_execution_usdc: Option<f64>,
    pub is_public: Option<bool>,
}

#[derive(Serialize)]
pub struct CreateWorkflowResponse {
    pub workflow_id: Uuid,
    pub status: String,
    pub next_run: Option<String>,
    pub onchain_pda: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWorkflowRequest {
    pub trigger: Option<Value>,
    pub steps: Option<Vec<Value>>,
    pub is_active: Option<bool>,
}

#[derive(Deserialize)]
pub struct TriggerWorkflowRequest {
    pub param_overrides: Option<Value>,
}

#[derive(Serialize)]
pub struct TriggerWorkflowResponse {
    pub run_id: Uuid,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct WebhookPayload {
    pub trigger_data: Value,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub run_id: Uuid,
    pub status: String,
}

#[derive(Serialize)]
pub struct AnalyticsResponse {
    pub total_executions: i64,
    pub successful: i64,
    pub failed: i64,
    pub total_fee_lamports: i64,
}

#[derive(Deserialize)]
pub struct PublishHubRequest {
    pub workflow_id: Uuid,
    pub fee_per_execution_usdc: f64,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ListRunsQuery {
    pub workflow_id: Option<Uuid>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct ListWorkflowsQuery {
    pub active_only: Option<bool>,
}
