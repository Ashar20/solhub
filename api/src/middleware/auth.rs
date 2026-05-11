use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::state::AppState;

const WEBHOOK_PREFIX: &str = "/v1/webhooks";

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Public routes — skip auth
    if path.starts_with(WEBHOOK_PREFIX) {
        return Ok(next.run(req).await);
    }
    if path == "/v1/hub" && method == axum::http::Method::GET {
        return Ok(next.run(req).await);
    }
    // payment_info is public so clients can discover fees without a key
    if path.ends_with("/payment_info") && method == axum::http::Method::GET {
        return Ok(next.run(req).await);
    }
    if path == "/health" {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let mut h = Sha256::new();
    h.update(token.as_bytes());
    let key_hash = hex::encode(h.finalize());

    let org = state
        .db
        .get_org_by_api_key_hash(&key_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Touch last_used_at — fire and forget
    let db = state.db.clone();
    let kh = key_hash.clone();
    tokio::spawn(async move {
        let _ = db.touch_api_key(&kh).await;
    });

    req.extensions_mut().insert(org);
    Ok(next.run(req).await)
}
