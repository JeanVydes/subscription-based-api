use axum::{BoxError, Json};
use axum::error_handling::HandleErrorLayer;
use axum::extract::rejection::JsonRejection;
use axum::http::{StatusCode, HeaderMap};
use axum::{Router, routing::post};

use crate::lemonsqueezy::webhook::{orders_webhook_events_listener, subscription_webhook_events_listener};
use crate::server::AppState;
use crate::types::lemonsqueezy::{SubscriptionEvent, OrderEvent};
use std::{sync::Arc, time::Duration};

use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};

// /api/webhooks
pub async fn get_webhooks_router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    return Router::new()
        .route(
            "/lemonsqueezy/events/orders",
            post({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<OrderEvent>, JsonRejection>)| {
                    orders_webhook_events_listener(headers, payload, app_state)
                }
            }),
        )
        .route(
            "/lemonsqueezy/events/subscriptions",
            post({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (
                    HeaderMap,
                    Result<Json<SubscriptionEvent>, JsonRejection>,
                )| {
                    subscription_webhook_events_listener(headers, payload, app_state)
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
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(120, Duration::from_secs(60))),
        );
}