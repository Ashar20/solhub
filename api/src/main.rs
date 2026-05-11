use std::sync::Arc;

use api::{app::build_router, payment::PaymentVerifier, state::AppState};
use solana_client::nonblocking::rpc_client::RpcClient;

/// Default treasury pubkey — matches the deployer in deployments/devnet.json.
const DEFAULT_TREASURY: &str = "FPRYNqc3vGqNsAmpj7xuCDWZDZ3ZWGiB45oD3rhrc6Nb";

/// Default Solana devnet RPC.
const DEFAULT_RPC: &str = "https://api.devnet.solana.com";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    let port: u16 = std::env::var("API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // Treasury: prefer SOLHUB_TREASURY env var; fall back to hard-coded devnet deployer.
    let treasury_pubkey =
        std::env::var("SOLHUB_TREASURY").unwrap_or_else(|_| DEFAULT_TREASURY.to_string());

    // RPC URL for payment verification.
    let rpc_url =
        std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC.to_string());

    let db = db::Db::connect(&database_url).await?;
    db.migrate().await?;

    let mut plugins_registry = engine::plugins::PluginRegistry::default();
    plugins_registry.register_solhub(db.clone());
    let plugins = Arc::new(plugins_registry);

    let rpc = Arc::new(RpcClient::new(rpc_url));
    let payment_verifier = PaymentVerifier::new(rpc);

    let state = AppState::new(db, plugins, payment_verifier, treasury_pubkey);
    let app = build_router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "solhub-api listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
