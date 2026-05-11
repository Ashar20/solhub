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

fn make_registry_with_echo() -> PluginRegistry {
    let mut reg = PluginRegistry::new();
    reg.register(Arc::new(EchoPlugin));
    reg
}

async fn make_worker(db: Db) -> ExecutorWorker {
    let kp = Keypair::new();
    ExecutorWorker {
        db,
        plugins: Arc::new(make_registry_with_echo()),
        signer: Arc::new(LocalKeypairSigner::new(kp)),
        bundle_builder: Arc::new(MockBundleBuilder::new()),
        simulator: Arc::new(MockSimulator { units: 200_000 }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A single `test.echo` step should run to `Confirmed` and the step log
/// should contain the echoed params.
#[tokio::test]
async fn executor_runs_single_echo_step_to_confirmed() {
    let db = setup_db().await;
    let org = db.create_org("test-org", None).await.unwrap();

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "echo-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "s1",
                "plugin": "test.echo",
                "action": "echo",
                "params": { "msg": "hello" }
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

    let updated = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(updated.status, "Confirmed");

    let logs = updated.steps_log.as_array().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["step_id"], "s1");
    assert_eq!(logs[0]["status"], "Completed");
    // Echo returns the params, so output["msg"] == "hello"
    assert_eq!(logs[0]["output"]["msg"], "hello");
}

/// A workflow that references a non-existent plugin should end up `Failed`.
#[tokio::test]
async fn executor_marks_run_failed_on_unknown_plugin() {
    let db = setup_db().await;
    let org = db.create_org("test-org2", None).await.unwrap();

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "bad-plugin-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "s1",
                "plugin": "nonexistent",
                "action": "do_thing",
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

    let worker = make_worker(db.clone()).await;
    worker.execute_run(run.run_id).await.unwrap();

    let updated = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(updated.status, "Failed");
    assert!(
        updated
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("nonexistent")
    );
}

/// A workflow with a Jupiter swap step should go through the mock bundle
/// builder and end up `Confirmed` with the bundle_id set.
#[tokio::test]
async fn executor_runs_transaction_step_through_mock_bundle_builder() {
    use engine::plugins::jupiter::JupiterPlugin;

    let db = setup_db().await;
    let org = db.create_org("test-org3", None).await.unwrap();

    // Start a mock HTTP server for the Jupiter API
    let mut server = mockito::Server::new_async().await;

    // Build a valid VersionedTransaction to return from the swap endpoint
    use solana_sdk::{
        hash::Hash,
        message::{v0, VersionedMessage},
        signature::Keypair as SdkKp,
        signer::Signer as SdkSigner,
        transaction::VersionedTransaction,
    };
    let payer = SdkKp::new();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[], &[], Hash::default()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();
    let tx_bytes = bincode::serialize(&tx).unwrap();
    let tx_b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &tx_bytes);

    let _quote_mock = server
        .mock("GET", mockito::Matcher::Regex(r"^/quote".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"inputMint":"So11111111111111111111111111111111111111112","outputMint":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","inAmount":"1000000","outAmount":"99000","priceImpactPct":"0.01","routePlan":[]}"#)
        .create_async()
        .await;

    let _swap_mock = server
        .mock("POST", "/swap")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{"swapTransaction":"{}"}}"#, tx_b64))
        .create_async()
        .await;

    let mut reg = PluginRegistry::new();
    reg.register(Arc::new(JupiterPlugin::with_base_url(server.url())));

    let kp = Keypair::new();
    let worker = ExecutorWorker {
        db: db.clone(),
        plugins: Arc::new(reg),
        signer: Arc::new(LocalKeypairSigner::new(kp)),
        bundle_builder: Arc::new(MockBundleBuilder::new()),
        simulator: Arc::new(MockSimulator { units: 200_000 }),
    };

    let wf = db
        .create_workflow(NewWorkflow {
            org_id: org.id,
            name: "jup-swap-wf".into(),
            trigger_type: "manual".into(),
            trigger_config: json!({}),
            steps: json!([{
                "id": "swap1",
                "plugin": "jupiter",
                "action": "swap",
                "params": {
                    "input_mint": "So11111111111111111111111111111111111111112",
                    "output_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                    "amount": 1_000_000u64
                }
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

    worker.execute_run(run.run_id).await.unwrap();

    let updated = db.get_run(run.run_id).await.unwrap().unwrap();
    assert_eq!(updated.status, "Confirmed", "run should be Confirmed");
    // The MockBundleBuilder sets jito_tip_lamports to 1_000
    assert_eq!(updated.jito_tip_lamports, Some(1_000));
}
