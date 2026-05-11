pub mod local;
pub mod agentic;
#[cfg(feature = "live-net")]
pub mod turnkey;

use async_trait::async_trait;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

#[async_trait]
pub trait Signer: Send + Sync {
    fn pubkey(&self) -> Pubkey;
    async fn sign_transaction(
        &self,
        tx: VersionedTransaction,
    ) -> anyhow::Result<VersionedTransaction>;
}

pub use local::LocalKeypairSigner;
pub use agentic::AgenticWallet;
