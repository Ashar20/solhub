use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use db::{Db, NewRun, NewWorkflow, Organization};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn fixture() -> (axum::Router, Db, Organization, String) {
    let db = Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    let org = db
        .create_org("approval-test", Some("11111111111111111111111111111111"))
        .await
        .unwrap();
    db.grant_credits(org.id, 1000, "test_fixture", None)
        .await
        .unwrap();
    let raw_key = format!("sk_{}", Uuid::new_v4().simple());
    let mut h = Sha256::new();
    h.update(raw_key.as_bytes());
    let kh = hex::encode(h.finalize());
    db.create_api_key(org.id, &kh, Some("test")).await.unwrap();
    let plugins = std::sync::Arc::new(engine::plugins::PluginRegistry::default());
    let rpc = std::sync::Arc::new(
        solana_client::nonblocking::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        ),
    );
    let payment_verifier = api::payment::PaymentVerifier::new(rpc);
    let treasury = "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb".to_string();
    let state = api::state::AppState::new(db.clone(), plugins, payment_verifier, treasury);
    let app = api::app::build_router(state);
    (app, db, org, raw_key)
}

fn auth(raw_key: &str) -> String {
    format!("Bearer {raw_key}")
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Create a workflow and a run directly in the DB, set the run to WaitingApproval.
async fn create_waiting_run(db: &Db, org_id: Uuid) -> (Uuid, Uuid) {
    let wf = db
        .create_workflow(NewWorkflow {
            org_id,
            name: "approval-test-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: serde_json::json!({}),
            steps: serde_json::json!([]),
            is_public: false,
            fee_per_exec_usdc: None,
        })
        .await
        .unwrap();

    let run = db
        .create_run(NewRun {
            workflow_id: wf.id,
            org_id,
            triggered_by: "manual".into(),
        })
        .await
        .unwrap();

    db.update_run_status(run.run_id, "WaitingApproval", None)
        .await
        .unwrap();

    (wf.id, run.run_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approve_run_transitions_to_resumed() {
    let (app, db, org, raw_key) = fixture().await;
    let (_wf_id, run_id) = create_waiting_run(&db, org.id).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{run_id}/approve"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["status"], "Resumed", "run should be Resumed after approval");
}

#[tokio::test]
async fn reject_run_transitions_to_failed() {
    let (app, db, org, raw_key) = fixture().await;
    let (_wf_id, run_id) = create_waiting_run(&db, org.id).await;

    let body = serde_json::json!({ "reason": "too risky" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{run_id}/reject"))
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["status"], "Failed");
    let err = j["error_message"].as_str().unwrap_or("");
    assert!(
        err.contains("rejected") && err.contains("too risky"),
        "error_message should mention rejection reason, got: {err}"
    );
}

#[tokio::test]
async fn approve_run_returns_400_when_not_waiting() {
    let (app, db, org, raw_key) = fixture().await;
    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "not-waiting-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: serde_json::json!({}),
            steps: serde_json::json!([]),
            is_public: false,
            fee_per_exec_usdc: None,
        })
        .await
        .unwrap();
    let run = db
        .create_run(NewRun {
            workflow_id: wf.id,
            org_id: org.id,
            triggered_by: "manual".into(),
        })
        .await
        .unwrap();
    // Run is still Pending — not WaitingApproval.

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{}/approve", run.run_id))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn approve_run_returns_404_for_other_org() {
    let (app, db, org, raw_key) = fixture().await;
    let (_wf_id, run_id) = create_waiting_run(&db, org.id).await;

    // Create a second org with its own key.
    let org2 = db.create_org("other-org", None).await.unwrap();
    let raw_key2 = format!("sk_{}", Uuid::new_v4().simple());
    let mut h = Sha256::new();
    h.update(raw_key2.as_bytes());
    let kh2 = hex::encode(h.finalize());
    db.create_api_key(org2.id, &kh2, None).await.unwrap();
    let _ = raw_key; // used to satisfy fixture

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{run_id}/approve"))
                .header("Authorization", auth(&raw_key2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
