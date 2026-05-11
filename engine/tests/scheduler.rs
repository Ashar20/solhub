use std::sync::Arc;
use std::time::Duration;

use db::{Db, NewRun, NewWorkflow};
use engine::{
    executor::{ExecutorWorker, MockBundleBuilder, MockSimulator},
    plugins::{test_plugin::EchoPlugin, PluginRegistry},
    trigger::Scheduler,
    wallet::LocalKeypairSigner,
};
use serde_json::json;
use solana_sdk::signature::Keypair;
use tokio::sync::mpsc;

async fn setup_db() -> Db {
    let db = Db::connect_in_memory().await.unwrap();
    db.migrate().await.unwrap();
    db
}

/// Insert a workflow + Pending run, spawn the Scheduler in a background task,
/// and assert that within 5 seconds the run transitions to `Confirmed`.
#[tokio::test]
async fn scheduler_picks_up_pending_run_and_confirms_it() {
    let db = setup_db().await;
    let org = db.create_org("sched-org", None).await.unwrap();

    let mut reg = PluginRegistry::new();
    reg.register(Arc::new(EchoPlugin));

    let kp = Keypair::new();
    let worker = Arc::new(ExecutorWorker {
        db: db.clone(),
        plugins: Arc::new(reg),
        signer: Arc::new(LocalKeypairSigner::new(kp)),
        bundle_builder: Arc::new(MockBundleBuilder::new()),
        simulator: Arc::new(MockSimulator { units: 200_000 }),
    });

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "sched-echo-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "s1",
                "plugin": "test.echo",
                "action": "echo",
                "params": { "x": 42 }
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

    let run_id = run.run_id;

    let (manual_tx, manual_rx) = mpsc::channel::<uuid::Uuid>(16);
    let scheduler = Scheduler {
        db: db.clone(),
        executor: worker,
        manual: manual_rx,
    };

    // Drop the sender so the scheduler's `manual.recv()` branch returns
    // `None` and doesn't block. The scheduler will still poll every 500ms.
    drop(manual_tx);

    tokio::spawn(async move {
        let _ = scheduler.run().await;
    });

    // Poll for up to 5 seconds
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let updated = db.get_run(run_id).await.unwrap().unwrap();
        if updated.status == "Confirmed" {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!(
                "run {} did not reach Confirmed within 5s (status = {})",
                run_id, updated.status
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Posting a run to the manual channel should also get it confirmed.
#[tokio::test]
async fn scheduler_executes_manually_triggered_run() {
    let db = setup_db().await;
    let org = db.create_org("manual-org", None).await.unwrap();

    let mut reg = PluginRegistry::new();
    reg.register(Arc::new(EchoPlugin));

    let kp = Keypair::new();
    let worker = Arc::new(ExecutorWorker {
        db: db.clone(),
        plugins: Arc::new(reg),
        signer: Arc::new(LocalKeypairSigner::new(kp)),
        bundle_builder: Arc::new(MockBundleBuilder::new()),
        simulator: Arc::new(MockSimulator { units: 200_000 }),
    });

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "manual-echo-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "s1",
                "plugin": "test.echo",
                "action": "echo",
                "params": {}
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

    let run_id = run.run_id;

    // Set the run to a non-Pending status so the poll-based path doesn't
    // grab it; only the manual channel should fire it.
    db.update_run_status(run_id, "Triggered", None).await.unwrap();

    let (manual_tx, manual_rx) = mpsc::channel::<uuid::Uuid>(16);
    let scheduler = Scheduler {
        db: db.clone(),
        executor: worker,
        manual: manual_rx,
    };

    tokio::spawn(async move {
        let _ = scheduler.run().await;
    });

    // Publish the run ID on the manual channel
    manual_tx.send(run_id).await.unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let updated = db.get_run(run_id).await.unwrap().unwrap();
        if updated.status == "Confirmed" {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!(
                "manually-triggered run {} did not reach Confirmed within 5s (status = {})",
                run_id, updated.status
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
