use axum::BoxError;
use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::{Router, routing::post};
use crate::controllers::customer::create_customer_record;

use crate::server::AppState;
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/identity
pub async fn get_customers_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/create",
            post({
                let app_state = Arc::clone(&app_state);
                move |payload| create_customer_record(payload, app_state)
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
                .layer(BufferLayer::new(32))
                .layer(RateLimitLayer::new(15, Duration::from_secs(60))),
        );
}