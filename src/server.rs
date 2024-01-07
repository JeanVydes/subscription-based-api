use crate::{identity_routers::{request_credentials, get_session, identity_middleware}, account::Account, account_routers::create_account, helpers::fallback};
use axum::{routing::{get, post}, Router, http::Method, middleware};
use mongodb::{Client as MongoClient, Collection, Database};
use redis::Client as RedisClient;
use std::{env, sync::Arc};
use tower_http::{cors::{Any, CorsLayer}, compression::CompressionLayer};

#[derive(Clone)]
pub struct AppState {
    pub mongodb_client: MongoClient,
    pub redis_connection: RedisClient,
    pub mongo_db: Database,
    pub last_account_id: u64,
}

pub async fn init(mongodb_client: MongoClient, redis_client: RedisClient) {
    let mongo_db = match env::var("MONGO_DB_NAME") {
        Ok(db) => db,
        Err(_) => panic!("mongo_db_name not found"),
    };

    let mongo_db = mongodb_client.database(&mongo_db);
    let accounts_collection: Collection<Account> = mongo_db.collection("accounts");
    let count = match accounts_collection.count_documents(None, None).await {
        Ok(count) => count,
        Err(e) => panic!("Error counting documents: {}", e),
    };

    let app_state = Arc::new(AppState {
        mongodb_client: mongodb_client.clone(),
        redis_connection: redis_client.clone(),
        mongo_db,
        last_account_id: count,
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

    let api = Router::new()
        .nest("/accounts", accounts)
        .nest("/identity", identity);
        
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
