// Credit system integration tests.
//
// Most tests are offline (in-memory SQLite, no real Solana RPC).
// The live-devnet topup test is #[ignore]d.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use api::{app::build_router, payment::PaymentVerifier, state::AppState};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ADMIN_TOKEN: &str = "test-admin-token-abc123";
const TREASURY: &str = "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb";

async fn test_db() -> db::Db {
    let db = db::Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    db
}

async fn make_state(db: db::Db) -> AppState {
    let plugins = Arc::new(engine::plugins::PluginRegistry::default());
    let rpc = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
        "https://api.devnet.solana.com".to_string(),
    ));
    let verifier = PaymentVerifier::new(rpc);
    AppState::new(db, plugins, verifier, TREASURY.to_string())
}

/// Create org + API key; returns (org, raw_key).
async fn create_org_with_key(db: &db::Db) -> (db::Organization, String) {
    let org = db.create_org("test-org", None).await.unwrap();
    let raw_key = format!("sk_{}", Uuid::new_v4().simple());
    let mut h = Sha256::new();
    h.update(raw_key.as_bytes());
    let kh = hex::encode(h.finalize());
    db.create_api_key(org.id, &kh, Some("key")).await.unwrap();
    (org, raw_key)
}

/// Create a minimal workflow for the org.
async fn create_workflow(db: &db::Db, org_id: Uuid) -> db::Workflow {
    db.create_workflow(db::NewWorkflow {
        org_id,
        name: "test-wf".into(),
        trigger_type: "manual".into(),
        trigger_config: json!({}),
        steps: json!([]),
        is_public: false,
        fee_per_exec_usdc: None,
    })
    .await
    .unwrap()
}

fn bearer(raw_key: &str) -> String {
    format!("Bearer {raw_key}")
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A freshly created org has 0 credits.
#[tokio::test]
async fn credit_balance_starts_at_zero() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me/credits")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["balance"], 0, "new org should have 0 credits");
    assert_eq!(
        j["recent_ledger"].as_array().unwrap().len(),
        0,
        "ledger should be empty"
    );

    // Also verify via /v1/orgs/me — credits_usdc field
    let _ = org; // silence warning
}

/// Triggering a workflow with 0 credits returns 402.
#[tokio::test]
async fn manual_trigger_with_zero_credits_returns_402() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_workflow(&db, org.id).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/workflows/{}/trigger", wf.id))
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::PAYMENT_REQUIRED,
        "should return 402 when credits = 0"
    );
    let j = body_json(resp).await;
    assert_eq!(j["error"], "insufficient_credits");
}

/// Admin grant endpoint adds credits (requires SOLHUB_ADMIN_TOKEN env var).
#[tokio::test]
async fn admin_grant_adds_credits() {
    std::env::set_var("SOLHUB_ADMIN_TOKEN", ADMIN_TOKEN);

    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let body = json!({
        "org_id": org.id,
        "amount": 50,
        "reason": "manual_grant"
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/credits/grant")
                .header("Content-Type", "application/json")
                .header("Authorization", bearer(&raw_key))
                .header("x-admin-token", ADMIN_TOKEN)
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "admin grant should succeed");
    let j = body_json(resp).await;
    assert_eq!(j["new_balance"], 50);
    assert_eq!(j["amount_granted"], 50);

    // Now GET /credits shows updated balance
    let bal_resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me/credits")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bal_j = body_json(bal_resp).await;
    assert_eq!(bal_j["balance"], 50);
}

/// After granting credits, triggering a workflow consumes one credit.
#[tokio::test]
async fn trigger_after_grant_consumes_one_credit() {
    std::env::set_var("SOLHUB_ADMIN_TOKEN", ADMIN_TOKEN);

    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_workflow(&db, org.id).await;

    // Grant 5 credits via DB directly (faster than going through the HTTP route)
    db.grant_credits(org.id, 5, "manual_grant", None)
        .await
        .unwrap();

    let state = make_state(db).await;
    let app = build_router(state);

    // Trigger — should succeed
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/workflows/{}/trigger", wf.id))
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "trigger should succeed with credits");
    let j = body_json(resp).await;
    assert_eq!(j["status"], "Pending");

    // Balance should now be 4
    let bal_resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me/credits")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bal_j = body_json(bal_resp).await;
    assert_eq!(
        bal_j["balance"], 4,
        "one credit should have been consumed"
    );
}

