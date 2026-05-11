// x402 payment-gating integration tests.
//
// Offline tests use an in-memory SQLite database and do NOT call the Solana RPC
// (the PaymentVerifier is only invoked when a valid-format signature header is
// present AND the workflow has fee > 0).
//
// The live-devnet test is marked #[ignore] so the default `cargo test` run
// stays offline.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use api::{app::build_router, payment::PaymentVerifier, state::AppState};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

async fn make_state(db: db::Db) -> AppState {
    let plugins = Arc::new(engine::plugins::PluginRegistry::default());
    let rpc = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
    let verifier = PaymentVerifier::new(rpc);
    let treasury = "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb".to_string();
    AppState::new(db, plugins, verifier, treasury)
}

async fn test_db() -> db::Db {
    let db = db::Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    db
}

/// Create an org + API key, return (org, raw_key).
async fn create_org_with_key(db: &db::Db) -> (db::Organization, String) {
    let org = db.create_org("test", None).await.unwrap();
    let raw_key = format!("sk_{}", Uuid::new_v4().simple());
    let mut h = Sha256::new();
    h.update(raw_key.as_bytes());
    let kh = hex::encode(h.finalize());
    db.create_api_key(org.id, &kh, Some("test")).await.unwrap();
    (org, raw_key)
}

/// Create a public workflow with the given fee (stored in fee_per_exec_usdc).
async fn create_public_workflow(db: &db::Db, org_id: Uuid, fee: Option<i64>) -> db::Workflow {
    let wf = db
        .create_workflow(db::NewWorkflow {
            org_id,
            name: "hub-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([]),
            is_public: true,
            fee_per_exec_usdc: fee,
        })
        .await
        .unwrap();

    // Set is_public = 1 (create_workflow may not expose that flag directly)
    sqlx::query("UPDATE workflows SET is_public = 1 WHERE id = ?1")
        .bind(wf.id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();

    wf
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

fn bearer(raw_key: &str) -> String {
    format!("Bearer {raw_key}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Workflow with fee_per_exec_usdc = None (or 0) does NOT require a payment
/// header — the call should succeed immediately.
#[tokio::test]
async fn hub_call_with_zero_fee_doesnt_require_payment() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_public_workflow(&db, org.id, None).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/hub/{}/call", wf.id))
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "expected 200 for free workflow");
    let body = body_json(resp).await;
    assert!(body.get("run_id").is_some(), "expected run_id in response");
}

/// Workflow with fee > 0 and NO X-PAYMENT header → 402 with correct body fields.
#[tokio::test]
async fn hub_call_returns_402_without_payment_header() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_public_workflow(&db, org.id, Some(5_000_000)).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/hub/{}/call", wf.id))
                .header("Authorization", bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
    let body = body_json(resp).await;
    assert_eq!(body["x402"], "1", "body must contain x402 field");
    let payment = &body["payment"];
    assert_eq!(payment["network"], "solana-devnet");
    assert_eq!(payment["asset"], "SOL");
    assert_eq!(payment["amount_lamports"], 5_000_000_i64);
    assert_eq!(
        payment["recipient"],
        "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"
    );
}

/// Malformed `X-PAYMENT` header → 402 with error message.
#[tokio::test]
async fn hub_call_returns_402_for_invalid_signature_format() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_public_workflow(&db, org.id, Some(1_000)).await;

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/hub/{}/call", wf.id))
                .header("Authorization", bearer(&raw_key))
                .header("x-payment", "garbage-not-a-solana-payment-header")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
    let body = body_json(resp).await;
    let err = body["error"].as_str().unwrap_or("");
    assert!(
        err.contains("invalid X-PAYMENT header"),
        "error message should explain format problem, got: {err}"
    );
}

/// Pre-seed a verified payment row with a given signature.
/// Re-using the same signature yields 409 Conflict.
#[tokio::test]
async fn hub_call_rejects_replay() {
    let db = test_db().await;
    let (org, raw_key) = create_org_with_key(&db).await;
    let wf = create_public_workflow(&db, org.id, Some(1_000)).await;

    // Pre-seed a payment that is already 'verified'.
    let existing_sig = "ReplaySig1111111111111111111111111111111111111111111111111111111111111";
    let payment = db
        .create_payment(
            wf.id,
            "SomePayer11111111111111111111111111111111",
            "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb",
            "solana-devnet",
            1_000,
            existing_sig,
        )
        .await
        .unwrap();

    // Create a run and mark payment verified so status = 'verified'.
    let run = db
        .create_run(db::NewRun {
            workflow_id: wf.id,
            org_id: org.id,
            triggered_by: "x402".into(),
        })
        .await
        .unwrap();
    db.mark_payment_verified(payment.id, run.run_id)
        .await
        .unwrap();

    let state = make_state(db).await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/hub/{}/call", wf.id))
                .header("Authorization", bearer(&raw_key))
                .header(
                    "x-payment",
                    format!("solana:devnet:tx:{}", existing_sig),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT, "replay must return 409");
    let body = body_json(resp).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("replay"),
        "error message should mention replay, got: {:?}",
        body
    );
}

/// `GET /v1/hub/:id/payment_info` returns the payment requirements.
/// This endpoint is public (no auth required).
#[tokio::test]
async fn payment_info_endpoint_returns_requirements() {
    let db = test_db().await;
    let (org, _raw_key) = create_org_with_key(&db).await;
    let wf = create_public_workflow(&db, org.id, Some(2_500_000)).await;

    let state = make_state(db).await;
    let app = build_router(state);

    // No Authorization header — must succeed (public endpoint).
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/hub/{}/payment_info", wf.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["network"], "solana-devnet");
    assert_eq!(body["asset"], "SOL");
    assert_eq!(body["amount_lamports"], 2_500_000_i64);
    assert_eq!(
        body["recipient"],
        "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb"
    );
    let memo = body["memo"].as_str().unwrap_or("");
    assert!(
        memo.starts_with("hub-call:"),
        "memo should start with hub-call:, got: {memo}"
    );
}

// ---------------------------------------------------------------------------
// Live-devnet integration test — skipped in CI
// ---------------------------------------------------------------------------

/// Verify a real Solana devnet payment signature end-to-end.
///
/// Run with: `cargo test -p api -- --ignored live_devnet_payment_verification`
///
/// Pre-requisite: set SOLHUB_TEST_SIG to a recent (<10 min) devnet tx signature
/// where SOL was transferred to FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb.
#[tokio::test]
#[ignore]
async fn live_devnet_payment_verification() {
    let sig = std::env::var("SOLHUB_TEST_SIG")
        .expect("set SOLHUB_TEST_SIG to a recent devnet tx signature");

    let treasury = "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb";
    let rpc = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
    let verifier = PaymentVerifier::new(rpc);
    let reqs = api::payment::PaymentRequirements {
        network: "solana-devnet".to_string(),
        asset: "SOL".to_string(),
        amount_lamports: 1, // just check it transferred something
        recipient: treasury.to_string(),
        memo: "hub-call:test".to_string(),
    };

    let result = verifier.verify(&sig, &reqs).await;
    assert!(
        result.is_ok(),
        "live verification failed: {:?}",
        result.err()
    );
    let v = result.unwrap();
    println!("payer={} amount={}L", v.payer, v.amount_lamports);
    assert!(v.amount_lamports >= 1);
}
