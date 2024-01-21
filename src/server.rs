use crate::{
    utilities::helpers::fallback,
    types::lemonsqueezy::Products, 
    routers::{
        identity::get_identity_router, 
        customer_actions::get_customer_actions_router, 
        customers::get_customers_router, 
        webhooks::get_webhooks_router
    },
};
use axum::{
    http::Method,
    routing::get,
    Router,
};
use diesel::{r2d2::ConnectionManager, PgConnection};
use mongodb::{Client as MongoClient, Database};
use r2d2::Pool;
use redis::Client as RedisClient;
use std::{env, sync::Arc};

use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use log::info;

#[derive(Clone)]
pub struct AppState {
    pub mongodb_client: MongoClient,
    pub redis_connection: RedisClient,
    pub postgres_conn: Option<Pool<ConnectionManager<PgConnection>>>,
    pub mongo_db: Database,
    pub lemonsqueezy_webhook_signature_key: String,
    pub products: Products,
}

pub async fn init(mongodb_client: MongoClient, redis_connection: RedisClient, postgres_conn: Option<Pool<ConnectionManager<PgConnection>>>) {
    let app_state = set_app_state(mongodb_client, redis_connection, postgres_conn).await;

    // show products, for testing purposes
    info!("Products: {:?}", app_state.products);

    // /api/customers
    let customers = get_customers_router(app_state.clone()).await;
    info!("Customers router loaded");
    // /api/me
    let customers_actions = get_customer_actions_router(app_state.clone()).await;
    info!("Customers actions router loaded");
    // /api/identity
    let identity = get_identity_router(app_state.clone()).await;
    info!("Identity router loaded");
    // /api/webhooks
    let webhooks = get_webhooks_router(app_state.clone()).await;
    info!("Webhooks router loaded");
    // /api
    let api = Router::new()
        .nest("/customers", customers)
        .nest("/me", customers_actions)
        .nest("/identity", identity)
        .nest("/webhooks", webhooks);

    info!("API router loaded");

    let cors = CorsLayer::new()
        .allow_credentials(false)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
        .allow_origin(Any);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/api", api)
        .layer(cors)
        .layer(CompressionLayer::new())
        .fallback(fallback)
        .with_state(app_state);

    let host = env::var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
    let port = env::var("PORT").unwrap_or_else(|_| String::from("8080"));
    let address = format!("{}:{}", host, port);

    info!("Starting server on {}", address);

    match axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
    {
        Ok(_) => {},
        Err(e) => panic!("Error starting server: {}", e),
    };
}

pub async fn set_app_state(mongodb_client: MongoClient, redis_connection: RedisClient, postgres_conn: Option<Pool<ConnectionManager<PgConnection>>>) -> Arc<AppState> {
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
        Ok(id) => id.parse::<i64>().unwrap(),
        Err(_) => panic!("pro_product_id not found"),
    };

    let pro_monthly_variant_id = match env::var("PRO_MONTHLY_VARIANT_ID") {
        Ok(id) => id.parse::<i64>().unwrap(),
        Err(_) => panic!("pro_monthly_variant_id not found"),
    };

    let pro_annually_variant_id = match env::var("PRO_ANNUALLY_VARIANT_ID") {
        Ok(id) => id.parse::<i64>().unwrap(),
        Err(_) => panic!("pro_annually_variant_id not found"),
    };

    let products = Products {
        pro_product_id,
        pro_monthly_variant_id,
        pro_annually_variant_id,
    };

    let app_state = Arc::new(AppState {
        mongodb_client,
        redis_connection,
        postgres_conn,
        mongo_db,
        lemonsqueezy_webhook_signature_key,
        products,
    });

    return app_state;
}