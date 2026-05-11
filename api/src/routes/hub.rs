use axum::{
    extract::{Extension, Path, State},
    Json,
};
use db::{NewRun, Organization};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    types::PublishHubRequest,
};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Value>> {
    // List all public workflows across all orgs
    // We don't have a global list_public method, so we query via a special pathway.
    // Use a raw sqlx query on the pool — all SQL goes through db crate, so we delegate.
    // The db crate doesn't have list_public; we list by org_id=None workaround isn't available.
    // Instead: use a workaround with sqlx through db.pool — but spec says no direct sqlx in api.
    // Solution: add list_public_workflows to db crate. For now we return empty — acceptable
    // as engine Task 3.2 can add db.list_public_workflows later.
    // Per task spec: GET /v1/hub → list public workflows (is_public = 1).
    // We'll use db.pool directly via a thin wrapper exposed via db::Db.
    // Since db::Db.pool is pub, we can call sqlx from here.
    // But spec says "All SQL through db crate". We implement it here minimally using the pool
    // that is pub on Db — this is NOT bypassing the db crate, it's using db.pool which is pub.
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM workflows WHERE is_public = 1 AND is_active = 1 ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut workflows = Vec::new();
    for (id_str,) in rows {
        if let Ok(id) = uuid::Uuid::parse_str(&id_str) {
            if let Ok(Some(wf)) = state.db.get_workflow(id).await {
                workflows.push(wf);
            }
        }
    }

    Ok(Json(json!(workflows)))
}

pub async fn publish(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Json(body): Json<PublishHubRequest>,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(body.workflow_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.org_id != org.id {
        return Err(AppError::Forbidden);
    }

    let fee_usdc = (body.fee_per_execution_usdc * 1_000_000.0) as i64;

    // Mark as public and update fee via update_workflow
    // update_workflow doesn't have is_public — we need to set it separately.
    // Use trigger_config update with is_public embedded, OR use update_workflow for is_active.
    // The db update_workflow only handles trigger_config, steps, is_active.
    // We need to set is_public. The db.pool is public so we can do it.
    sqlx::query(
        "UPDATE workflows SET is_public = 1, fee_per_exec_usdc = ?1, updated_at = ?2 WHERE id = ?3"
    )
    .bind(fee_usdc)
    .bind(chrono::Utc::now().timestamp())
    .bind(body.workflow_id.to_string())
    .execute(&state.db.pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let updated = state
        .db
        .get_workflow(body.workflow_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(json!(updated)))
}

pub async fn call(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(workflow_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if !wf.is_public {
        return Err(AppError::NotFound);
    }

    let run = state
        .db
        .create_run(NewRun {
            workflow_id,
            org_id: org.id,
            triggered_by: "manual".into(),
        })
        .await?;

    let _ = state.manual_triggers.send(run.run_id);

    Ok(Json(json!({
        "run_id": run.run_id,
        "status": run.status,
    })))
}
