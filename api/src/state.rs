use std::sync::Arc;

use db::Db;
use engine::plugins::PluginRegistry;
use uuid::Uuid;

use crate::payment::PaymentVerifier;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub plugins: Arc<PluginRegistry>,
    pub manual_triggers: tokio::sync::broadcast::Sender<Uuid>,
    /// Payment verifier backed by a Solana RPC client.
    pub payment_verifier: PaymentVerifier,
    /// Base58 pubkey of the platform treasury (payment recipient).
    pub treasury_pubkey: String,
}

impl AppState {
    pub fn new(
        db: Db,
        plugins: Arc<PluginRegistry>,
        payment_verifier: PaymentVerifier,
        treasury_pubkey: String,
    ) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            db,
            plugins,
            manual_triggers: tx,
            payment_verifier,
            treasury_pubkey,
        }
    }
}
