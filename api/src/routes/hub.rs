use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use db::{NewRun, Organization};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    payment::{parse_x_payment_header, PaymentRequirements},
    state::AppState,
    types::PublishHubRequest,
};

pub async fn list(State(state): State<AppState>) -> AppResult<Json<Value>> {
    // List all public workflows across all orgs.
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

/// `GET /v1/hub/:id/payment_info` — public endpoint; returns the fee
/// requirements for a workflow. No auth required so clients can discover the fee.
pub async fn payment_info(
    State(state): State<AppState>,
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

    let reqs = build_payment_requirements(&state, &wf, workflow_id);
    Ok(Json(json!(reqs)))
}

pub async fn call(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(workflow_id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Json<Value>> {
    let wf = state
        .db
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if !wf.is_public {
        return Err(AppError::NotFound);
    }

    // x402-fee-is-lamports-for-mvp: fee_per_exec_usdc is reused as a lamports
    // amount for x402 payment gating. When it is None or zero, the workflow is
    // free to call and no payment header is required.
    let fee_lamports = wf.fee_per_exec_usdc.unwrap_or(0);
    if fee_lamports > 0 {
        let reqs = build_payment_requirements(&state, &wf, workflow_id);
        return call_with_payment(&state, &headers, reqs, org.id, workflow_id).await;
    }

    // No fee — free workflow: create run directly.
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_payment_requirements(
    state: &AppState,
    wf: &db::Workflow,
    workflow_id: Uuid,
) -> PaymentRequirements {
    // x402-fee-is-lamports-for-mvp: treat fee_per_exec_usdc integer as lamports.
    let amount_lamports = wf.fee_per_exec_usdc.unwrap_or(0) as u64;
    PaymentRequirements {
        network: "solana-devnet".to_string(),
        asset: "SOL".to_string(),
        amount_lamports,
        recipient: state.treasury_pubkey.clone(),
        memo: format!("hub-call:{}", workflow_id),
    }
}

/// Handle the full x402 flow for paid workflow calls.
///
/// Flow:
/// 1. No X-PAYMENT header → 402 with payment requirements.
/// 2. Malformed header  → 402 with error.
/// 3. Replay (already verified signature) → 409.
/// 4. On-chain verification fails → 402 with error.
/// 5. Success → create payment + run, return run_id.
async fn call_with_payment(
    state: &AppState,
    headers: &HeaderMap,
    reqs: PaymentRequirements,
    org_id: Uuid,
    workflow_id: Uuid,
) -> AppResult<Json<Value>> {
    // 1. Read the X-PAYMENT header (case-insensitive via axum's HeaderMap).
    let raw_header = headers
        .get("x-payment")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let payment_header = match raw_header {
        None => {
            return Err(AppError::PaymentRequired(json!({
                "x402": "1",
                "payment": reqs,
            })));
        }
        Some(h) => h,
    };

    // 2. Parse: solana:devnet:tx:<signature>
    let signature = match parse_x_payment_header(&payment_header) {
        Some(sig) => sig,
        None => {
            return Err(AppError::PaymentRequired(json!({
                "x402": "1",
                "error": format!(
                    "invalid X-PAYMENT header; expected solana:devnet:tx:<sig>, got {:?}",
                    payment_header
                ),
                "payment": reqs,
            })));
        }
    };

    // 3. Replay guard.
    if let Some(existing) = state.db.get_payment_by_signature(&signature).await? {
        if existing.status == "verified" {
            return Err(AppError::PaymentReplay);
        }
    }

    // 4. On-chain RPC verification.
    let verified = match state.payment_verifier.verify(&signature, &reqs).await {
        Ok(v) => v,
        Err(e) => {
            return Err(AppError::PaymentRequired(json!({
                "x402": "1",
                "error": e.to_string(),
                "payment": reqs,
            })));
        }
    };

    // 5. Persist payment (status='pending') then create run, then mark verified.
    let payment = state
        .db
        .create_payment(
            workflow_id,
            &verified.payer,
            &reqs.recipient,
            &reqs.network,
            verified.amount_lamports as i64,
            &signature,
        )
        .await?;

    let run = state
        .db
        .create_run(NewRun {
            workflow_id,
            org_id,
            triggered_by: "x402".into(),
        })
        .await?;

    state
        .db
        .mark_payment_verified(payment.id, run.run_id)
        .await?;

    let _ = state.manual_triggers.send(run.run_id);

    Ok(Json(json!({
        "run_id": run.run_id,
        "status": run.status,
        "payment_signature": signature,
    })))
}
