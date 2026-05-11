use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use db::Organization;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    payment::{parse_x_payment_header, PaymentRequirements},
    state::AppState,
};

// ---------------------------------------------------------------------------
// Env helpers
// ---------------------------------------------------------------------------

fn lamports_per_credit() -> u64 {
    std::env::var("SOLHUB_LAMPORTS_PER_CREDIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000)
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct CreditsResponse {
    pub balance: i64,
    pub recent_ledger: Vec<db::LedgerEntry>,
}

#[derive(Serialize)]
pub struct TopupInfoResponse {
    pub network: String,
    pub asset: String,
    pub amount_lamports: u64,
    pub recipient: String,
    pub memo: String,
    pub lamports_per_credit: u64,
}

#[derive(Deserialize)]
pub struct AdminGrantRequest {
    pub org_id: Uuid,
    pub amount: i64,
    pub reason: String,
}

#[derive(Serialize)]
pub struct TopupResponse {
    pub credits_granted: i64,
    pub new_balance: i64,
    pub payment_id: Uuid,
    pub signature: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /v1/orgs/me/credits`
pub async fn get_credits(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
) -> AppResult<Json<CreditsResponse>> {
    let ledger = state.db.list_ledger(org.id, 20).await?;
    Ok(Json(CreditsResponse {
        balance: org.credits_usdc,
        recent_ledger: ledger,
    }))
}

/// `GET /v1/orgs/me/credits/topup_info`
pub async fn topup_info(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
) -> AppResult<Json<TopupInfoResponse>> {
    let lpc = lamports_per_credit();
    let memo = format!("topup:{}", org.id);
    Ok(Json(TopupInfoResponse {
        network: "solana-devnet".into(),
        asset: "SOL".into(),
        amount_lamports: lpc,
        recipient: state.treasury_pubkey.clone(),
        memo,
        lamports_per_credit: lpc,
    }))
}

/// `POST /v1/orgs/me/credits/topup`
///
/// Expects header: `X-PAYMENT: solana:devnet:tx:<signature>`
pub async fn topup(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    headers: HeaderMap,
) -> AppResult<Json<TopupResponse>> {
    let lpc = lamports_per_credit();

    // Build payment requirements (minimum = 1 credit worth)
    let reqs = PaymentRequirements {
        network: "solana-devnet".into(),
        asset: "SOL".into(),
        amount_lamports: lpc,
        recipient: state.treasury_pubkey.clone(),
        memo: format!("topup:{}", org.id),
    };

    // Read X-PAYMENT header
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

    // Replay guard
    if let Some(existing) = state.db.get_payment_by_signature(&signature).await? {
        if existing.status == "verified" {
            return Err(AppError::PaymentReplay);
        }
    }

    // On-chain verification
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

    // Calculate credits from lamports received
    let credits_granted = (verified.amount_lamports / lpc) as i64;
    if credits_granted < 1 {
        return Err(AppError::BadRequest(format!(
            "payment amount {} lamports grants 0 credits (min {} lamports per credit)",
            verified.amount_lamports, lpc
        )));
    }

    // Assign a stable payment UUID for this topup (used in the ledger).
    let payment_id = Uuid::new_v4();

    // Grant credits — the ledger entry records the payment_id and signature context.
    let new_balance = state
        .db
        .grant_credits(org.id, credits_granted, "topup", Some(payment_id))
        .await?;

    Ok(Json(TopupResponse {
        credits_granted,
        new_balance,
        payment_id,
        signature,
    }))
}

/// `POST /v1/orgs/me/credits/grant` — admin only, gated by `SOLHUB_ADMIN_TOKEN`.
pub async fn admin_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminGrantRequest>,
) -> AppResult<Json<Value>> {
    let expected_token = std::env::var("SOLHUB_ADMIN_TOKEN").unwrap_or_default();
    if expected_token.is_empty() {
        return Err(AppError::Forbidden);
    }

    let provided = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected_token {
        return Err(AppError::Forbidden);
    }

    // Verify org exists
    let _org = state
        .db
        .get_org(body.org_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let new_balance = state
        .db
        .grant_credits(body.org_id, body.amount, &body.reason, None)
        .await?;

    Ok(Json(json!({
        "org_id": body.org_id,
        "amount_granted": body.amount,
        "new_balance": new_balance,
    })))
}
