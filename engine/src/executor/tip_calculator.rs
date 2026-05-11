use std::collections::VecDeque;

/// Calculates optimal Jito tip based on recent bundle telemetry.
///
/// Strategy (IDEA.md §6.2): 75th-percentile of the last 50 recorded tips,
/// bounded by `[min_tip, 10_000_000]` lamports (10_000_000 = 0.01 SOL).
pub struct TipCalculator {
    /// Rolling window of recent successful bundle tips (up to 50 entries).
    recent_tips: VecDeque<u64>,
    /// Hard minimum tip floor in lamports.
    min_tip: u64,
}

impl TipCalculator {
    pub fn new() -> Self {
        Self {
            recent_tips: VecDeque::with_capacity(50),
            min_tip: 1_000, // 0.000001 SOL
        }
    }

    /// Calculate the recommended tip in lamports.
    ///
    /// - If no history: returns `min_tip`.
    /// - Otherwise: 75th percentile, clamped to `[min_tip, 0.01 SOL]`.
    pub fn calculate(&self) -> u64 {
        if self.recent_tips.is_empty() {
            return self.min_tip;
        }

        let mut sorted: Vec<u64> = self.recent_tips.iter().copied().collect();
        sorted.sort_unstable();
        let p75_idx = sorted.len() * 75 / 100;
        let p75 = sorted[p75_idx];

        p75.max(self.min_tip).min(10_000_000)
    }

    /// Record a successful tip so future calculations are informed.
    pub fn record_successful_tip(&mut self, tip: u64) {
        if self.recent_tips.len() == 50 {
            self.recent_tips.pop_front();
        }
        self.recent_tips.push_back(tip);
    }
}

impl Default for TipCalculator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_returns_min_when_empty() {
        let calc = TipCalculator::new();
        assert_eq!(calc.calculate(), 1_000);
    }

    #[test]
    fn calculate_returns_p75_after_records() {
        let mut calc = TipCalculator::new();
        // 4 records: [1000, 2000, 3000, 4000]
        // p75 index = 4 * 75 / 100 = 3 → value = 4000
        for v in [1_000u64, 2_000, 3_000, 4_000] {
            calc.record_successful_tip(v);
        }
        assert_eq!(calc.calculate(), 4_000);
    }

    #[test]
    fn calculate_caps_at_0_01_sol() {
        let mut calc = TipCalculator::new();
        // Fill with values above the cap
        for _ in 0..50 {
            calc.record_successful_tip(99_000_000);
        }
        assert_eq!(calc.calculate(), 10_000_000);
    }

    #[test]
    fn rolling_window_drops_oldest_after_50() {
        let mut calc = TipCalculator::new();
        for i in 0..50 {
            calc.record_successful_tip(i * 100);
        }
        // Queue is full; push one more — oldest (0) should be dropped
        calc.record_successful_tip(9_999_999);
        assert_eq!(calc.recent_tips.len(), 50);
        assert!(!calc.recent_tips.contains(&0));
    }

    #[test]
    fn calculate_respects_min_floor() {
        let mut calc = TipCalculator::new();
        // All tips below min_tip (500 < 1_000)
        for _ in 0..10 {
            calc.record_successful_tip(500);
        }
        assert_eq!(calc.calculate(), 1_000);
    }
}
