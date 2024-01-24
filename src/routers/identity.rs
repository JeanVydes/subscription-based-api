use axum::BoxError;
use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::{Router, routing::{get, post, patch}};
use crate::controllers::identity::{get_session, gooogle_authentication, legacy_authentication, renew_session};

use crate::server::AppState;
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/identity
pub async fn get_identity_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/session/legacy",
            post({
                let app_state = Arc::clone(&app_state);
                move |payload| legacy_authentication(payload, app_state)
            }),
        )
        .route(
            "/session/legacy",
            get({
                let app_state = Arc::clone(&app_state);
                move |headers| get_session(headers, app_state)
            }),
        )
        .route(
            "/session/legacy",
            patch({
                let app_state = Arc::clone(&app_state);
                move |headers| renew_session(headers, app_state)
            }),
        )
        .route(
            "/session/google",
            get({
                let app_state = Arc::clone(&app_state);
                move |headers| gooogle_authentication(headers, app_state)
            }),
        )
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err: BoxError| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled error: {}", err),
                    )
                }))
                .layer(BufferLayer::new(256))
                .layer(RateLimitLayer::new(30, Duration::from_secs(60))),
        );
}