use axum::extract::Query;
use axum::BoxError;
use axum::error_handling::HandleErrorLayer;
use axum::http::{HeaderMap, StatusCode};
use axum::{Router, routing::get};
use crate::controllers::customer::fetch_customer_record_by_id;

use crate::server::AppState;
use crate::types::incoming_requests::FetchCustomerByID;
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/public
pub async fn get_public_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/fetch/customer/by/id",
            get({
                let app_state = Arc::clone(&app_state);
                move |(headers, query): (HeaderMap, Query<FetchCustomerByID>)| fetch_customer_record_by_id(headers, query, app_state)
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
                .layer(BufferLayer::new(128))
                .layer(RateLimitLayer::new(15, Duration::from_secs(60))),
        );
}