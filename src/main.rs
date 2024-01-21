mod server;

mod controllers;
mod lemonsqueezy;
mod storage;
mod types;
mod utilities;
mod routers;

use std::env;
use chrono::Local;
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::Pool;
use storage::{mongo, redis, diesel_postgres};
use log::{warn, info, debug};

#[tokio::main]
async fn main() {
    let _ = configure_logger().await;
    debug!("Logger configured");

    debug!("Loading environment variables...");
    let postgres_uri = load_env().await;
    debug!("Environment variables loaded");

    debug!("Connecting to MongoDB...");
    let mongo_client = match mongo::init_connection().await {
        Ok(client) => {
            match client
                .database("admin")
                .run_command(mongodb::bson::doc! {"ping": 1}, None)
                .await
            {
                Ok(_) => info!("Connected to MongoDB"),
                Err(e) => panic!("Error verifying connecting to MongoDB: {}", e),
            };

            client
        },
        Err(e) => panic!("Error connecting to MongoDB: {}", e),
    };

    debug!("Connecting to Redis...");
    let redis_connection = match redis::init_connection() {
        Ok(redis_connection) => {
            info!("Connected to Redis");
            redis_connection
        },
        Err(e) => panic!("Error connecting to Redis: {}", e),
    };

    let mut postgres_conn: Option<Pool<ConnectionManager<PgConnection>>> = None;
    if !postgres_uri.is_empty() {
        debug!("Connecting to PostgreSQL...");
        postgres_conn = match diesel_postgres::new_connection(&postgres_uri).await {
            Ok(conn) => {
                info!("Connected to PostgreSQL");
                Option::from(conn)
            },
            Err(e) => {
                panic!("Error connecting to PostgreSQL: {}", e);
            },
        };
    } else {
        warn!("PostgreSQL connection string not found, skipping connection...");
    }

    debug!("Starting server...");
    server::init(mongo_client, redis_connection, postgres_conn).await;
}

async fn load_env() -> String {
    dotenv::dotenv().ok();
    env::var("HOST").expect("ADDRESS must be set");
    let port = env::var("PORT").expect("PORT must be set");
    match port.parse::<u16>() {
        Ok(_) => (),
        Err(_) => panic!("PORT must be a number"),
    };

    let postgres_uri = match env::var("POSTGRES_URI") {
        Ok(val) => val,
        Err(_) => String::new(),
    };

    env::var("MONGO_URI").expect("DATABASE_URL must be set");
    env::var("MONGO_DB_NAME").expect("DB_NAME must be set");
    env::var("REDIS_URI").expect("REDIS_URI must be set");

    env::var("API_TOKENS_SIGNING_KEY").expect("API_SIGNING_KEY must be set");
    env::var("LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY").expect("LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY must be set");

        
    let expiration_time = match env::var("API_TOKENS_EXPIRATION_TIME") {
        Ok(expiration_time) => expiration_time,
        Err(_) => panic!("API_TOKENS_EXPIRATION_TIME must be set"),
    };

    match expiration_time.parse::<usize>() {
        Ok(_) => (),
        Err(_) => panic!("API_TOKENS_EXPIRATION_TIME must be a number"),
    };

    return postgres_uri
}

async fn configure_logger() -> Result<(), fern::InitError>  {
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("hyper", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;

    Ok(())
}