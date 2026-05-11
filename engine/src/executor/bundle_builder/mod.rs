use async_trait::async_trait;
use solana_sdk::{signature::Signature, transaction::VersionedTransaction};
use std::sync::Arc;

use crate::wallet::Signer;

#[derive(Debug, Clone)]
pub struct BundleResult {
    pub bundle_id: String,
    pub tip_lamports: u64,
    pub signatures: Vec<Signature>,
}

#[async_trait]
pub trait BundleBuilder: Send + Sync {
    async fn build_and_submit(
        &self,
        txs: Vec<VersionedTransaction>,
        signer: Arc<dyn Signer>,
    ) -> anyhow::Result<BundleResult>;
}

// ---------------------------------------------------------------------------
// Mock bundle builder (for tests)
// ---------------------------------------------------------------------------

pub struct MockBundleBuilder {
    /// When `true`, every submission call returns an error.
    pub always_fail: bool,
    pub tip_lamports: u64,
}

impl MockBundleBuilder {
    pub fn new() -> Self {
        Self {
            always_fail: false,
            tip_lamports: 1_000,
        }
    }

    pub fn failing() -> Self {
        Self {
            always_fail: true,
            tip_lamports: 1_000,
        }
    }
}

impl Default for MockBundleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BundleBuilder for MockBundleBuilder {
    async fn build_and_submit(
        &self,
        txs: Vec<VersionedTransaction>,
        _signer: Arc<dyn Signer>,
    ) -> anyhow::Result<BundleResult> {
        if self.always_fail {
            anyhow::bail!("mock bundle failure");
        }
        let signatures = txs
            .iter()
            .map(|tx| tx.signatures.first().copied().unwrap_or_default())
            .collect();
        Ok(BundleResult {
            bundle_id: uuid::Uuid::new_v4().to_string(),
            tip_lamports: self.tip_lamports,
            signatures,
        })
    }
}

// ---------------------------------------------------------------------------
// Live Jito bundle builder (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "live-net")]
pub mod jito;
#[cfg(feature = "live-net")]
pub use jito::JitoBundleBuilder;
