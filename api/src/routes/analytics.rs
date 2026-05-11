use axum::{
    extract::{Extension, State},
    Json,
};
use db::Organization;

use crate::{
    error::AppResult,
    state::AppState,
    types::AnalyticsResponse,
};

pub async fn get_analytics(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
) -> AppResult<Json<AnalyticsResponse>> {
    let all_runs = state.db.list_runs(org.id, None, None, 100_000).await?;

    let total_executions = all_runs.len() as i64;
    let successful = all_runs.iter().filter(|r| r.status == "Confirmed").count() as i64;
    let failed = all_runs.iter().filter(|r| r.status == "Failed").count() as i64;
    let total_fee_lamports = all_runs
        .iter()
        .filter_map(|r| r.fee_lamports)
        .sum::<i64>();

    Ok(Json(AnalyticsResponse {
        total_executions,
        successful,
        failed,
        total_fee_lamports,
    }))
}
