use crate::{identity_routers::{request_credentials, get_session, identity_middleware}, account_routers::create_account, helpers::fallback, webhook_listener::{orders_webhook_events_listener, subscription_webhook_events_listener}, lemonsqueezy::{OrderEvent, SubscriptionEvent}};
use axum::{routing::{get, post}, Router, http::{Method, HeaderMap}, middleware, Json, extract::rejection::JsonRejection};
use mongodb::{Client as MongoClient, Database};
use redis::Client as RedisClient;
use std::{env, sync::Arc};
use tower_http::{cors::{Any, CorsLayer}, compression::CompressionLayer};

#[derive(Clone)]
pub struct Products {
    pub pro_product_id: u64,
    pub pro_monthly_variant_id: u64,
    pub pro_annually_variant_id: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub mongodb_client: MongoClient,
    pub redis_connection: RedisClient,
    pub mongo_db: Database,
    pub lemonsqueezy_webhook_signature_key: String,
    pub products: Products,
}

pub async fn init(mongodb_client: MongoClient, redis_client: RedisClient) {
    let mongo_db = match env::var("MONGO_DB_NAME") {
        Ok(db) => db,
        Err(_) => panic!("mongo_db_name not found"),
    };

    let mongo_db = mongodb_client.database(&mongo_db);
    let lemonsqueezy_webhook_signature_key = match env::var("LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY") {
        Ok(uri) => uri,
        Err(_) => String::from("lemonsqueezy_webhook_signature_key not found"),
    };

    let pro_product_id = match env::var("PRO_PRODUCT_ID") {
        Ok(id) => id.parse::<u64>().unwrap(),
        Err(_) => panic!("pro_product_id not found"),
    };

    let pro_monthly_variant_id = match env::var("PRO_MONTHLY_VARIANT_ID") {
        Ok(id) => id.parse::<u64>().unwrap(),
        Err(_) => panic!("pro_monthly_variant_id not found"),
    };

    let pro_annually_variant_id = match env::var("PRO_ANNUALLY_VARIANT_ID") {
        Ok(id) => id.parse::<u64>().unwrap(),
        Err(_) => panic!("pro_annually_variant_id not found"),
    };

    let products = Products {
        pro_product_id,
        pro_monthly_variant_id,
        pro_annually_variant_id,
    };

    let app_state = Arc::new(AppState {
        mongodb_client: mongodb_client.clone(),
        redis_connection: redis_client.clone(),
        mongo_db,
        lemonsqueezy_webhook_signature_key,
        products,
    });
    
    let accounts = Router::new()
        .route(
            "/account",
            post({
                let app_state = Arc::clone(&app_state);
                move |payload| create_account(payload, app_state)
            }),
        )
        .route_layer(middleware::from_fn_with_state(app_state.redis_connection.clone(), identity_middleware));

    let identity = Router::new()
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
                move |payload| get_session(payload, app_state)
            }),
        );

    let webhooks = Router::new()
        .route(
            "/lemonsqueezy/events/orders",
            get({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<OrderEvent>, JsonRejection>)| {
                    orders_webhook_events_listener(headers, payload, app_state)
                }
            }),
        )
        .route(
            "/lemonsqueezy/events/subscriptions",
            get({
                let app_state = Arc::clone(&app_state);
                move |(headers, payload): (HeaderMap, Result<Json<SubscriptionEvent>, JsonRejection>)| {
                    subscription_webhook_events_listener(headers, payload, app_state)
                }
            }),
        );

    let api = Router::new()
        .nest("/accounts", accounts)
        .nest("/identity", identity)
        .nest("/webhooks", webhooks);
        
    let cors = CorsLayer::new()
        .allow_credentials(false)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(Any);

    let app = Router::new()
        .nest("/api", api)
        .layer(cors)
        .layer(CompressionLayer::new())
        .fallback(fallback)
        .with_state(app_state);

    let host = env::var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
    let port = env::var("PORT").unwrap_or_else(|_| String::from("3000"));
    let address = format!("{}:{}", host, port);

    match axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
    {
        Ok(_) => println!("Server running on {}", address),
        Err(e) => println!("Error starting server: {}", e),
    };
}
