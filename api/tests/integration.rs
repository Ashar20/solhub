use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn fixture() -> (axum::Router, db::Db, db::Organization, String) {
    let db = db::Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    let org = db
        .create_org("acme", Some("11111111111111111111111111111111"))
        .await
        .unwrap();
    let raw_key = format!("sk_{}", Uuid::new_v4().simple());
    let mut h = Sha256::new();
    h.update(raw_key.as_bytes());
    let kh = hex::encode(h.finalize());
    db.create_api_key(org.id, &kh, Some("test")).await.unwrap();
    let plugins = std::sync::Arc::new(engine::plugins::PluginRegistry::default());
    let state = api::state::AppState::new(db.clone(), plugins);
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

fn workflow_body() -> serde_json::Value {
    serde_json::json!({
        "name": "test-wf",
        "trigger": {"type": "cron", "schedule": "* * * * *"},
        "steps": [{"action": "noop"}]
    })
}

fn webhook_workflow_body(secret: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "wh-wf",
        "trigger": {"type": "webhook", "secret": secret},
        "steps": [{"action": "noop"}]
    })
}

async fn create_workflow(app: axum::Router, raw_key: &str) -> (axum::Router, Uuid) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(raw_key))
                .body(Body::from(serde_json::to_vec(&workflow_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    let id: Uuid = serde_json::from_value(j["workflow_id"].clone()).unwrap();
    (app, id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_workflow_returns_id_and_persists() {
    let (app, _db, _org, raw_key) = fixture().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&workflow_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert!(j["workflow_id"].is_string());
    assert_eq!(j["status"], "created");

    // verify list returns it
    let list_resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/workflows")
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_j = body_json(list_resp).await;
    assert_eq!(list_j.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn unauthorized_without_bearer() {
    let (app, _db, _org, _raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&workflow_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthorized_with_bad_bearer() {
    let (app, _db, _org, _raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer sk_bogus_key_does_not_exist")
                .body(Body::from(serde_json::to_vec(&workflow_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_workflow_returns_404_for_missing() {
    let (app, _db, _org, raw_key) = fixture().await;

    let missing_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/workflows/{missing_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_workflow_patches_is_active() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    let patch_body = serde_json::json!({"is_active": false});
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/workflows/{wf_id}"))
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&patch_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["is_active"], false);
}

#[tokio::test]
async fn delete_workflow_soft_deletes() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/workflows/{wf_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // GET still works but is_active=false
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/workflows/{wf_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let j = body_json(get_resp).await;
    assert_eq!(j["is_active"], false);
}

#[tokio::test]
async fn trigger_workflow_creates_a_run() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/workflows/{wf_id}/trigger"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert!(j["run_id"].is_string());
    assert_eq!(j["status"], "Pending");
}

#[tokio::test]
async fn list_runs_filters_by_workflow_id() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    // Create a run
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/workflows/{wf_id}/trigger"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // List with workflow_id filter
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs?workflow_id={wf_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_run_returns_404_for_missing() {
    let (app, _db, _org, raw_key) = fixture().await;
    let missing_id = Uuid::new_v4();

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{missing_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn webhook_valid_hmac_triggers_run() {
    let (app, _db, _org, raw_key) = fixture().await;
    let secret = "mysecret";

    // Create a webhook workflow
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(
                    serde_json::to_vec(&webhook_workflow_body(secret)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let j = body_json(create_resp).await;
    let wf_id: Uuid = serde_json::from_value(j["workflow_id"].clone()).unwrap();

    // Compute HMAC
    let payload = b"hello webhook";
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload);
    let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/webhooks/{wf_id}"))
                .header("Content-Type", "application/json")
                .header("X-SK-Signature", sig)
                .body(Body::from(payload.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert!(j["run_id"].is_string());
}

#[tokio::test]
async fn webhook_invalid_hmac_returns_401() {
    let (app, _db, _org, raw_key) = fixture().await;

    // Create a webhook workflow
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(
                    serde_json::to_vec(&webhook_workflow_body("realsecret")).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let j = body_json(create_resp).await;
    let wf_id: Uuid = serde_json::from_value(j["workflow_id"].clone()).unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/webhooks/{wf_id}"))
                .header("Content-Type", "application/json")
                .header("X-SK-Signature", "sha256=badhash")
                .body(Body::from(b"hello webhook".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn analytics_returns_zero_for_empty_org() {
    let (app, _db, _org, raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/analytics")
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["total_executions"], 0);
    assert_eq!(j["successful"], 0);
    assert_eq!(j["failed"], 0);
    assert_eq!(j["total_fee_lamports"], 0);
}

#[tokio::test]
async fn analytics_counts_runs_after_creation() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    // Trigger a run
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/workflows/{wf_id}/trigger"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/analytics")
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["total_executions"], 1);
}

#[tokio::test]
async fn orgs_me_returns_org_info() {
    let (app, _db, org, raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me")
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["id"], org.id.to_string());
    assert_eq!(j["name"], "acme");
}

#[tokio::test]
async fn create_api_key_returns_raw_once_and_persists_hash() {
    let (app, db, org, raw_key) = fixture().await;

    let body = serde_json::json!({"name": "new-key"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/api_keys")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    let new_raw_key = j["key"].as_str().unwrap().to_string();
    assert!(new_raw_key.starts_with("sk_"));

    // Verify hash is stored
    let mut h = Sha256::new();
    h.update(new_raw_key.as_bytes());
    let hash = hex::encode(h.finalize());
    let found = db.get_org_by_api_key_hash(&hash).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, org.id);
}

#[tokio::test]
async fn revoke_api_key_then_request_unauthorized() {
    let (app, _db, _org, raw_key) = fixture().await;

    // Create a second key
    let body = serde_json::json!({"name": "second-key"});
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/orgs/me/api_keys")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let j = body_json(create_resp).await;
    let new_raw_key = j["key"].as_str().unwrap().to_string();
    let key_id: Uuid = serde_json::from_value(j["id"].clone()).unwrap();

    // Revoke it
    let revoke_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/orgs/me/api_keys/{key_id}"))
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    // Now try to use the revoked key
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/orgs/me")
                .header("Authorization", auth(&new_raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn hub_list_works_without_auth() {
    let (app, _db, _org, _raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/hub")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert!(j.as_array().is_some());
}

#[tokio::test]
async fn hub_publish_marks_workflow_public() {
    let (app, _db, _org, raw_key) = fixture().await;
    let (app, wf_id) = create_workflow(app, &raw_key).await;

    let body = serde_json::json!({
        "workflow_id": wf_id,
        "fee_per_execution_usdc": 0.01,
        "description": "test",
        "tags": ["defi"]
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/hub/publish")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["is_public"], true);

    // Verify public list now returns it
    let hub_resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/hub")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hub_resp.status(), StatusCode::OK);
    let hub_j = body_json(hub_resp).await;
    assert_eq!(hub_j.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn rate_limit_429_after_burst() {
    let (app, _db, _org, raw_key) = fixture().await;

    let mut got_429 = false;
    for _ in 0..70 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/orgs/me")
                    .header("Authorization", auth(&raw_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            got_429 = true;
            break;
        }
    }
    assert!(got_429, "expected at least one 429 after 70 rapid requests");
}

#[tokio::test]
async fn health_check_returns_ok() {
    let (app, _db, _org, _raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), b"ok");
}

#[tokio::test]
async fn list_runs_returns_empty_without_runs() {
    let (app, _db, _org, raw_key) = fixture().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/runs")
                .header("Authorization", auth(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn create_workflow_with_bad_trigger_returns_400() {
    let (app, _db, _org, raw_key) = fixture().await;

    let bad_body = serde_json::json!({
        "name": "bad-wf",
        "trigger": {"type": "unknown_trigger_xyz"},
        "steps": []
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/workflows")
                .header("Content-Type", "application/json")
                .header("Authorization", auth(&raw_key))
                .body(Body::from(serde_json::to_vec(&bad_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
