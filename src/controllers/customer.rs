use crate::utilities::helpers::{payload_analyzer, random_string, valid_password, valid_email, parse_class};
use crate::storage::mongo::{build_customer_filter, find_customer, update_customer};
use crate::types::customer::{Customer, Email, Preferences};
use crate::types::incoming_requests::{CreateCustomerRecord, CustomerUpdateName, CustomerUpdatePassword, CustomerAddEmail};
use crate::types::subscription::{Slug, Subscription, SubscriptionFrequencyClass};
use crate::{server::AppState, types::customer::GenericResponse};

use axum::http::HeaderMap;
use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use chrono::Utc;
use mongodb::bson::doc;
use regex::Regex;
use serde_json::json;
use std::sync::Arc;

use bcrypt::{hash, DEFAULT_COST};

use super::identity::get_user_id_from_req;

pub async fn create_customer_record(
    payload_result: Result<Json<CreateCustomerRecord>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if !payload.accepted_terms {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("you must accept the terms of service and privacy"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.name.len() < 2 || payload.name.len() > 25 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid name, must be at least 2 characters and at most 15"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    match valid_email(&payload.email).await {
        Ok(_) => (),
        Err((status_code, json)) => return (status_code, json),
    };

    match valid_password(&payload.password).await {
        Ok(_) => (),
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.password != payload.password_confirmation {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("password and password confirmation must match"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.email.to_lowercase() == payload.password.to_lowercase() {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("email and password must be different"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let filter = build_customer_filter("", payload.email.to_lowercase().as_str()).await;
    let (found, _) = match find_customer(state.mongo_db.clone(), filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
    };

    if found {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("email already taken"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let hashed_password = match hash(&payload.password, DEFAULT_COST) {
        Ok(hashed_password) => hashed_password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error hashing password"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let emails = vec![Email {
        address: payload.email.to_lowercase(),
        verified: false,
        main: true,
    }];

    let class = match parse_class(&payload.class).await {
        Ok(class) => class,
        Err((status_code, json)) => return (status_code, json),
    };

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();
    let subscription_id = random_string(10).await;
    let subscription = Subscription {
        id: subscription_id,
        product_id: 0,
        variant_id: 0,
        slug: Slug::FREE.to_string(),
        frequency: SubscriptionFrequencyClass::UNDEFINED,
        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        starts_at: "".to_string(),
        ends_at: "".to_string(),
        renews_at: "".to_string(),
        status: "".to_string(),
        history_logs: vec![],
    };

    let id = random_string(30).await;
    let customer = Customer {
        id,
        name: payload.name.clone(),
        class,
        emails,

        password: hashed_password,
        backup_security_codes: vec![],

        preferences: Preferences {
            dark_mode: false,
            language: String::from("en"),
            notifications: true,
        },
        subscription,

        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        deleted: false,
    };

    let collection = state.mongo_db.collection("customers");
    match collection.insert_one(customer.clone(), None).await {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error inserting record into database"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    (
        StatusCode::CREATED,
        Json(GenericResponse {
            message: String::from("customer record registered successfully"),
            data: json!(customer),
            exited_code: 0,
        }),
    )
}

pub async fn update_name(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerUpdateName>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let customer_id = match get_user_id_from_req(headers, state.redis_connection.clone()).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.name.len() < 2 || payload.name.len() > 25 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid name, must be at least 2 characters and at most 15"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let update = doc! {"$set": {"name": &payload.name}};
    match update_customer(state.mongo_db.clone(), filter, update).await {
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: String::from("customer name updated successfully"),
                data: json!({}),
                exited_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
}

pub async fn update_password(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerUpdatePassword>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let customer_id = match get_user_id_from_req(headers, state.redis_connection.clone()).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let (found, customer) = match find_customer(state.mongo_db.clone(), filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
    };

    if !found {
        return (
            StatusCode::NOT_FOUND,
            Json(GenericResponse {
                message: String::from("customer not found"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.old_password.len() < 8 || payload.old_password.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid old password, must be at least 8 characters"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.new_password.len() < 8 || payload.new_password.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid new password, must be at least 8 characters"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let password_re = Regex::new(r"^[a-zA-Z0-9_]{8,20}$").unwrap();
    if !password_re.is_match(&payload.new_password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid new password"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.new_password == payload.old_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("new password and old password must be different"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.new_password != payload.new_password_confirmation {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("new password and new password confirmation must match"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let hashed_new_password = match hash(&payload.new_password, DEFAULT_COST) {
        Ok(hashed_password) => hashed_password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error hashing password"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let hashed_old_password = match hash(&payload.old_password, DEFAULT_COST) {
        Ok(hashed_password) => hashed_password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error hashing password"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    };

    let customer = customer.unwrap();
    if customer.password != hashed_old_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid old password"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let update = doc! {"$set": {"password": hashed_new_password}};
    match update_customer(state.mongo_db.clone(), filter, update).await {
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: String::from("customer password updated successfully"),
                data: json!({}),
                exited_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
}

pub async fn add_email(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerAddEmail>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    let customer_id = match get_user_id_from_req(headers, state.redis_connection.clone()).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let (found, customer) = match find_customer(state.mongo_db.clone(), filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
    };

    if !found {
        return (
            StatusCode::NOT_FOUND,
            Json(GenericResponse {
                message: String::from("customer not found"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    };

    let customer = customer.unwrap();
    let mut emails = customer.emails;
    if emails.len() >= 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("you can only have up to 5 emails"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    emails.push(Email {
        address: payload.email.to_lowercase(),
        verified: false,
        main: false,
    });

    let bson_emails = emails
        .iter()
        .map(|email| {
            doc! {
                "address": &email.address,
                "verified": &email.verified,
                "main": &email.main,
            }
        })
        .collect::<Vec<_>>();

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let update = doc! {"$set": {"emails": &bson_emails}};
    match update_customer(state.mongo_db.clone(), filter, update).await {
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: String::from("customer email updated successfully"),
                data: json!({}),
                exited_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
}
