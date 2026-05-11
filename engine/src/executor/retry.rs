use std::{future::Future, time::Duration};

/// Exponential-backoff retry policy (IDEA.md §6.3).
///
/// Delays: `base_delay_ms * 2^(attempt-1)`, capped at `max_delay_ms`.
/// Example with defaults: 500ms → 1 000ms → 2 000ms … up to 8 000ms.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_attempts: u8, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Execute `f` up to `max_attempts` times, applying exponential backoff
    /// between failures.
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0u8;
        loop {
            match f().await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.max_attempts {
                        return Err(e);
                    }
                    let delay = (self.base_delay_ms
                        * 2u64.pow(u32::from(attempt) - 1))
                    .min(self.max_delay_ms);
                    tracing::warn!(
                        attempt,
                        delay_ms = delay,
                        error = ?e,
                        "step failed; retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, 500, 8_000)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn retries_three_times_then_succeeds() {
        let call_count = Arc::new(Mutex::new(0u8));
        let policy = RetryPolicy::new(4, 1, 100);

        let cc = call_count.clone();
        let result: Result<&str, &str> = policy
            .execute(|| {
                let cc = cc.clone();
                async move {
                    let mut n = cc.lock().unwrap();
                    *n += 1;
                    if *n < 4 {
                        Err("not yet")
                    } else {
                        Ok("done")
                    }
                }
            })
            .await;

        assert_eq!(result, Ok("done"));
        assert_eq!(*call_count.lock().unwrap(), 4);
    }

    #[tokio::test]
    async fn gives_up_after_max_attempts() {
        let call_count = Arc::new(Mutex::new(0u8));
        let policy = RetryPolicy::new(3, 1, 100);

        let cc = call_count.clone();
        let result: Result<(), &str> = policy
            .execute(|| {
                let cc = cc.clone();
                async move {
                    *cc.lock().unwrap() += 1;
                    Err("always fails")
                }
            })
            .await;

        assert_eq!(result, Err("always fails"));
        // Should have called exactly max_attempts times
        assert_eq!(*call_count.lock().unwrap(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn uses_exponential_backoff() {
        use tokio::time::Instant;

        let start = Instant::now();

        let call_count = Arc::new(Mutex::new(0u8));
        let policy = RetryPolicy::new(3, 100, 10_000);

        let cc = call_count.clone();
        let _: Result<(), &str> = policy
            .execute(|| {
                let cc = cc.clone();
                async move {
                    *cc.lock().unwrap() += 1;
                    Err("fail")
                }
            })
            .await;

        // With max_attempts=3 and base=100ms:
        //  attempt 1 fails → sleep 100ms (100ms * 2^0)
        //  attempt 2 fails → sleep 200ms (100ms * 2^1)
        //  attempt 3 fails → return error (no sleep)
        // Total elapsed must be >= 300ms in paused-time
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(300),
            "elapsed {:?} < 300ms",
            elapsed
        );
    }
}
