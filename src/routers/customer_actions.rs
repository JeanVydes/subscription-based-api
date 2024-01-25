use axum::{BoxError, Json};
use axum::error_handling::HandleErrorLayer;
use axum::extract::rejection::JsonRejection;
use axum::http::{StatusCode, HeaderMap};
use axum::{Router, routing::{get, patch}};
use crate::controllers::customer::{update_name, update_password};
use crate::controllers::email::{add_email, verify_email};
use crate::server::AppState;
use crate::types::incoming_requests::{CustomerUpdateName, CustomerUpdatePassword, CustomerAddEmail};
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/me
pub async fn get_customer_actions_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/update/name",
            patch({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<CustomerUpdateName>, JsonRejection>)| {
                    update_name(headers, payload, app_state)
                }
            }),
        )
        .route(
            "/update/password",
            patch({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<CustomerUpdatePassword>, JsonRejection>)| {
                    update_password(headers, payload, app_state)
                }
            }),
        )
        .route(
            "/add/email",
            patch({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<CustomerAddEmail>, JsonRejection>)| {
                    add_email(headers, payload, app_state)
                }
            }),
        )
        .route(
            "/verify/email",
            get({
                let app_state = Arc::clone(&app_state);
                move |query_params| {
                   verify_email(query_params, app_state)
                }
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
                .layer(RateLimitLayer::new(10, Duration::from_secs(60))),
        );
}