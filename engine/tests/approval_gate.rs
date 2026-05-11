use std::sync::Arc;

use db::{Db, NewRun, NewWorkflow};
use engine::{
    executor::{ExecutorWorker, MockBundleBuilder, MockSimulator},
    plugins::{test_plugin::EchoPlugin, PluginRegistry},
    wallet::LocalKeypairSigner,
};
use serde_json::json;
use solana_sdk::signature::Keypair;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup_db() -> Db {
    let db = Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    db
}

async fn make_worker(db: Db) -> ExecutorWorker {
    let mut reg = PluginRegistry::new();
    reg.register(Arc::new(EchoPlugin));
    reg.register_solhub(db.clone());
    let kp = Keypair::new();
    ExecutorWorker {
        db,
        plugins: Arc::new(reg),
        signer: Arc::new(LocalKeypairSigner::new(kp)),
        bundle_builder: Arc::new(MockBundleBuilder::new()),
        simulator: Arc::new(MockSimulator { units: 200_000 }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A workflow with two steps: [require_approval, echo].
/// The executor should pause at step 0 (require_approval) and leave the run in
/// WaitingApproval status with resume_from_step_index = 1.
/// After flipping the run to Resumed via DB, re-running the executor should
/// complete step 1 (echo) and finish the run as Confirmed.
#[tokio::test]
async fn approval_gate_pause_then_resume() {
    let db = setup_db().await;
    let org = db.create_org("test-org-approval", None).await.unwrap();

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "approval-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([
                {
                    "id": "gate",
                    "plugin": "solhub",
                    "action": "require_approval",
                    "params": { "message": "approve this trade?" }
                },
                {
                    "id": "s2",
                    "plugin": "test.echo",
                    "action": "echo",
                    "params": { "msg": "post-approval step" }
                }
            ]),
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

    // First execution — should pause at the approval gate.
    let worker = make_worker(db.clone()).await;
    worker.execute_run(run.run_id).await.unwrap();

    let paused = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(
        paused.status, "WaitingApproval",
        "run should be WaitingApproval after hitting the gate"
    );
    assert_eq!(
        paused.resume_from_step_index,
        Some(1),
        "executor should record step 1 as the resume point"
    );

    // The step log should contain the approval-gate entry.
    let logs = paused.steps_log.as_array().unwrap();
    assert_eq!(logs.len(), 1, "one step log entry for the gate");
    assert_eq!(logs[0]["status"], "WaitingApproval");

    // Simulate approval: flip status to Resumed.
    db.approve_run(run.run_id).await.unwrap();
    let resumed = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(resumed.status, "Resumed");

    // Second execution — should resume from step 1 (echo) and finish.
    worker.execute_run(run.run_id).await.unwrap();

    let finished = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(
        finished.status, "Confirmed",
        "run should be Confirmed after resuming"
    );

    let final_logs = finished.steps_log.as_array().unwrap();
    assert_eq!(final_logs.len(), 2, "two log entries total (gate + echo)");
    assert_eq!(final_logs[1]["step_id"], "s2");
    assert_eq!(final_logs[1]["status"], "Completed");
    assert_eq!(final_logs[1]["output"]["msg"], "post-approval step");
}

/// Rejecting a WaitingApproval run should mark it Failed with an error message
/// containing the rejection reason.
#[tokio::test]
async fn approval_gate_reject_marks_failed() {
    let db = setup_db().await;
    let org = db.create_org("test-org-reject", None).await.unwrap();

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "reject-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "gate",
                "plugin": "solhub",
                "action": "require_approval",
                "params": { "message": "approve?" }
            }]),
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

    let worker = make_worker(db.clone()).await;
    worker.execute_run(run.run_id).await.unwrap();

    let paused = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(paused.status, "WaitingApproval");

    // Reject the run.
    db.reject_run(run.run_id, "risk too high").await.unwrap();

    let rejected = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(rejected.status, "Failed");
    let err = rejected.error_message.as_deref().unwrap_or("");
    assert!(
        err.contains("rejected") && err.contains("risk too high"),
        "error_message should mention rejection reason, got: {err}"
    );
    assert!(rejected.completed_at.is_some(), "completed_at should be set for Failed run");
}
