use crate::helpers::payload_analyzer;
use crate::server::AppState;
use crate::token::{create_token, validate_token};
use crate::types::account::{Account, GenericResponse};
use crate::types::incoming_requests::SignIn;

use axum::http::HeaderMap;
use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use regex::Regex;
use std::sync::Arc;

use bcrypt::verify;
use redis::{Client, Commands, RedisError};
use serde_json::json;

use mongodb::bson::doc;

// util to verify identity before to access to a private resource
pub async fn get_user_id_from_req(
    headers: HeaderMap,
    redis_connection: Client,
) -> Result<String, (StatusCode, Json<GenericResponse>)> {
    let token = match headers.get("Authorization") {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                }),
            ))
        }
    };

    let token_string = match token.to_str() {
        Ok(token) => token,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error parsing token"),
                    data: json!({}),
                    exited_code: 0,
                }),
            ))
        }
    };

    match validate_token(token_string) {
        Ok(_) => (),
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                }),
            ))
        }
    };

    let result = redis_connection
        .clone()
        .get::<String, String>(token_string.to_string());

    let id: String = match result {
        Ok(id) => id,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: format!("error getting session: {}", err),
                    data: json!({}),
                    exited_code: 0,
                }),
            ))
        }
    };

    Ok(id)
}

pub async fn get_session(
    headers: HeaderMap,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let token = match headers.get("Authorization") {
        Some(token) => token,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let token_string = match token.to_str() {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error parsing token"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    match validate_token(token_string) {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let result = state
        .redis_connection
        .clone()
        .get::<String, u64>(token_string.to_string());
    let id: u64 = match result {
        Ok(id) => id,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: format!("error getting session: {}", err),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    if id == 0 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("authorized"),
            data: json!({
                "account_id": id,
            }),
            exited_code: 0,
        }),
    );
}

pub async fn request_credentials(
    payload_result: Result<Json<SignIn>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    let email_re = Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$").unwrap();
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

    let filter = doc! {"$or": [
        {
            "emails": {
                "$elemMatch": {
                    "address": payload.email.clone(),
                }
            }
        }
    ]};

    let accounts_collection = state.mongo_db.collection("accounts");
    let account: Account = match accounts_collection
        .find_one(
            filter,
            None,
        )
        .await
    {
        Ok(account) => match account {
            Some(account) => account,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(GenericResponse {
                        message: String::from("email not associated with any account"),
                        data: json!({}),
                        exited_code: 0,
                    }),
                )
            }
        },
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("database error"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let is_valid = match verify(&payload.password, &account.password) {
        Ok(is_valid) => is_valid,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error verifying password"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    if !is_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    let token = match create_token(&account.id) {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error creating token"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let result: Result<bool, RedisError> =
        state
            .redis_connection
            .clone()
            .set_ex(token.clone(), &account.id, 86400);

    match result {
        Ok(_) => (),
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: format!("error caching session: {}", err),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("authorized"),
            data: json!({
                "token": token,
            }),
            exited_code: 0,
        }),
    );
}
