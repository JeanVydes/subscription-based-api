use std::sync::Arc;

use axum::Json;
use mongodb::{bson::{doc, Bson, to_bson}, Collection};
use serde_json::json;
use serde::{Deserialize, Serialize};

use crate::{server::AppState, account::{Account, GenericResponse}, subscription::{Subscription, SubscriptionFrequencyClass, SubscriptionHistoryLog, Slug}, helpers::random_string};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub meta: Meta,
    pub data: OrderData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    pub meta: Meta,
    pub data: SubscriptionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub event_name: String,
    pub custom_data: CustomData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomData {
    pub customer_id: String, // same as acount id, this is using custom data, read more here https://docs.lemonsqueezy.com/help/checkout/passing-custom-data
}


///////////
// Order //
///////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub r#type: String,
    pub id: String,
    pub attributes: OrderAttributes,
    pub relationships: Relationships,
    pub links: Links,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAttributes {
    pub store_id: u64,
    pub customer_id: u64,
    pub identifier: String,
    pub order_number: u64,
    pub user_name: String,
    pub user_email: String,
    pub currency: String,
    pub currency_rate: String,
    pub subtotal: u64,
    pub discount_total: u64,
    pub tax: u64,
    pub total: u64,
    pub subtotal_usd: u64,
    pub discount_total_usd: u64,
    pub tax_usd: u64,
    pub total_usd: u64,
    pub tax_name: String,
    pub tax_rate: String,
    pub status: String,
    pub status_formatted: String,
    pub refunded: bool,
    pub refunded_at: String,
    pub subtotal_formatted: String,
    pub discount_total_formatted: String,
    pub tax_formatted: String,
    pub total_formatted: String,
    pub first_order_item: OrderItem,
    pub urls: OrderUrls,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: u64,
    pub order_id: u64,
    pub product_id: u64,
    pub variant_id: u64,
    pub product_name: String,
    pub variant_name: String,
    pub price: u64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
    pub test_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUrls {
    pub receipt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationships {
    pub store: RelationshipLinks,
    pub customer: RelationshipLinks,
    pub order_items: RelationshipLinks,
    pub subscriptions: RelationshipLinks,
    pub license_keys: RelationshipLinks,
    #[serde(rename = "discount-redemptions")]
    pub discount_redemptions: RelationshipLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLinks {
    pub related: String,
    #[serde(rename = "self")]
    pub link_self: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    pub link_self: String,
}

/////////////////
// Subscription //
/////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionData {
    pub r#type: String,
    pub id: String,
    pub attributes: SubscriptionAttributes,
    pub relationships: Relationships,
    pub links: Links,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscriptionAttributes {
    pub store_id: u64,
    pub customer_id: u64,
    pub order_id: u64,
    pub order_item_id: u64,
    pub product_id: u64,
    pub variant_id: u64,
    pub product_name: String,
    pub variant_name: String,
    pub user_name: String,
    pub user_email: String,
    pub status: String,
    pub status_formatted: String,
    pub card_brand: String,
    pub card_last_four: String,
    pub pause: Option<String>,
    pub cancelled: bool,
    pub trial_ends_at: String,
    pub billing_anchor: u64,
    pub first_subscription_item: FirstSubscriptionItem,
    pub urls: SubscriptionUrls,
    pub renews_at: String,
    pub ends_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub test_mode: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FirstSubscriptionItem {
    pub id: u64,
    pub subscription_id: u64,
    pub price_id: u64,
    pub quantity: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscriptionUrls {
    pub update_payment_method: String,
    pub customer_portal: String,
}

// Subscription Manager
pub async fn subscription_created(event: SubscriptionEvent, state: Arc<AppState>) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
     Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(
                    Json(GenericResponse {
                        message: String::from("invalid customer_id: not records"),
                        data: json!({}),
                        exited_code: 1,
                    }),
                )
            },
        },
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error checking customer existence"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let frequency: SubscriptionFrequencyClass;
    if event.data.attributes.variant_id == state.products.pro_monthly_variant_id {
        frequency = SubscriptionFrequencyClass::MONTHLY;
    } else if event.data.attributes.variant_id == state.products.pro_annually_variant_id {
        frequency = SubscriptionFrequencyClass::ANNUALLY;
    } else {
        return Err(
            Json(GenericResponse {
                message: String::from("invalid variant_id"),
                data: json!({}),
                exited_code: 1,
            }),
        )
    }

    let subscription_id = random_string(15).await;
    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog{
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let mut slug = Slug::FREE.to_string();
    if event.data.attributes.product_id == state.products.pro_product_id {
        slug = Slug::PRO.to_string();
    }

    let update_subscription = Subscription{
        id: subscription_id,
        product_id: event.data.attributes.product_id,
        variant_id: event.data.attributes.variant_id,
        slug,
        frequency,
        status: event.data.attributes.status,
        created_at: customer.created_at,
        updated_at: event.data.attributes.updated_at,
        starts_at: event.data.attributes.created_at,
        ends_at: event.data.attributes.ends_at,
        renews_at: event.data.attributes.renews_at,
        history_logs,
    };


    let update_subscription = match to_bson(&update_subscription) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error converting suscription struct to bson"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let update = doc! {"$set": {
        "$set": doc!{
            "suscription": update_subscription
        },
    }};

    match collection.update_one(filter, update, None).await {
        Ok(_) => {},
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error updating customer suscription"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    Ok(())
}

pub async fn subscription_updated(event: SubscriptionEvent, state: Arc<AppState>) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
     Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(
                    Json(GenericResponse {
                        message: String::from("invalid customer_id: not records"),
                        data: json!({}),
                        exited_code: 1,
                    }),
                )
            },
        },
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error checking customer existence"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog{
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error converting suscription struct to bson"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let update = doc! {"$set": {
        "$set": doc!{
            "subscription": doc!{
                "variant_id": event.data.attributes.variant_id as i64,
                "status": event.data.attributes.status,
                "updated_at": event.data.attributes.updated_at,
                "history_logs": bson_history_logs,
            },
        },
    }};

    match collection.update_one(filter, update, None).await {
        Ok(_) => {},
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error updating customer suscription"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    Ok(())
}


pub async fn subscription_update_status(event: SubscriptionEvent, state: Arc<AppState>) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
     Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(
                    Json(GenericResponse {
                        message: String::from("invalid customer_id: not records"),
                        data: json!({}),
                        exited_code: 1,
                    }),
                )
            },
        },
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error checking customer existence"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog{
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error converting suscription struct to bson"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let update = doc! {"$set": {
        "$set": doc!{
            "subscription": doc!{
                "status": event.data.attributes.status,
                "updated_at": event.data.attributes.updated_at,
                "history_logs": bson_history_logs,
            },
        },
    }};

    match collection.update_one(filter, update, None).await {
        Ok(_) => {},
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error updating customer suscription"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    Ok(())
}

pub async fn subscription_update_history_logs(event: SubscriptionEvent, state: Arc<AppState>) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
     Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(
                    Json(GenericResponse {
                        message: String::from("invalid customer_id: not records"),
                        data: json!({}),
                        exited_code: 1,
                    }),
                )
            },
        },
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error checking customer existence"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog{
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error converting suscription struct to bson"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let update = doc! {"$set": {
        "$set": doc!{
            "subscription": doc!{
                "updated_at": event.data.attributes.updated_at,
                "history_logs": bson_history_logs,
            },
        },
    }};

    match collection.update_one(filter, update, None).await {
        Ok(_) => {},
        Err(_) => {
            return Err(
                Json(GenericResponse {
                    message: String::from("error updating customer suscription"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    Ok(())
}