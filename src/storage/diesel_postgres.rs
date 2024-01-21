pub mod schemas;

use r2d2 as original_r2d2;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};

pub async fn new_connection(uri: &str) -> Result<Pool<ConnectionManager<PgConnection>>, original_r2d2::Error> {
    let manager = ConnectionManager::<PgConnection>::new(uri);
    let pool = match Pool::builder().build(manager) {
        Ok(pool) => pool,
        Err(err) => return Err(err),
    };

    Ok(pool)
}