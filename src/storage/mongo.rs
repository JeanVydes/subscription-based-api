use mongodb::{
    bson::doc, options::ClientOptions, options::ServerApi, options::ServerApiVersion, Client,
};

use std::env;

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
