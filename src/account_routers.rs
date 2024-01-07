use crate::helpers::{payload_analyzer, random_string};
use crate::account::{Preferences, Account, Email, AccountType};
use crate::requests_interfaces::SignUp;
use crate::subscription::{Subscription, SubscriptionFrequencyClass};
use crate::{account::GenericResponse, server::AppState};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use chrono::Utc;
use mongodb::{bson::doc, Collection};
use regex::Regex;
use serde_json::json;
use std::sync::Arc;

use bcrypt::{hash, DEFAULT_COST};

pub async fn create_account(
    payload_result: Result<Json<SignUp>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.name.len() < 2 || payload.name.len() > 25 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from(
                    "invalid name, must be at least 2 characters and at most 15",
                ),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.email.len() < 5 || payload.email.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from(
                    "invalid email, must be at least 5 characters and at most 100",
                ),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if payload.password.len() < 8 || payload.password.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid password, must be at least 8 characters"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let email_re = Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$").unwrap();
    let password_re = Regex::new(r"^[a-zA-Z0-9_]{8,20}$").unwrap();

    if !email_re.is_match(&payload.email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid email"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    if !password_re.is_match(&payload.password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid password"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

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

    let collection: Collection<Account> = state.mongo_db.collection("accounts");
    let filter = doc! {"$or": [
        {"email": &payload.email.to_lowercase()},
    ]};

    match collection.find_one(filter, None).await {
        Ok(account) => match account {
            Some(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(GenericResponse {
                        message: String::from("email already taken"),
                        data: json!({}),
                        exited_code: 1,
                    }),
                )
            }
            None => (),
        },
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error checking email availability"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
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

    let emails = vec![Email{
        address: payload.email.to_lowercase(),
        verified: false,
        main: true,
    }];

    let class: AccountType;
    if payload.class == "personal" {
        class = AccountType::PERSONAL;
    } else if payload.class == "manager" {
        class = AccountType::MANAGER;
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("invalid account type"),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();
    let suscription_id = random_string(10).await;
    let suscription = Subscription {
        id: suscription_id,
        product_id: 0,
        variant_id: 0,
        frequency: SubscriptionFrequencyClass::UNDEFINED,
        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        starts_at: "".to_string(),
        ends_at: "".to_string(),
        renews_at: "".to_string(),
        active: true,
        history_logs: vec![],
    };

    let id = random_string(30).await;
    let account = Account {
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
        suscription,

        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        deleted: false,
    };

    match collection.insert_one(account.clone(), None).await {
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
            message: String::from("account registered successfully"),
            data: json!(account),
            exited_code: 0,
        }),
    )
}
