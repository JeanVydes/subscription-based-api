use redis::{Client, RedisError};
use std::env;

pub fn init_connection() -> Result<Client, RedisError> {
    let uri = match env::var("REDIS_URI") {
        Ok(uri) => uri,
        Err(_) => panic!("REDIS_URI not found"),
    };

    let client = Client::open(uri)?;

    println!("Connected to Redis");

    Ok(client)
}
