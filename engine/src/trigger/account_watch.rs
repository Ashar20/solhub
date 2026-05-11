//! Account-watch trigger.
//!
//! Full implementation requires the Geyser/Yellowstone gRPC feed which is
//! gated behind the `live-net` feature flag (see `engine/src/geyser/`).
//!
//! Architecture (IDEA.md §5):
//! 1. `AccountWatcher` subscribes to account updates via `DualFeedRouter`.
//! 2. On each `CanonicalEvent` it evaluates the workflow's `WatchCondition`.
//! 3. If the condition matches and the workflow is active, it calls
//!    `db.create_run()` with `triggered_by = "account_watch"`.
//!
//! The `Scheduler` then picks up the resulting `Pending` run as normal.

pub struct AccountWatchTrigger;

/// A condition that can be checked against an account balance change.
#[derive(Debug, Clone)]
pub enum WatchCondition {
    BalanceAbove { lamports: u64 },
    BalanceBelow { lamports: u64 },
    DataChanges,
}

impl WatchCondition {
    /// Returns `true` if `current_lamports` satisfies this condition.
    pub fn matches(&self, current_lamports: u64, changed: bool) -> bool {
        match self {
            Self::BalanceAbove { lamports } => current_lamports > *lamports,
            Self::BalanceBelow { lamports } => current_lamports < *lamports,
            Self::DataChanges => changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_above_condition() {
        let c = WatchCondition::BalanceAbove { lamports: 1_000_000 };
        assert!(c.matches(2_000_000, false));
        assert!(!c.matches(500_000, false));
    }

    #[test]
    fn data_changes_condition() {
        let c = WatchCondition::DataChanges;
        assert!(c.matches(0, true));
        assert!(!c.matches(0, false));
    }
}
