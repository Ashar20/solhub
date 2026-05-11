use async_trait::async_trait;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer as SdkSigner,
    transaction::VersionedTransaction,
};
use std::time::{Duration, SystemTime};

use super::Signer;

/// A scoped signer for AI agent workflows with per-execution spend limits.
/// The keypair is ephemeral — generated fresh for each workflow run.
/// Once the budget is exhausted or the expiry time passes, all spend checks
/// will fail and no further transactions may be signed.
pub struct AgenticWallet {
    keypair: Keypair,
    spend_limit_lamports: u64,
    spent_lamports: u64,
    expires_at: SystemTime,
}

impl AgenticWallet {
    /// Create an ephemeral signer valid for a single workflow run.
    /// Funded from the org vault; unused balance should be returned after the run.
    /// Wallet expires after 5 minutes.
    pub fn new_for_run(budget_lamports: u64) -> Self {
        Self::new_for_run_with_expiry(
            budget_lamports,
            SystemTime::now() + Duration::from_secs(300),
        )
    }

    /// Testable constructor that accepts an explicit expiry timestamp.
    pub fn new_for_run_with_expiry(budget_lamports: u64, expires_at: SystemTime) -> Self {
        Self {
            keypair: Keypair::new(),
            spend_limit_lamports: budget_lamports,
            spent_lamports: 0,
            expires_at,
        }
    }

    /// Attempt to record a spend of `lamports`.
    /// Returns an error if the spend would exceed the budget or the wallet has expired.
    /// On success, bumps `spent_lamports`.
    pub fn check_spend(&mut self, lamports: u64) -> anyhow::Result<()> {
        if SystemTime::now() > self.expires_at {
            anyhow::bail!("Agentic wallet expired");
        }
        if self.spent_lamports + lamports > self.spend_limit_lamports {
            anyhow::bail!("Spend limit exceeded");
        }
        self.spent_lamports += lamports;
        Ok(())
    }

    /// Remaining budget in lamports.
    pub fn budget_remaining(&self) -> u64 {
        self.spend_limit_lamports.saturating_sub(self.spent_lamports)
    }
}

#[async_trait]
impl Signer for AgenticWallet {
    fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    async fn sign_transaction(
        &self,
        mut tx: VersionedTransaction,
    ) -> anyhow::Result<VersionedTransaction> {
        let message_bytes = tx.message.serialize();
        let sig: Signature = self.keypair.sign_message(&message_bytes);

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

    #[test]
    fn agentic_wallet_rejects_overspend() {
        let mut wallet = AgenticWallet::new_for_run(1_000_000);
        // First spend within budget — ok
        wallet.check_spend(500_000).expect("should allow");
        assert_eq!(wallet.budget_remaining(), 500_000);

        // Second spend that would exceed — must fail
        let err = wallet.check_spend(600_000).unwrap_err();
        assert!(err.to_string().contains("Spend limit exceeded"));

        // Remaining must be unchanged after failure
        assert_eq!(wallet.budget_remaining(), 500_000);
    }

    #[test]
    fn agentic_wallet_expires_after_5_minutes() {
        // Use an already-expired timestamp
        let past = SystemTime::now() - Duration::from_secs(1);
        let mut wallet = AgenticWallet::new_for_run_with_expiry(1_000_000, past);

        let err = wallet.check_spend(1).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn agentic_wallet_budget_remaining_tracks_spends() {
        let mut wallet = AgenticWallet::new_for_run(10_000);
        assert_eq!(wallet.budget_remaining(), 10_000);
        wallet.check_spend(3_000).unwrap();
        assert_eq!(wallet.budget_remaining(), 7_000);
        wallet.check_spend(7_000).unwrap();
        assert_eq!(wallet.budget_remaining(), 0);
    }
}
