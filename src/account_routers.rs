use crate::helpers::{payload_analyzer, random_string};
use crate::account::{Preferences, Account, Email, AccountType};
use crate::requests_interfaces::SignUp;
use crate::suscription::{Suscription, SuscriptionFrequencyClass};
use crate::{account::GenericResponse, server::AppState};

use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use mongodb::{bson::doc, Collection};
use regex::Regex;
use serde_json::json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::ops::Add;

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
                    "invalid username, must be at least 2 characters and at most 15",
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
                message: String::from("username and password must be different"),
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

    let ends_at = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .add(std::time::Duration::from_secs(3155695200));

    let suscription_id = random_string(10);
    let suscription = Suscription {
        id: suscription_id.await,
        suscription_plan_id: "free_default_v0.0.1alpha".to_string(),
        frequency: SuscriptionFrequencyClass::UNDEFINED,
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        starts_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
        // in 100 years
        ends_at,
        renews_at: ends_at,
        is_active: true,
        history_logs: vec![],
    };

    let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let user = Account {
        id: state.last_account_id + 1,
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

        created_at,
        updated_at: created_at,
        deleted: false,
    };

    match collection.insert_one(user.clone(), None).await {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error inserting user into database"),
                    data: json!({}),
                    exited_code: 1,
                }),
            )
        }
    }

    (
        StatusCode::CREATED,
        Json(GenericResponse {
            message: String::from("user registered successfully"),
            data: json!(user),
            exited_code: 0,
        }),
    )
}
