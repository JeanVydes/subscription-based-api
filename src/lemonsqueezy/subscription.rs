use std::sync::Arc;

use axum::Json;
use mongodb::bson::{doc, to_bson, Bson};
use serde_json::json;

use crate::{
    utilities::helpers::{random_string, add_subscription_history_log_and_to_bson},
    server::AppState,
    types::{
        customer::GenericResponse,
        lemonsqueezy::SubscriptionEvent,
        subscription::{Slug, Subscription, SubscriptionFrequencyClass, SubscriptionHistoryLog},
    }, storage::mongo::{build_customer_filter, find_customer, update_customer},
};

pub async fn subscription_created(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let filter = build_customer_filter(customer_id.as_str(), event.data.attributes.user_email.as_str()).await;
    let (found, customer) = match find_customer(&state.mongo_db, filter.clone()).await {
        Ok(customer) => customer,
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exit_code: 1,
            }));
        }
    };

    if !found {
        return Err(Json(GenericResponse {
            message: String::from("invalid customer_id: not records"),
            data: json!({}),
            exit_code: 1,
        }));
    }

    let frequency: SubscriptionFrequencyClass;
    if event.data.attributes.variant_id == state.products.pro_monthly_variant_id {
        frequency = SubscriptionFrequencyClass::MONTHLY;
    } else if event.data.attributes.variant_id == state.products.pro_annually_variant_id {
        frequency = SubscriptionFrequencyClass::ANNUALLY;
    } else {
        return Err(Json(GenericResponse {
            message: String::from("invalid variant_id"),
            data: json!({}),
            exit_code: 1,
        }));
    }

    let customer = customer.unwrap();

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
                message: String::from("error converting subscription struct to bson"),
                data: json!({}),
                exit_code: 1,
            }))
        }
    };

    let update = doc! {
        "$set": doc!{
            "subscription": update_subscription
        },
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => Ok(()),
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer subscription"),
                data: json!({}),
                exit_code: 1,
            }))
        }
    }
}

pub async fn subscription_updated(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let filter = build_customer_filter(customer_id.as_str(), event.data.attributes.user_email.as_str()).await;

    let (found, customer) = match find_customer(&state.mongo_db, filter.clone()).await {
        Ok(customer) => customer,
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exit_code: 1,
            }));
        }
    };

    if !found {
        return Err(Json(GenericResponse {
            message: String::from("invalid customer_id: not records"),
            data: json!({}),
            exit_code: 1,
        }));
    }

    let customer = customer.unwrap();
    let bson_history_logs = add_subscription_history_log_and_to_bson(customer.subscription.history_logs, SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    }).await;

    let update = doc! {
        "$set": doc!{
            "subscription.variant_id": event.data.attributes.variant_id as i64,
            "subscription.status": event.data.attributes.status,
            "subscription.updated_at": event.data.attributes.updated_at,
            "subscription.history_logs": bson_history_logs,
        },
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => Ok(()),
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer subscription"),
                data: json!({}),
                exit_code: 1,
            }))
        }
    }
}

// ready
pub async fn subscription_update_status(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let filter = build_customer_filter(customer_id.as_str(), event.data.attributes.user_email.as_str()).await;

    let (found, customer) = match find_customer(&state.mongo_db, filter.clone()).await {
        Ok(customer) => customer,
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exit_code: 1,
            }));
        }
    };

    if !found {
        return Err(Json(GenericResponse {
            message: String::from("invalid customer_id: not records"),
            data: json!({}),
            exit_code: 1,
        }));
    }

    let customer = customer.unwrap();
    let bson_history_logs = add_subscription_history_log_and_to_bson(customer.subscription.history_logs, SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    }).await;

    let update = doc! {
        "$set": doc!{
            "subscription.status": event.data.attributes.status.clone(),
            "subscription.updated_at": event.data.attributes.updated_at,
            "subscription.history_logs": bson_history_logs,
        },
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => Ok(()),
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer subscription"),
                data: json!({}),
                exit_code: 1,
            }))
        }
    }
}

pub async fn subscription_update_history_logs(
    event: SubscriptionEvent,
    state: Arc<AppState>,
) -> Result<(), Json<GenericResponse>> {
    let customer_id = event.meta.custom_data.unwrap().customer_id;
    let filter = build_customer_filter(customer_id.as_str(), event.data.attributes.user_email.as_str()).await;
    let (found, customer) = match find_customer(&state.mongo_db, filter.clone()).await {
        Ok(customer) => customer,
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error checking customer existence"),
                data: json!({}),
                exit_code: 1,
            }));
        }
    };

    if !found {
        return Err(Json(GenericResponse {
            message: String::from("invalid customer_id: not records"),
            data: json!({}),
            exit_code: 1,
        }));
    }

    let customer = customer.unwrap();
    let bson_history_logs = add_subscription_history_log_and_to_bson(customer.subscription.history_logs, SubscriptionHistoryLog {
        event: event.meta.event_name,
        date: event.data.attributes.updated_at.clone(),
    }).await;

    let update = doc!  {
        "$set": doc!{
            "subscription.updated_at": event.data.attributes.updated_at,
            "subscription.history_logs": bson_history_logs,
        },
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => Ok(()),
        Err(_) => {
            return Err(Json(GenericResponse {
                message: String::from("error updating customer subscription"),
                data: json!({}),
                exit_code: 1,
            }))
        }
    }
}
