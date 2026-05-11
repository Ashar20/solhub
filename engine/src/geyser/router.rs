use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RawEvent {
    pub account: String,
    pub slot: u64,
    pub data: Vec<u8>,
}

impl RawEvent {
    /// 32-byte SHA-256 content hash used for deduplication.
    pub fn content_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.account.as_bytes());
        h.update(self.slot.to_le_bytes());
        h.update(&self.data);
        h.finalize().into()
    }

    /// Promote to a canonical engine event.
    pub fn into_canonical(self) -> CanonicalEvent {
        CanonicalEvent {
            event_type: EventType::Account,
            account: self.account,
            slot: self.slot,
            data: serde_json::Value::Null,
            received_at: std::time::SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum EventType {
    Account,
    Tx,
    Log,
}

#[derive(Debug, Clone)]
pub struct CanonicalEvent {
    pub event_type: EventType,
    pub account: String,
    pub slot: u64,
    pub data: serde_json::Value,
    pub received_at: std::time::SystemTime,
}

// ---------------------------------------------------------------------------
// Dual-feed router
// ---------------------------------------------------------------------------

/// Fan-in router for Yellowstone and ShredStream feeds.
///
/// The first feed to deliver an event wins; duplicates within the
/// `dedup_window` (default 500 ms) are silently dropped.
pub struct DualFeedRouter {
    dedup_window: Duration,
    /// `content_hash → first-seen instant`
    seen: HashMap<[u8; 32], Instant>,
    output: mpsc::Sender<CanonicalEvent>,
}

impl DualFeedRouter {
    pub fn new(output: mpsc::Sender<CanonicalEvent>) -> Self {
        Self {
            dedup_window: Duration::from_millis(500),
            seen: HashMap::new(),
            output,
        }
    }

    /// Ingest an event from either feed.
    ///
    /// Returns `true` if the event was forwarded, `false` if it was a duplicate
    /// within the dedup window.
    pub async fn ingest(&mut self, event: RawEvent) -> bool {
        let key = event.content_hash();
        let now = Instant::now();

        // Evict stale entries
        self.seen
            .retain(|_, ts| now.duration_since(*ts) < self.dedup_window);

        if self.seen.contains_key(&key) {
            return false; // duplicate — drop
        }

        self.seen.insert(key, now);
        self.output.send(event.into_canonical()).await.is_ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_event(account: &str, slot: u64, data: &[u8]) -> RawEvent {
        RawEvent {
            account: account.to_string(),
            slot,
            data: data.to_vec(),
        }
    }

    #[tokio::test]
    async fn duplicate_within_window_is_dropped() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut router = DualFeedRouter::new(tx);

        let ev = make_event("acct1", 1, b"hello");
        let fwd = router.ingest(ev.clone()).await;
        assert!(fwd, "first delivery should be forwarded");

        let dup = router.ingest(ev).await;
        assert!(!dup, "duplicate within window should be dropped");

        // Only one event should be in the channel
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn same_content_after_window_passes_through() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut router = DualFeedRouter::new(tx);

        let ev = make_event("acct2", 2, b"world");

        router.ingest(ev.clone()).await;
        // Advance time past the dedup window
        tokio::time::advance(Duration::from_millis(600)).await;
        let fwd = router.ingest(ev).await;
        assert!(fwd, "same content after window should pass through");

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn different_content_both_pass() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut router = DualFeedRouter::new(tx);

        let ev1 = make_event("acct3", 3, b"alpha");
        let ev2 = make_event("acct3", 3, b"beta"); // same account+slot but different data

        assert!(router.ingest(ev1).await);
        assert!(router.ingest(ev2).await);

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }
}
