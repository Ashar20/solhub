use axum::{
    extract::{Extension, Path, State},
    Json,
};
use db::Organization;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    types::{CreateApiKeyRequest, CreateApiKeyResponse},
};

pub async fn me(
    Extension(org): Extension<Organization>,
) -> AppResult<Json<Organization>> {
    Ok(Json(org))
}

pub async fn create_key(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    body: Option<Json<CreateApiKeyRequest>>,
) -> AppResult<Json<CreateApiKeyResponse>> {
    let name = body.and_then(|b| b.0.name);

    let raw_key = format!("sk_{}", Uuid::new_v4().simple());

    let mut h = Sha256::new();
    h.update(raw_key.as_bytes());
    let key_hash = hex::encode(h.finalize());

    let api_key = state
        .db
        .create_api_key(org.id, &key_hash, name.as_deref())
        .await?;

    Ok(Json(CreateApiKeyResponse {
        id: api_key.id,
        key: raw_key,
        name: api_key.name,
    }))
}

pub async fn list_keys(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
) -> AppResult<Json<serde_json::Value>> {
    let keys = state.db.list_api_keys(org.id).await?;
    // Return metadata only — no raw keys
    let sanitized: Vec<serde_json::Value> = keys
        .into_iter()
        .map(|k| {
            serde_json::json!({
                "id": k.id,
                "org_id": k.org_id,
                "name": k.name,
                "last_used_at": k.last_used_at,
                "created_at": k.created_at,
                "revoked_at": k.revoked_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(sanitized)))
}

pub async fn revoke_key(
    State(state): State<AppState>,
    Extension(org): Extension<Organization>,
    Path(key_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify the key belongs to this org
    let keys = state.db.list_api_keys(org.id).await?;
    let found = keys.iter().any(|k| k.id == key_id);
    if !found {
        return Err(AppError::NotFound);
    }

    state.db.revoke_api_key(key_id).await?;
    Ok(Json(serde_json::json!({"status": "revoked"})))
}
