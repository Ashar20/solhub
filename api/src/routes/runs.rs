use axum::{
    extract::{Extension, Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use db::Organization;
use futures_util::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    types::ListRunsQuery,
};

#[derive(Debug, Deserialize)]
pub struct RejectBody {
    pub reason: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Query(q): Query<ListRunsQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(50);
    let runs = state
        .db
        .list_runs(org.id, q.workflow_id, q.status.as_deref(), limit)
        .await?;
    Ok(Json(json!(runs)))
}

pub async fn get_one(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(run_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let run = state
        .db
        .get_run(run_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if run.org_id != org.id {
        return Err(AppError::NotFound);
    }

    Ok(Json(json!(run)))
}

pub async fn approve_run(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(run_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let run = state
        .db
        .get_run(run_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if run.org_id != org.id {
        return Err(AppError::NotFound);
    }

    if run.status != "WaitingApproval" {
        return Err(AppError::BadRequest(format!(
            "run is in status '{}', not 'WaitingApproval'",
            run.status
        )));
    }

    let updated = state.db.approve_run(run_id).await?;
    Ok(Json(json!(updated)))
}

pub async fn reject_run(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(run_id): Path<Uuid>,
    Json(body): Json<RejectBody>,
) -> AppResult<Json<Value>> {
    let run = state
        .db
        .get_run(run_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if run.org_id != org.id {
        return Err(AppError::NotFound);
    }

    if run.status != "WaitingApproval" {
        return Err(AppError::BadRequest(format!(
            "run is in status '{}', not 'WaitingApproval'",
            run.status
        )));
    }

    let reason = body.reason.as_deref().unwrap_or("no reason provided");
    let updated = state.db.reject_run(run_id, reason).await?;
    Ok(Json(json!(updated)))
}

pub async fn stream_run_logs(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Extension(org): Extension<Organization>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    let db = state.db.clone();
    let stream = async_stream::stream! {
        let mut last_log_len = 0usize;
        loop {
            let run = match db.get_run(run_id).await {
                Ok(Some(r)) if r.org_id == org.id => r,
                _ => break,
            };
            if let Some(arr) = run.steps_log.as_array() {
                while last_log_len < arr.len() {
                    let payload = serde_json::to_string(&arr[last_log_len]).unwrap_or_default();
                    yield Ok(Event::default().event("step_log").data(payload));
                    last_log_len += 1;
                }
            }
            if matches!(run.status.as_str(), "Confirmed" | "Failed" | "Skipped") {
                let payload = serde_json::to_string(&run).unwrap_or_default();
                yield Ok(Event::default().event("run_complete").data(payload));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
