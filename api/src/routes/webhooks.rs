use axum::{
    extract::{Path, State},
    Json,
};
use db::NewRun;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    types::WebhookResponse,
};

type HmacSha256 = Hmac<Sha256>;

pub async fn receive_webhook(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> AppResult<Json<WebhookResponse>> {
    let wf = state
        .db
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if wf.trigger_type != "webhook" {
        return Err(AppError::BadRequest("not a webhook workflow".into()));
    }

    let secret = wf
        .trigger_config
        .get("secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("workflow missing secret".into()))?;

    let sig = headers
        .get("X-SK-Signature")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("sha256="))
        .ok_or(AppError::InvalidSignature)?;

    let expected = {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| AppError::Internal("hmac key error".into()))?;
        mac.update(&body);
        hex::encode(mac.finalize().into_bytes())
    };

    if !constant_time_eq::constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        return Err(AppError::InvalidSignature);
    }

    let run = state
        .db
        .create_run(NewRun {
            workflow_id,
            org_id: wf.org_id,
            triggered_by: "webhook".into(),
        })
        .await?;

    Ok(Json(WebhookResponse {
        run_id: run.run_id,
        status: run.status,
    }))
}
