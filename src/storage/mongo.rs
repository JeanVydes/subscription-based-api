use axum::{Json, http::StatusCode};
use mongodb::{
    bson::{doc, Document}, options::ClientOptions, options::ServerApi, options::ServerApiVersion, Client, Database, Collection,
};
use serde_json::json;

use std::env;

use crate::types::customer::{GenericResponse, Customer};

pub async fn init_connection() -> mongodb::error::Result<Client> {
    let uri = match env::var("MONGO_URI") {
        Ok(uri) => uri,
        Err(_) => String::from("mongo_uri not found"),
    };

    let mut client_options = ClientOptions::parse(&uri).await?;

    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    let client = Client::with_options(client_options)?;

    client
        .database("admin")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    Ok(client)
}

pub async fn build_customer_filter(id: &str, email: &str) -> Document {
    let customer_filter = doc! {"$or": [
        {"id": id},
        {
            "emails": {
                "$elemMatch": {
                    "address": email,
                }
            }
        }
    ]};

    return customer_filter
}

pub async fn get_customers_collection(db: &Database) -> Collection<Customer> {
    return db.collection("customers");
}

pub async fn find_customer(db: &Database, filter: Document) -> Result<(bool, Option<Customer>), (StatusCode, Json<GenericResponse>)> {
    let collection = get_customers_collection(db).await;
    match collection.find_one(filter, None).await {
        Ok(customer) => match customer {
            Some(customer) => Ok((true, Some(customer))),
            None => Ok((false, None)),
        },
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error fetching customer"),
                    data: json!({}),
                    exit_code: 1,
                }),
            ));
        },
    }
}

pub async fn update_customer(db: &Database, filter: Document, update: Document) -> Result<(), (StatusCode, Json<GenericResponse>)> {
    let collection = get_customers_collection(db).await;
    match collection.update_one(filter, update, None).await {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!("error updating customer: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error updating record in database"),
                    data: json!({}),
                    exit_code: 1,
                }),
            ));
        }
    }
}