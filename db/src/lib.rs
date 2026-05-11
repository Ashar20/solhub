pub mod credits;
pub mod error;
pub mod models;
pub mod orgs;
pub mod payments;
pub mod runs;
pub mod workflows;

pub use credits::LedgerEntry;
pub use error::DbError;
pub use models::{ApiKey, NewRun, NewWorkflow, Organization, Workflow, WorkflowRun};
pub use payments::Payment;

use sqlx::SqlitePool;

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let pool = SqlitePool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn connect_in_memory() -> Result<Self, DbError> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), DbError> {
        sqlx::migrate!("../migrations").run(&self.pool).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    async fn test_db() -> Db {
        let db = Db::connect_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    // Helper: create a workflow in the test org
    async fn make_workflow(db: &Db, org_id: uuid::Uuid) -> Workflow {
        db.create_workflow(NewWorkflow {
            org_id,
            name: "my-workflow".into(),
            trigger_type: "cron".into(),
            trigger_config: json!({"cron": "* * * * *"}),
            steps: json!([{"action": "http", "url": "https://example.com"}]),
            is_public: false,
            fee_per_exec_usdc: None,
        })
        .await
        .unwrap()
    }

    // -----------------------------------------------------------------------
    // Org tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_org_then_get_org() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let fetched = db.get_org(org.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "acme");
    }

    #[tokio::test]
    async fn get_org_missing_returns_none() {
        let db = test_db().await;
        let missing = db.get_org(uuid::Uuid::new_v4()).await.unwrap();
        assert!(missing.is_none());
    }

    // -----------------------------------------------------------------------
    // ApiKey tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_api_key_then_lookup_org() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        db.create_api_key(org.id, "deadbeef01", Some("test-key"))
            .await
            .unwrap();
        let found = db
            .get_org_by_api_key_hash("deadbeef01")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, org.id);
    }

    #[tokio::test]
    async fn revoke_api_key_hides_org() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let key = db
            .create_api_key(org.id, "deadbeef02", None)
            .await
            .unwrap();
        db.revoke_api_key(key.id).await.unwrap();
        let found = db
            .get_org_by_api_key_hash("deadbeef02")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // -----------------------------------------------------------------------
    // Workflow tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_workflow_then_get_workflow_roundtrips() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;

        let fetched = db.get_workflow(wf.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, wf.id);
        assert_eq!(fetched.org_id, org.id);
        assert_eq!(fetched.name, "my-workflow");
        assert_eq!(fetched.trigger_type, "cron");
        assert_eq!(fetched.trigger_config, json!({"cron": "* * * * *"}));
        assert_eq!(
            fetched.steps,
            json!([{"action": "http", "url": "https://example.com"}])
        );
        assert!(fetched.is_active);
        assert!(!fetched.is_public);
    }

    #[tokio::test]
    async fn list_workflows_filters_active_only() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();

        let wf1 = make_workflow(&db, org.id).await;
        let wf2 = make_workflow(&db, org.id).await;

        // Soft-delete wf2
        db.delete_workflow(wf2.id).await.unwrap();

        let active = db.list_workflows(org.id, true).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, wf1.id);

        let all = db.list_workflows(org.id, false).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn update_workflow_patches_only_provided_fields() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;

        // Patch steps only
        let new_steps = json!([{"action": "slack"}]);
        let updated = db
            .update_workflow(wf.id, None, Some(new_steps.clone()), None)
            .await
            .unwrap();

        assert_eq!(updated.steps, new_steps);
        // trigger_config must be unchanged
        assert_eq!(updated.trigger_config, wf.trigger_config);
    }

    #[tokio::test]
    async fn delete_workflow_sets_is_active_false() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;

        db.delete_workflow(wf.id).await.unwrap();

        let fetched = db.get_workflow(wf.id).await.unwrap().unwrap();
        assert!(!fetched.is_active);
    }

    // -----------------------------------------------------------------------
    // Run tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_run_then_get_run_roundtrips() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;

        let run = db
            .create_run(NewRun {
                workflow_id: wf.id,
                org_id: org.id,
                triggered_by: "cron".into(),
            })
            .await
            .unwrap();

        let fetched = db.get_run(run.run_id).await.unwrap().unwrap();
        assert_eq!(fetched.run_id, run.run_id);
        assert_eq!(fetched.status, "Pending");
        assert_eq!(fetched.triggered_by, "cron");
        assert!(fetched.completed_at.is_none());
    }

    #[tokio::test]
    async fn update_run_status_confirmed_sets_completed_at() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;
        let run = db
            .create_run(NewRun {
                workflow_id: wf.id,
                org_id: org.id,
                triggered_by: "manual".into(),
            })
            .await
            .unwrap();

        db.update_run_status(run.run_id, "Confirmed", None)
            .await
            .unwrap();

        let fetched = db.get_run(run.run_id).await.unwrap().unwrap();
        assert_eq!(fetched.status, "Confirmed");
        assert!(fetched.completed_at.is_some());
    }

    #[tokio::test]
    async fn record_run_outcome_sets_fields() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;
        let run = db
            .create_run(NewRun {
                workflow_id: wf.id,
                org_id: org.id,
                triggered_by: "webhook".into(),
            })
            .await
            .unwrap();

        db.record_run_outcome(run.run_id, 12345678, "5abc123", 5000, 1000)
            .await
            .unwrap();

        let fetched = db.get_run(run.run_id).await.unwrap().unwrap();
        assert_eq!(fetched.slot, Some(12345678));
        assert_eq!(fetched.signature.as_deref(), Some("5abc123"));
        assert_eq!(fetched.fee_lamports, Some(5000));
        assert_eq!(fetched.jito_tip_lamports, Some(1000));
    }

    #[tokio::test]
    async fn append_step_log_grows_array() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf = make_workflow(&db, org.id).await;
        let run = db
            .create_run(NewRun {
                workflow_id: wf.id,
                org_id: org.id,
                triggered_by: "mcp".into(),
            })
            .await
            .unwrap();

        db.append_step_log(run.run_id, json!({"step": 1, "ok": true}))
            .await
            .unwrap();
        db.append_step_log(run.run_id, json!({"step": 2, "ok": true}))
            .await
            .unwrap();

        let fetched = db.get_run(run.run_id).await.unwrap().unwrap();
        let arr = fetched.steps_log.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["step"], 1);
        assert_eq!(arr[1]["step"], 2);
    }

    #[tokio::test]
    async fn list_runs_filters_by_status_and_workflow_and_limit() {
        let db = test_db().await;
        let org = db.create_org("acme", None).await.unwrap();
        let wf1 = make_workflow(&db, org.id).await;
        let wf2 = make_workflow(&db, org.id).await;

        // Three runs: two on wf1, one on wf2
        let r1 = db
            .create_run(NewRun {
                workflow_id: wf1.id,
                org_id: org.id,
                triggered_by: "cron".into(),
            })
            .await
            .unwrap();
        let r2 = db
            .create_run(NewRun {
                workflow_id: wf1.id,
                org_id: org.id,
                triggered_by: "manual".into(),
            })
            .await
            .unwrap();
        let _r3 = db
            .create_run(NewRun {
                workflow_id: wf2.id,
                org_id: org.id,
                triggered_by: "cron".into(),
            })
            .await
            .unwrap();

        // Mark r1 as Failed
        db.update_run_status(r1.run_id, "Failed", Some("oops"))
            .await
            .unwrap();

        // Filter by workflow_id = wf1 → 2 runs
        let by_wf = db
            .list_runs(org.id, Some(wf1.id), None, 100)
            .await
            .unwrap();
        assert_eq!(by_wf.len(), 2);

        // Filter by status = Failed → 1 run
        let by_status = db
            .list_runs(org.id, None, Some("Failed"), 100)
            .await
            .unwrap();
        assert_eq!(by_status.len(), 1);
        assert_eq!(by_status[0].run_id, r1.run_id);

        // Filter by workflow_id = wf1 + status = Pending → 1 run (r2 is Pending)
        let by_both = db
            .list_runs(org.id, Some(wf1.id), Some("Pending"), 100)
            .await
            .unwrap();
        assert_eq!(by_both.len(), 1);
        assert_eq!(by_both[0].run_id, r2.run_id);

        // Limit to 1 result
        let limited = db.list_runs(org.id, None, None, 1).await.unwrap();
        assert_eq!(limited.len(), 1);
    }
}
