//! Turnkey HSM-backed signer.
//!
//! Gated behind the `live-net` feature flag. The private key NEVER leaves the
//! Turnkey secure enclave. This module compiles but its `sign_transaction`
//! implementation calls `unimplemented!()` until the Turnkey integration is
//! wired with live credentials.
//!
//! See IDEA.md §10.1 for the full design.

use async_trait::async_trait;
use reqwest::Client;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::wallet::Signer;

pub struct TurnkeyWallet {
    api_key: String,
    organization_id: String,
    wallet_id: String,
    /// Ed25519 public key of this wallet (fetched at construction time).
    pubkey: Pubkey,
    http: Client,
    base_url: String,
}

impl TurnkeyWallet {
    pub fn new(
        api_key: String,
        organization_id: String,
        wallet_id: String,
        pubkey: Pubkey,
    ) -> Self {
        Self {
            api_key,
            organization_id,
            wallet_id,
            pubkey,
            http: Client::new(),
            base_url: "https://api.turnkey.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[async_trait]
impl Signer for TurnkeyWallet {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    /// Sign a transaction using Turnkey's secure enclave.
    ///
    /// # Implementation note
    /// The production flow (IDEA.md §10.1):
    /// 1. Serialize `tx.message` with `bincode`.
    /// 2. Base64-encode the bytes.
    /// 3. POST to `{base_url}/public/v1/submit/sign_raw_payload`.
    /// 4. Decode the returned hex signature into a `[u8; 64]` array.
    /// 5. Replace `tx.signatures[0]` with the received signature.
    ///
    /// This requires a live Turnkey account with a funded wallet. Until then,
    /// `unimplemented!()` is the safe default to prevent silent no-ops.
    async fn sign_transaction(
        &self,
        _tx: VersionedTransaction,
    ) -> anyhow::Result<VersionedTransaction> {
        let _ = &self.http;
        let _ = &self.api_key;
        let _ = &self.organization_id;
        let _ = &self.wallet_id;
        let _ = &self.base_url;
        unimplemented!(
            "TurnkeyWallet::sign_transaction requires a live Turnkey account — \
             see IDEA.md §10.1 and set TURNKEY_API_KEY, TURNKEY_ORG_ID, TURNKEY_WALLET_ID"
        )
    }
}
