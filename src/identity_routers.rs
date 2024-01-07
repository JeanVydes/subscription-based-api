use crate::helpers::payload_analyzer;
use crate::account::{GenericResponse, Account};
use crate::requests_interfaces::SignIn;
use crate::server::AppState;
use crate::token::{create_token, validate_token};

use axum::http::{HeaderMap, Request};
use axum::middleware::Next;
use axum::response::Response;
use axum::{extract::rejection::JsonRejection, extract::State, http::StatusCode, Json};
use std::sync::Arc;

use bcrypt::verify;
use serde_json::json;
use redis::{Commands, RedisError, Client};

use mongodb::bson::doc;

// util to verify identity before to access to a private resource
pub async fn identity_middleware<B>(
    redis_connection: State<Client>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, Json<GenericResponse>)> {
    let token = match request.headers().get("Authorization") {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                })),
            )
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
                })),
            )
        }
    };

    match validate_token(&token_string.to_string()) {
        Ok(_) => (),
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(GenericResponse {
                    message: String::from("unauthorized"),
                    data: json!({}),
                    exited_code: 0,
                })),
            )
        }
    };

    let result = redis_connection.clone().get::<String, u64>(token_string.to_string());
    let id: u64 = match result {
        Ok(id) => id,
        Err(err) => return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericResponse {
                message: format!("error getting session: {}", err),
                data: json!({}),
                exited_code: 0,
            })),
        ),
    };

    println!("id: {}", id);

    let response = next.run(request).await;
    Ok(response)
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

    match validate_token(&token_string.to_string()) {
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

    
    let result = state.redis_connection.clone().get::<String, u64>(token_string.to_string());
    let id: u64 = match result {
        Ok(id) => id,
        Err(err) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericResponse {
                message: format!("error getting session: {}", err),
                data: json!({}),
                exited_code: 0,
            }),
        ),
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

    let accounts_collection = state.mongo_db.collection("accounts");
    let account: Account = match accounts_collection
        .find_one(
            doc! {"$or":
                [
                    {"email": &payload.email.to_lowercase()}
                ]
            },
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
    
    let result: Result<bool, RedisError> = state.redis_connection.clone().set_ex(
        token.clone(),
        &account.id,
        86400,
    );

    match result {
        Ok(_) => (),
        Err(err) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericResponse {
                message: format!("error caching session: {}", err),
                data: json!({}),
                exited_code: 0,
            }),
        ),
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
