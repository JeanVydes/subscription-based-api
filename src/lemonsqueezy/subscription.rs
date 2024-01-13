use std::sync::Arc;

use axum::Json;
use mongodb::{
    bson::{doc, to_bson, Bson},
    Collection,
};
use serde_json::json;

use crate::{
    helpers::random_string,
    server::AppState,
    types::{
        account::{Account, GenericResponse},
        lemonsqueezy::SubscriptionEvent,
        subscription::{Slug, Subscription, SubscriptionFrequencyClass, SubscriptionHistoryLog},
    },
};

pub async fn subscription_created(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
        Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(Json(GenericResponse {
                    message: String::from("invalid customer_id: not records"),
                    data: json!({}),
                    exited_code: 1,
                }))
            }
        },
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    };

    let frequency: SubscriptionFrequencyClass;
    if event.data.attributes.variant_id == state.products.pro_monthly_variant_id {
        frequency = SubscriptionFrequencyClass::MONTHLY;
    } else if event.data.attributes.variant_id == state.products.pro_annually_variant_id {
        frequency = SubscriptionFrequencyClass::ANNUALLY;
    } else {
        return Err(Json(GenericResponse {
            message: String::from("invalid variant_id"),
            data: json!({}),
            exited_code: 1,
        }));
    }

    let subscription_id = random_string(15).await;
    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let mut slug = Slug::FREE.to_string();
    if event.data.attributes.product_id == state.products.pro_product_id {
        slug = Slug::PRO.to_string();
    }

    let ends_at = match event.data.attributes.ends_at {
        Some(ends_at) => ends_at,
        None => "".to_string(),
    };

    let update_subscription = Subscription {
        id: subscription_id,
        product_id: event.data.attributes.product_id,
        variant_id: event.data.attributes.variant_id,
        slug,
        frequency,
        status: event.data.attributes.status,
        created_at: customer.created_at,
        updated_at: event.data.attributes.updated_at,
        starts_at: event.data.attributes.created_at,
        ends_at,
        renews_at: event.data.attributes.renews_at,
        history_logs,
    };

    let update_subscription = match to_bson(&update_subscription) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(Json(GenericResponse {
                message: String::from("error converting suscription struct to bson"),
                data: json!({}),
                exited_code: 1,
            }))
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
        Ok(_) => {}
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer suscription"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    }

    Ok(())
}

pub async fn subscription_updated(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
        Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(Json(GenericResponse {
                    message: String::from("invalid customer_id: not records"),
                    data: json!({}),
                    exited_code: 1,
                }))
            }
        },
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(Json(GenericResponse {
                message: String::from("error converting suscription struct to bson"),
                data: json!({}),
                exited_code: 1,
            }))
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
        Ok(_) => {}
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer suscription"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    }

    Ok(())
}

pub async fn subscription_update_status(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
        Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(Json(GenericResponse {
                    message: String::from("invalid customer_id: not records"),
                    data: json!({}),
                    exited_code: 1,
                }))
            }
        },
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(Json(GenericResponse {
                message: String::from("error converting suscription struct to bson"),
                data: json!({}),
                exited_code: 1,
            }))
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
        Ok(_) => {}
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer suscription"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    }

    Ok(())
}

pub async fn subscription_update_history_logs(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"id": &customer_id},
    ]};

    let customer = match collection.find_one(filter, None).await {
        Ok(account) => match account {
            Some(acc) => acc,
            None => {
                return Err(Json(GenericResponse {
                    message: String::from("invalid customer_id: not records"),
                    data: json!({}),
                    exited_code: 1,
                }))
            }
        },
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    };

    let mut history_logs = customer.subscription.history_logs.clone();
    history_logs.push(SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    });

    let bson_history_logs = match to_bson(&history_logs) {
        Ok(Bson::Document(document)) => document,
        _ => {
            return Err(Json(GenericResponse {
                message: String::from("error converting suscription struct to bson"),
                data: json!({}),
                exited_code: 1,
            }))
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
        Ok(_) => {}
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer suscription"),
                data: json!({}),
                exited_code: 1,
            }))
        }
    }

    Ok(())
}