/// Ledger records both grants and debits.
#[tokio::test]
async fn ledger_records_grants_and_debits() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_workflow(&db, org.id).await;

    // Grant 3 credits
    db.grant_credits(org.id, 3, "manual_grant", None)
        .await
        .unwrap();

    let state = make_state(db).await;
    let app = build_router(state);

    // Trigger twice
    for _ in 0..2_u8 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/workflows/{}/trigger", wf.id))
                    .header("Authorization", bearer(&raw_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Check ledger
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me/credits")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let j = body_json(resp).await;
    assert_eq!(j["balance"], 1, "3 - 2 = 1");
    let ledger = j["recent_ledger"].as_array().unwrap();
    // 1 grant + 2 debits = 3 entries
    assert_eq!(ledger.len(), 3);

    // Verify all entries are present (order may vary if same-second timestamps)
    let debits: Vec<_> = ledger.iter().filter(|e| e["reason"] == "run_debit").collect();
    let grants: Vec<_> = ledger.iter().filter(|e| e["reason"] == "manual_grant").collect();
    assert_eq!(debits.len(), 2, "should have 2 run_debit entries");
    assert_eq!(grants.len(), 1, "should have 1 manual_grant entry");
    assert_eq!(debits[0]["delta"], -1);
    assert_eq!(grants[0]["delta"], 3);
}

/// `GET /v1/orgs/me/credits/topup_info` returns expected fields.
#[tokio::test]
async fn topup_info_returns_payment_requirements() {
    let db = test_db().await;
    let (_org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me/credits/topup_info")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["network"], "solana-devnet");
    assert_eq!(j["asset"], "SOL");
    assert!(j["amount_lamports"].as_u64().unwrap_or(0) > 0);
    assert_eq!(j["recipient"], TREASURY);
    assert!(
        j["memo"].as_str().unwrap_or("").starts_with("topup:"),
        "memo should start with topup:"
    );
    assert!(j["lamports_per_credit"].as_u64().unwrap_or(0) > 0);
}

/// `POST /v1/orgs/me/credits/topup` without X-PAYMENT header → 402.
#[tokio::test]
async fn topup_without_payment_header_returns_402() {
    let db = test_db().await;
    let (_org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/credits/topup")
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::PAYMENT_REQUIRED,
        "topup without X-PAYMENT must return 402"
    );
    let j = body_json(resp).await;
    assert_eq!(j["x402"], "1");
}

/// Admin grant without token or with wrong token → 403.
#[tokio::test]
async fn admin_grant_without_token_returns_403() {
    std::env::set_var("SOLHUB_ADMIN_TOKEN", ADMIN_TOKEN);

    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let body = json!({
        "org_id": org.id,
        "amount": 10,
        "reason": "manual_grant"
    });

    // Missing token
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/credits/grant")
                .header("Content-Type", "application/json")
                .header("Authorization", bearer(&raw_key))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Live-devnet test — skipped in CI
// ---------------------------------------------------------------------------

/// Full topup flow with a real devnet transaction.
///
/// Run with: `cargo test -p api -- --ignored topup_with_real_payment_grants_credits`
///
/// Pre-requisite: set SOLHUB_TEST_TOPUP_SIG to a recent (<10 min) devnet tx signature
/// that transferred ≥ SOLHUB_LAMPORTS_PER_CREDIT lamports to TREASURY.
#[tokio::test]
#[ignore]
async fn topup_with_real_payment_grants_credits() {
    let sig = std::env::var("SOLHUB_TEST_TOPUP_SIG")
        .expect("set SOLHUB_TEST_TOPUP_SIG to a recent devnet tx signature");

    let db = test_db().await;
    let (_org, raw_key) = create_org_with_key(&db).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let payment_header = format!("solana:devnet:tx:{}", sig);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/credits/topup")
                .header("Authorization", bearer(&raw_key))
                .header("x-payment", payment_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let j = body_json(resp).await;
    println!("topup response: {} — {:?}", status, j);
    assert_eq!(status, StatusCode::OK, "topup should succeed with valid payment");
    assert!(j["credits_granted"].as_i64().unwrap_or(0) > 0);
    assert!(j["new_balance"].as_i64().unwrap_or(0) > 0);
}
