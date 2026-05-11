use async_trait::async_trait;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer as SdkSigner,
    transaction::VersionedTransaction,
};
use std::sync::Arc;

use super::Signer;

/// A signer backed by a local in-memory `solana_sdk::signature::Keypair`.
/// For development and testing only. Production uses Turnkey HSM.
pub struct LocalKeypairSigner {
    keypair: Arc<Keypair>,
}

impl LocalKeypairSigner {
    pub fn new(keypair: Keypair) -> Self {
        Self {
            keypair: Arc::new(keypair),
        }
    }

    pub fn new_random() -> Self {
        Self::new(Keypair::new())
    }
}

#[async_trait]
impl Signer for LocalKeypairSigner {
    fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    async fn sign_transaction(
        &self,
        mut tx: VersionedTransaction,
    ) -> anyhow::Result<VersionedTransaction> {
        let message_bytes = tx.message.serialize();
        let sig: Signature = self.keypair.sign_message(&message_bytes);

        // Replace (or set) the first signature slot.
        if tx.signatures.is_empty() {
            tx.signatures.push(sig);
        } else {
            tx.signatures[0] = sig;
        }

        Ok(tx)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        hash::Hash,
        message::{v0, VersionedMessage},
    };

    fn make_versioned_tx(payer: &Keypair) -> VersionedTransaction {
        let msg = v0::Message::try_compile(&payer.pubkey(), &[], &[], Hash::default())
            .expect("compile empty message");
        let versioned_msg = VersionedMessage::V0(msg);
        VersionedTransaction {
            signatures: vec![Signature::default()],
            message: versioned_msg,
        }
    }

    #[tokio::test]
    async fn local_signer_signs_tx_with_pubkey() {
        let kp = Keypair::new();
        let expected_pubkey = kp.pubkey();
        let signer = LocalKeypairSigner::new(kp);

        assert_eq!(signer.pubkey(), expected_pubkey);

        let tx = make_versioned_tx(
            &Keypair::try_from(signer.keypair.to_bytes().as_ref()).unwrap(),
        );
        let signed = signer.sign_transaction(tx).await.expect("should sign");

        // Signature slot 0 must be non-default
        assert_ne!(signed.signatures[0], Signature::default());
    }
}
