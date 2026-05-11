use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use db::{NewRun, NewWorkflow, Organization};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    types::{
        CreateWorkflowRequest, CreateWorkflowResponse, ListWorkflowsQuery, TriggerWorkflowRequest,
        TriggerWorkflowResponse, UpdateWorkflowRequest,
    },
};

const KNOWN_TRIGGER_TYPES: &[&str] = &["cron", "webhook", "manual", "price_alert", "on_chain"];

pub async fn create(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Json(body): Json<CreateWorkflowRequest>,
) -> AppResult<Json<CreateWorkflowResponse>> {
    let trigger_type = body
        .trigger
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("trigger.type is required".into()))?;

    if !KNOWN_TRIGGER_TYPES.contains(&trigger_type) {
        return Err(AppError::BadRequest(format!(
            "unknown trigger type: {trigger_type}"
        )));
    }

    let fee = body
        .fee_per_execution_usdc
        .map(|f| (f * 1_000_000.0) as i64);

    let steps_value: Value = serde_json::to_value(&body.steps)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let wf = state
        .db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: body.name,
            trigger_type: trigger_type.to_string(),
            trigger_config: body.trigger,
            steps: steps_value,
            is_public: body.is_public.unwrap_or(false),
            fee_per_exec_usdc: fee,
        })
        .await?;

    Ok(Json(CreateWorkflowResponse {
        workflow_id: wf.id,
        status: "created".into(),
        next_run: None,
        onchain_pda: wf.onchain_pda,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Query(q): Query<ListWorkflowsQuery>,
) -> AppResult<Json<Value>> {
    let active_only = q.active_only.unwrap_or(false);
    let workflows = state.db.list_workflows(org.id, active_only).await?;
    Ok(Json(json!(workflows)))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.org_id != org.id {
        return Err(AppError::NotFound);
    }

    Ok(Json(json!(wf)))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateWorkflowRequest>,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.org_id != org.id {
        return Err(AppError::NotFound);
    }

    let steps_value: Option<Value> = body
        .steps
        .map(|s| serde_json::to_value(s).map_err(|e| AppError::Internal(e.to_string())))
        .transpose()?;

    let updated = state
        .db
        .update_workflow(id, body.trigger, steps_value, body.is_active)
        .await?;

    Ok(Json(json!(updated)))
}

pub async fn delete_wf(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.org_id != org.id {
        return Err(AppError::NotFound);
    }

    state.db.delete_workflow(id).await?;
    Ok(Json(json!({"status": "deleted"})))
}

pub async fn trigger(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(id): Path<Uuid>,
    body: Option<Json<TriggerWorkflowRequest>>,
) -> AppResult<Json<TriggerWorkflowResponse>> {
    let _ = body; // param_overrides reserved for engine Task 3.2

    let wf = state
        .db
        .get_workflow(id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.org_id != org.id {
        return Err(AppError::NotFound);
    }

    let run = state
        .db
        .create_run(NewRun {
            workflow_id: id,
            org_id: org.id,
            triggered_by: "manual".into(),
        })
        .await?;

    // Notify downstream engine listeners — ignore send errors (no receivers yet)
    let _ = state.manual_triggers.send(run.run_id);

    Ok(Json(TriggerWorkflowResponse {
        run_id: run.run_id,
        status: run.status,
    }))
}
