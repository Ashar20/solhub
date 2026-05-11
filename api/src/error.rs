use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("rate limited")]
    RateLimited,
    #[error("invalid signature")]
    InvalidSignature,
    /// HTTP 402 — carry a structured JSON body alongside the status.
    #[error("payment required")]
    PaymentRequired(Value),
    /// HTTP 409 — replay detected (signature already consumed).
    #[error("payment replay detected")]
    PaymentReplay,
    #[error("db error: {0}")]
    Db(#[from] db::DbError),
    #[error("internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::PaymentRequired(body) => {
                (StatusCode::PAYMENT_REQUIRED, Json(body)).into_response()
            }
            AppError::PaymentReplay => (
                StatusCode::CONFLICT,
                Json(json!({"error": "payment replay detected: signature already used"})),
            )
                .into_response(),
            other => {
                let (code, msg) = match &other {
                    AppError::NotFound => (StatusCode::NOT_FOUND, other.to_string()),
                    AppError::Unauthorized => (StatusCode::UNAUTHORIZED, other.to_string()),
                    AppError::Forbidden => (StatusCode::FORBIDDEN, other.to_string()),
                    AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, other.to_string()),
                    AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, other.to_string()),
                    AppError::InvalidSignature => {
                        (StatusCode::UNAUTHORIZED, other.to_string())
                    }
                    AppError::Db(_) | AppError::Internal(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, other.to_string())
                    }
                    // Already handled above
                    AppError::PaymentRequired(_) | AppError::PaymentReplay => unreachable!(),
                };
                (code, Json(json!({"error": msg}))).into_response()
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
