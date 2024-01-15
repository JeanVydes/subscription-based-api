use axum::BoxError;
use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::{Router, routing::{get, post}};
use crate::controllers::identity::{request_credentials, get_session};

use crate::server::AppState;
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/identity
pub async fn get_identity_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/session",
            post({
                let app_state = Arc::clone(&app_state);
                move |payload| request_credentials(payload, app_state)
            }),
        )
        .route(
            "/session",
            get({
                let app_state = Arc::clone(&app_state);
                move |headers| get_session(headers, app_state)
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