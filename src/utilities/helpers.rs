use crate::types::{customer::{GenericResponse, CustomerType}, subscription::SubscriptionHistoryLog};
use axum::{
    extract::rejection::JsonRejection,
    http::{StatusCode, Uri},
    Json,
};
use mongodb::bson::{to_document, Document};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde_json::json;

use super::api_messages::{APIMessages, CustomerMessages, EmailMessages, InputMessages};

pub fn payload_analyzer<T>(
    payload_result: Result<Json<T>, JsonRejection>,
) -> Result<Json<T>, (StatusCode, Json<GenericResponse>)> {
    let payload = match payload_result {
        Ok(payload) => payload,
        Err(err) => {
            let message = format!("invalid.payload: {}", err);
            let json = Json(GenericResponse {
                message,
                data: json!({}),
                exit_code: 1,
            });

            return Err((StatusCode::INTERNAL_SERVER_ERROR, json));
        }
    };

    Ok(payload)
}

pub async fn fallback(uri: Uri) -> (StatusCode, Json<GenericResponse>) {
    let message = format!("invalid.endpoint.{}", uri.path());
    (
        StatusCode::NOT_FOUND,
        Json(GenericResponse {
            message,
            data: json!({}),
            exit_code: 1,
        }),
    )
}

pub async fn random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub async fn valid_email(email: &String) -> Result<bool, (StatusCode, Json<GenericResponse>)> {
    if  email.len() < 5 || email.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Email(EmailMessages::Invalid).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    }

    let re = Regex::new(r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})").unwrap();
    if !re.is_match(email.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Email(EmailMessages::Invalid).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    };
    
    Ok(true)
}

pub async fn valid_password(password: &String) -> Result<bool, (StatusCode, Json<GenericResponse>)> {
    if password.len() < 8 || password.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::InvalidNewPasswordLength).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    }

    let re = Regex::new(r"^[a-zA-Z0-9_]{8,20}$").unwrap();
    if !re.is_match(password.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::PasswordMustHaveAtLeastOneLetterAndOneNumber).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    };

    Ok(true)
}

pub async fn parse_class(raw_class: &String) -> Result<CustomerType, (StatusCode, Json<GenericResponse>)> {
    let class: CustomerType;
    if raw_class.to_lowercase() == "personal" {
        class = CustomerType::PERSONAL;
    } else if raw_class.to_lowercase() == "manager" {
        class = CustomerType::MANAGER;
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::InvalidType).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    }

    return Ok(class)
}

pub async fn add_subscription_history_log_and_to_bson(mut history_logs: Vec<SubscriptionHistoryLog>, log: SubscriptionHistoryLog) -> Vec<Document> {
    history_logs.push(log);
    let bson_history_logs: Vec<Document> = history_logs.iter()
    .map(|log| {
        match to_document(log) {
            Ok(document) => document,
            Err(_) => {
                return Document::new();
            }
        }
    })
    .collect();

    return bson_history_logs;
}