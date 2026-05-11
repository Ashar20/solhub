use std::sync::{Arc, Mutex};
use std::time::Duration;

use engine::{
    executor::RetryPolicy,
    geyser::router::{DualFeedRouter, RawEvent},
};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// RetryPolicy integration-level tests (using tokio time pause)
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn retry_integration_exponential_backoff_total_delay() {
    let call_count = Arc::new(Mutex::new(0u8));
    let start = tokio::time::Instant::now();

    // base=200ms, max_attempts=4 → delays: 200 + 400 + 800 = 1400ms
    let policy = RetryPolicy::new(4, 200, 10_000);

    let cc = call_count.clone();
    let _: Result<(), &str> = policy
        .execute(|| {
            let cc = cc.clone();
            async move {
                *cc.lock().unwrap() += 1;
                Err("always fail")
            }
        })
        .await;

    let elapsed = start.elapsed();
    // Should have waited 200 + 400 + 800 = 1400ms total
    assert!(
        elapsed >= Duration::from_millis(1400),
        "elapsed {:?} < 1400ms",
        elapsed
    );
    assert_eq!(*call_count.lock().unwrap(), 4u8);
}

#[tokio::test(start_paused = true)]
async fn retry_integration_succeeds_on_second_attempt() {
    let call_count = Arc::new(Mutex::new(0u8));
    let policy = RetryPolicy::new(3, 50, 1_000);

    let cc = call_count.clone();
    let result: Result<&str, &str> = policy
        .execute(|| {
            let cc = cc.clone();
            async move {
                let mut n = cc.lock().unwrap();
                *n += 1;
                if *n == 1 { Err("first try") } else { Ok("ok") }
            }
        })
        .await;

    assert_eq!(result, Ok("ok"));
    assert_eq!(*call_count.lock().unwrap(), 2u8);
}

// ---------------------------------------------------------------------------
// DualFeedRouter integration tests (these complement the unit tests)
// ---------------------------------------------------------------------------

fn raw(account: &str, slot: u64, data: &[u8]) -> RawEvent {
    RawEvent {
        account: account.to_string(),
        slot,
        data: data.to_vec(),
    }
}

#[tokio::test(start_paused = true)]
async fn router_integration_dedup_then_window_expiry() {
    let (tx, mut rx) = mpsc::channel(32);
    let mut router = DualFeedRouter::new(tx);

    let ev = raw("acc", 99, b"data");

    // First delivery — forwarded
    assert!(router.ingest(ev.clone()).await);

    // Immediate duplicate — dropped
    assert!(!router.ingest(ev.clone()).await);

    // Advance time past the 500ms window
    tokio::time::advance(Duration::from_millis(600)).await;

    // Should pass again after expiry
    assert!(router.ingest(ev).await);

    // We should have received exactly 2 canonical events
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn router_integration_high_volume_no_false_drops() {
    let (tx, mut rx) = mpsc::channel(256);
    let mut router = DualFeedRouter::new(tx);

    // 100 distinct events — all should pass
    for i in 0u64..100 {
        let ev = raw("acc", i, &i.to_le_bytes());
        assert!(router.ingest(ev).await);
    }

    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 100);
}
