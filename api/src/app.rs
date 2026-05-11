use axum::{
    http::{HeaderValue, Method},
    middleware as ax_mw,
    routing::{delete, get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    middleware::{auth::auth_middleware, rate_limit::rate_limit_middleware},
    state::AppState,
};

/// Browser clients (Next.js dev server, etc.) need CORS. Preflight OPTIONS must succeed
/// before `Authorization` is sent; this layer handles that ahead of auth middleware.
fn cors_layer() -> CorsLayer {
    if let Ok(raw) = std::env::var("SOLHUB_CORS_ORIGINS") {
        let trimmed = raw.trim();
        if trimmed == "*" {
            return CorsLayer::permissive();
        }
        let origins: Vec<HeaderValue> = trimmed
            .split(',')
            .filter_map(|s| HeaderValue::from_str(s.trim()).ok())
            .collect();
        if !origins.is_empty() {
            return CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(Any);
        }
    }
    CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            HeaderValue::from_static("http://localhost:3000"),
            HeaderValue::from_static("http://127.0.0.1:3000"),
        ]))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
}

pub fn build_router(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route(
            "/v1/workflows",
            post(crate::routes::workflows::create).get(crate::routes::workflows::list),
        )
        .route(
            "/v1/workflows/:id",
            get(crate::routes::workflows::get)
                .patch(crate::routes::workflows::update)
                .delete(crate::routes::workflows::delete_wf),
        )
        .route(
            "/v1/workflows/:id/trigger",
            post(crate::routes::workflows::trigger),
        )
        .route("/v1/runs", get(crate::routes::runs::list))
        .route("/v1/runs/:run_id", get(crate::routes::runs::get_one))
        .route(
            "/v1/runs/:run_id/logs",
            get(crate::routes::runs::stream_run_logs),
        )
        .route(
            "/v1/runs/:run_id/approve",
            post(crate::routes::runs::approve_run),
        )
        .route(
            "/v1/runs/:run_id/reject",
            post(crate::routes::runs::reject_run),
        )
        .route("/v1/analytics", get(crate::routes::analytics::get_analytics))
        .route("/v1/orgs/me", get(crate::routes::orgs::me))
        .route(
            "/v1/orgs/me/api_keys",
            post(crate::routes::orgs::create_key).get(crate::routes::orgs::list_keys),
        )
        .route(
            "/v1/orgs/me/api_keys/:id",
            delete(crate::routes::orgs::revoke_key),
        )
        .route(
            "/v1/orgs/me/credits",
            get(crate::routes::credits::get_credits),
        )
        .route(
            "/v1/orgs/me/credits/topup_info",
            get(crate::routes::credits::topup_info),
        )
        .route(
            "/v1/orgs/me/credits/topup",
            post(crate::routes::credits::topup),
        )
        .route(
            "/v1/orgs/me/credits/grant",
            post(crate::routes::credits::admin_grant),
        )
        .route("/v1/hub/publish", post(crate::routes::hub::publish))
        .route("/v1/hub/:id/call", post(crate::routes::hub::call))
        .route("/v1/execute/program", post(crate::routes::execute::program))
        .route(
            "/v1/execute/transfer",
            post(crate::routes::execute::transfer),
        )
        .layer(ax_mw::from_fn(rate_limit_middleware))
        .layer(ax_mw::from_fn_with_state(state.clone(), auth_middleware));

    let public_routes = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/hub", get(crate::routes::hub::list))
        .route(
            "/v1/hub/:id/payment_info",
            get(crate::routes::hub::payment_info),
        )
        .route(
            "/v1/webhooks/:workflow_id",
            post(crate::routes::webhooks::receive_webhook),
        );

    Router::new()
        .merge(auth_routes)
        .merge(public_routes)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
}
