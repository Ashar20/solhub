use anchor_lang::prelude::*;

declare_id!("JwYqHkFc9w3bwZuK87FEE2jviVsVCDkp7RdBXQGey7h");

/// Default CPI interface for SolHub workflow conditions.
/// Any protocol deploys their own program with the same `evaluate` instruction
/// to become a native condition source. SolHub calls it via CPI before firing
/// an action step and reads the emitted ConditionEvaluated event from logs.
#[program]
pub mod condition_oracle {
    use super::*;

    /// Evaluates a condition and emits a ConditionEvaluated event.
    /// Default implementation always emits met = true.
    ///
    /// params: protocol-specific borsh-encoded condition parameters.
    pub fn evaluate(ctx: Context<Evaluate>, params: Vec<u8>) -> Result<()> {
        let params_hash = params_to_hash(&params);

        emit!(ConditionEvaluated {
            met: true,
            params_hash,
            evaluated_at: Clock::get()?.unix_timestamp,
        });

        let _ = ctx;
        Ok(())
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Lightweight deterministic hash of params for event fingerprinting.
/// Production overrides may substitute a proper SHA-256 CPI hash.
fn params_to_hash(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut out = [0u8; 32];
    let mut h = DefaultHasher::new();
    data.hash(&mut h);
    let v = h.finish().to_le_bytes();
    out[..8].copy_from_slice(&v);
    out
}

// ─── Events ───────────────────────────────────────────────────────────────────

#[event]
pub struct ConditionEvaluated {
    pub met: bool,
    pub params_hash: [u8; 32],
    pub evaluated_at: i64,
}

// ─── Accounts Contexts ────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Evaluate<'info> {
    /// CHECK: protocol's state account — validated by the implementing program
    pub state_account: UncheckedAccount<'info>,

    pub caller: Signer<'info>,
}
