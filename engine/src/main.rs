use std::sync::Arc;

use engine::{
    executor::{ExecutorWorker, MockBundleBuilder, MockSimulator, RpcSubmitBuilder, RpcSimulator},
    plugins::PluginRegistry,
    trigger::{cron::CronTriggers, Scheduler},
    wallet::{LocalKeypairSigner, Signer},
};
use solana_sdk::signature::Keypair;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());

    let db = db::Db::connect(&database_url).await?;
    db.migrate().await?;

    let plugins = Arc::new(PluginRegistry::default());

    // When SOLANA_RPC_URL is set, use real RPC + real keypair from disk.
    // Otherwise fall back to mock implementations for local development.
    let (signer, bundle_builder, simulator): (
        Arc<dyn Signer>,
        Arc<dyn engine::executor::BundleBuilder>,
        Arc<dyn engine::executor::Simulator>,
    ) = if let Ok(rpc_url) = std::env::var("SOLANA_RPC_URL") {
        let rpc = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url));

        let keypair_path = std::env::var("SOLHUB_KEYPAIR")
            .unwrap_or_else(|_| "./solhub-dev.json".to_string());
        let keypair_bytes: Vec<u8> =
            serde_json::from_str(&std::fs::read_to_string(&keypair_path)?)?;
        let keypair = Keypair::try_from(keypair_bytes.as_slice())
            .map_err(|e| anyhow::anyhow!("bad keypair: {e}"))?;

        (
            Arc::new(LocalKeypairSigner::new(keypair)),
            Arc::new(RpcSubmitBuilder::new(rpc.clone())),
            Arc::new(RpcSimulator { rpc }),
        )
    } else {
        // Development / CI — ephemeral keypair and mock infra
        let keypair = Keypair::new();
        (
            Arc::new(LocalKeypairSigner::new(keypair)),
            Arc::new(MockBundleBuilder::new()),
            Arc::new(MockSimulator { units: 200_000 }),
        )
    };

    let executor = Arc::new(ExecutorWorker {
        db: db.clone(),
        plugins,
        signer,
        bundle_builder,
        simulator,
    });

    // Cron triggers — register all active cron workflows
    let crons = CronTriggers::new(db.clone()).await?;
    crons.load_and_start().await?;

    // Manual-trigger channel (will be wired to the API layer in a future task)
    let (manual_tx, manual_rx) = mpsc::channel::<uuid::Uuid>(256);
    // Keep the sender alive so the scheduler doesn't see a closed channel.
    let _manual_tx = manual_tx;

    let scheduler = Scheduler {
        db,
        executor,
        manual: manual_rx,
    };

    tracing::info!("solhub-engine started");
    scheduler.run().await
}
