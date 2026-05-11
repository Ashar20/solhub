use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::Value;

/// POST /v1/execute/transfer — MVP stub
pub async fn transfer(Json(_body): Json<Value>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "not implemented",
            "note": "execute/transfer is a Task 3.2 stub"
        })),
    )
}

/// POST /v1/execute/program — MVP stub
pub async fn program(Json(_body): Json<Value>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "not implemented",
            "note": "execute/program is a Task 3.2 stub"
        })),
    )
}
