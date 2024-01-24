mod server;

mod controllers;
mod lemonsqueezy;
mod storage;
mod types;
mod utilities;
mod routers;
mod email;
mod oauth;

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

    env::var("API_URL").expect("API_URL must be set");
    env::var("MONGO_URI").expect("DATABASE_URL must be set");
    env::var("MONGO_DB_NAME").expect("DB_NAME must be set");
    env::var("REDIS_URI").expect("REDIS_URI must be set");

    env::var("API_TOKENS_SIGNING_KEY").expect("API_SIGNING_KEY must be set");
    env::var("LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY").expect("LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY must be set");

    let email_integration = match env::var("ENABLE_EMAIL_INTEGRATION").expect("ENABLE_EMAIL_INTEGRATION must be set").parse::<bool>() {
        Ok(val) => val,
        Err(_) => panic!("ENABLE_EMAIL_INTEGRATION must be a boolean"),
    };

    let created_customer_list = env::var("BREVO_CUSTOMERS_LIST_ID");
    let api_key = env::var("BREVO_CUSTOMERS_WEBFLOW_API_KEY");
    let master_email_address = env::var("BREVO_MASTER_EMAIL_ADDRESS");
    let master_name = env::var("BREVO_MASTER_NAME");
    let brevo_email_verification_template_id = env::var("BREVO_EMAIL_VERIFY_TEMPLATE_ID");

    if email_integration {
        if api_key.is_err() {
            warn!("BREVO_CUSTOMERS_WEBFLOW_API_KEY isn't set, skipping Brevo integration, including email verification");
        }
    
        if api_key.is_ok() && created_customer_list.is_err() {
            env::set_var("BREVO_CUSTOMERS_LIST_ID", "1");
            warn!("BREVO_CUSTOMERS_LIST_ID isn't set, using default list id: 1");
        }

        if master_email_address.is_err() {
            env::set_var("BREVO_MASTER_EMAIL_ADDRESS", "test@example.com");
            warn!("BREVO_MASTER_EMAIL_ADDRESS isn't set, using default email address: test@example.com");
        }

        if master_name.is_err() {
            env::set_var("BREVO_MASTER_NAME", "My Example Company");
            warn!("BREVO_MASTER_NAME isn't set, using default name: My Example Company");
        }

        if brevo_email_verification_template_id.is_err() {
            env::set_var("BREVO_EMAIL_VERIFY_TEMPLATE_ID", "1");
            warn!("BREVO_EMAIL_VERIFY_TEMPLATE_ID isn't set, using default template id: 1");
        }
    }

    env::var("GOOGLE_OAUTH_CLIENT_ID").expect("GOOGLE_OAUTH_CLIENT_ID must be set");
    env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("GOOGLE_OAUTH_CLIENT_SECRET must be set");
    env::var("GOOGLE_OAUTH_CLIENT_REDIRECT_ENDPOINT").expect("GOOGLE_CLIENT_OAUTH_REDIRECT_URL must be set");
        
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