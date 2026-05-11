use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

use db::Organization;

struct Bucket {
    tokens: f64,
    last: Instant,
}

static BUCKETS: OnceLock<Mutex<HashMap<uuid::Uuid, Bucket>>> = OnceLock::new();

const CAPACITY: f64 = 60.0;
const REFILL_PER_SEC: f64 = 5.0;

pub async fn rate_limit_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    if let Some(org) = req.extensions().get::<Organization>() {
        let map = BUCKETS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map.lock().unwrap();
        let now = Instant::now();
        let bucket = guard.entry(org.id).or_insert(Bucket {
            tokens: CAPACITY,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * REFILL_PER_SEC).min(CAPACITY);
        bucket.last = now;
        if bucket.tokens < 1.0 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        bucket.tokens -= 1.0;
    }
    Ok(next.run(req).await)
}
