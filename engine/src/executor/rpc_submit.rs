use async_trait::async_trait;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, transaction::VersionedTransaction};
use std::sync::Arc;

use crate::wallet::Signer;

use super::{BundleBuilder, BundleResult};

pub struct RpcSubmitBuilder {
    pub rpc: Arc<RpcClient>,
}

impl RpcSubmitBuilder {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl BundleBuilder for RpcSubmitBuilder {
    async fn build_and_submit(
        &self,
        txs: Vec<VersionedTransaction>,
        _signer: Arc<dyn Signer>,
    ) -> anyhow::Result<BundleResult> {
        // Transactions are assumed to be already signed by the executor signing step.
        let mut signatures = Vec::with_capacity(txs.len());
        for tx in &txs {
            let sig = self
                .rpc
                .send_and_confirm_transaction_with_spinner_and_commitment(
                    tx,
                    CommitmentConfig::confirmed(),
                )
                .await?;
            signatures.push(sig);
        }
        Ok(BundleResult {
            bundle_id: format!("rpc-{}", uuid::Uuid::new_v4()),
            tip_lamports: 0,
            signatures,
        })
    }
}
