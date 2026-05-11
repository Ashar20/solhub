//! Real Jito block-engine bundle submitter.
//!
//! Gated behind the `live-net` cargo feature. Requires a running Jito
//! searcher-client connection. The current body calls `unimplemented!()` —
//! it exists so that `cargo check --features live-net` passes and the
//! scaffolding is clear for whoever wires the live integration.
//!
//! See IDEA.md §6.1 for the full design spec.

use async_trait::async_trait;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;

use crate::wallet::Signer;
use super::{BundleBuilder, BundleResult};

/// Live Jito bundle builder backed by the Jito searcher-client gRPC API.
pub struct JitoBundleBuilder {
    /// Jito block-engine endpoint, e.g. `https://mainnet.block-engine.jito.wtf`
    pub endpoint: String,
    /// Auth token for the Jito block engine.
    pub auth_token: String,
}

impl JitoBundleBuilder {
    pub fn new(endpoint: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth_token: auth_token.into(),
        }
    }
}

#[async_trait]
impl BundleBuilder for JitoBundleBuilder {
    async fn build_and_submit(
        &self,
        _txs: Vec<VersionedTransaction>,
        _signer: Arc<dyn Signer>,
    ) -> anyhow::Result<BundleResult> {
        unimplemented!(
            "JitoBundleBuilder::build_and_submit requires a live Jito block-engine — \
             set JITO_ENDPOINT and JITO_AUTH_TOKEN (see IDEA.md §6.1)"
        )
    }
}
