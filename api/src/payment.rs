use std::str::FromStr;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status_client_types::{EncodedTransaction, UiTransactionEncoding};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Describes what the server requires from the payer for a given workflow call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentRequirements {
    pub network: String,          // "solana-devnet"
    pub asset: String,            // "SOL"
    pub amount_lamports: u64,
    pub recipient: String,        // base58 pubkey
    pub memo: String,             // "hub-call:<workflow_id>"
}

/// Returned by the verifier on a successful on-chain check.
#[derive(Debug, Clone)]
pub struct VerifiedPayment {
    pub signature: String,
    pub payer: String,
    pub amount_lamports: u64,
}

// ---------------------------------------------------------------------------
// Verifier
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PaymentVerifier {
    pub rpc: Arc<RpcClient>,
    /// Maximum age (in seconds) of the on-chain transaction block_time.
    /// Transactions older than this window are rejected to prevent replay of stale proofs.
    pub max_age_secs: u64,
}

impl PaymentVerifier {
    pub fn new(rpc: Arc<RpcClient>) -> Self {
        Self {
            rpc,
            max_age_secs: 600,
        }
    }

    /// Verify a `solana:devnet:tx:<signature>` payment proof against the live RPC.
    ///
    /// Checks:
    /// 1. The signature is a valid base58 Solana signature.
    /// 2. The transaction exists on-chain and succeeded.
    /// 3. The transaction's block_time is within `max_age_secs` of now.
    /// 4. The recipient account balance increased by at least `req.amount_lamports`.
    pub async fn verify(
        &self,
        signature: &str,
        req: &PaymentRequirements,
    ) -> anyhow::Result<VerifiedPayment> {
        let sig = Signature::from_str(signature)
            .map_err(|e| anyhow::anyhow!("invalid signature format: {e}"))?;

        let recipient_pk = Pubkey::from_str(&req.recipient)
            .map_err(|e| anyhow::anyhow!("invalid recipient pubkey: {e}"))?;

        let cfg = solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let tx_status = self
            .rpc
            .get_transaction_with_config(&sig, cfg)
            .await
            .map_err(|e| anyhow::anyhow!("rpc fetch failed: {e}"))?;

        // --- Block time freshness check ---
        if let Some(block_time) = tx_status.block_time {
            let now = chrono::Utc::now().timestamp();
            let age = now.saturating_sub(block_time);
            if age as u64 > self.max_age_secs {
                anyhow::bail!(
                    "payment too old: {}s > {}s limit",
                    age,
                    self.max_age_secs
                );
            }
        }

        // --- Transaction success check ---
        let meta = tx_status
            .transaction
            .meta
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing tx meta"))?;

        if meta.err.is_some() {
            anyhow::bail!("payment tx failed on-chain: {:?}", meta.err);
        }

        // --- Decode transaction to resolve account indices ---
        let encoded = match &tx_status.transaction.transaction {
            EncodedTransaction::Binary(b64, _) => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?
            }
            _ => anyhow::bail!("unexpected tx encoding (expected Base64)"),
        };

        let vtx: solana_sdk::transaction::VersionedTransaction =
            bincode::deserialize(&encoded)
                .map_err(|e| anyhow::anyhow!("tx deserialize failed: {e}"))?;

        let account_keys = vtx.message.static_account_keys();

        let recipient_idx = account_keys
            .iter()
            .position(|k| k == &recipient_pk)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "recipient {} not found in tx accounts",
                    req.recipient
                )
            })?;

        // Index 0 is always the fee-payer / signer
        let payer_idx: usize = 0;

        // --- Balance delta check ---
        let pre = meta
            .pre_balances
            .get(recipient_idx)
            .copied()
            .unwrap_or(0);
        let post = meta
            .post_balances
            .get(recipient_idx)
            .copied()
            .unwrap_or(0);
        let delta = post.saturating_sub(pre);

        if delta < req.amount_lamports {
            anyhow::bail!(
                "payment amount {} lamports < required {} lamports",
                delta,
                req.amount_lamports
            );
        }

        Ok(VerifiedPayment {
            signature: signature.to_string(),
            payer: account_keys[payer_idx].to_string(),
            amount_lamports: delta,
        })
    }
}

// ---------------------------------------------------------------------------
// Header parsing helpers
// ---------------------------------------------------------------------------

/// Parse the `X-PAYMENT` header value of the form `solana:devnet:tx:<signature>`.
/// Returns the raw signature string on success.
pub fn parse_x_payment_header(value: &str) -> Option<String> {
    // Expected format: solana:devnet:tx:<base58-signature>
    let parts: Vec<&str> = value.splitn(4, ':').collect();
    if parts.len() == 4 && parts[0] == "solana" && parts[1] == "devnet" && parts[2] == "tx" {
        Some(parts[3].to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_x_payment_header_valid() {
        let sig = "5BranyzMmRfKkYMNyXZadKUFkH5cGVBF1SHfVdYvLkTt1a5hMJrKRwGx9UkGT1FqxNE5wT8VaXqRaDMFhHyeptz";
        let header = format!("solana:devnet:tx:{}", sig);
        assert_eq!(parse_x_payment_header(&header), Some(sig.to_string()));
    }

    #[test]
    fn parse_x_payment_header_invalid_prefix() {
        assert!(parse_x_payment_header("ethereum:mainnet:tx:abc123").is_none());
        assert!(parse_x_payment_header("garbage").is_none());
        assert!(parse_x_payment_header("solana:mainnet:tx:abc").is_none());
    }

    #[test]
    fn parse_x_payment_header_with_colon_in_signature() {
        // Signatures don't have colons, but splitn(4) ensures extras go to sig part
        let header = "solana:devnet:tx:abc:def";
        // With splitn(4) this produces ["solana","devnet","tx","abc:def"]
        assert_eq!(
            parse_x_payment_header(header),
            Some("abc:def".to_string())
        );
    }
}
