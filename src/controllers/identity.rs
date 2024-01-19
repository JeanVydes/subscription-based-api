use crate::utilities::helpers::payload_analyzer;
use crate::server::AppState;
use crate::storage::mongo::{build_customer_filter, find_customer};
use crate::utilities::token::{create_token, validate_token};
use crate::types::customer::GenericResponse;
use crate::types::incoming_requests::SignIn;

use axum::http::HeaderMap;
use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use regex::Regex;
use std::sync::Arc;

use bcrypt::verify;
use redis::{Client, Commands, RedisError};
use serde_json::json;

fn extract_token_string(headers: &HeaderMap) -> Result<&str, (StatusCode, Json<GenericResponse>)> {
    match headers.get("Authorization") {
        Some(token) => match token.to_str() {
            Ok(token) => Ok(token),
            Err(_) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error parsing token"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )),
        },
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exited_code: 0,
            }),
        )),
    }
}

// Common function to get session from Redis
async fn get_session_from_redis(
    redis_connection: Client,
    token_string: &str,
) -> Result<String, (StatusCode, Json<GenericResponse>)> {
    let result = redis_connection.clone().get::<String, String>(token_string.to_string());

    match result {
        Ok(id) => Ok(id),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenericResponse {
                message: format!("error getting session: {}", err),
                data: json!({}),
                exited_code: 0,
            }),
        )),
    }
}

pub async fn get_user_id_from_req(
    headers: HeaderMap,
    redis_connection: Client,
) -> Result<String, (StatusCode, Json<GenericResponse>)> {
    let token_string = extract_token_string(&headers)?;
    let _ = match validate_token(token_string) {
        Ok(_) => Ok(()),
        Err(msg) => Err((
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from(format!("unauthorized: {}", msg)),
                data: json!({}),
                exited_code: 0,
            }),
        )),
    };
    get_session_from_redis(redis_connection, &token_string).await
}

pub async fn get_session(
    headers: HeaderMap,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let id = match get_user_id_from_req(headers, state.redis_connection.clone()).await {
        Ok(id) => id,
        Err((status_code, json)) => return (status_code, json)
    };

    if id.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("authorized"),
            data: json!({
                "customer_id": id,
            }),
            exited_code: 0,
        }),
    )
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

    let filter = build_customer_filter("", payload.email.as_str()).await;
    let (found, customer) = match find_customer(state.mongo_db.clone(), filter).await {
        Ok((found, customer)) => (found, customer),
        Err((status_code, json)) => return (status_code, json),
    };

    if !found {
        return (
            StatusCode::NOT_FOUND,
            Json(GenericResponse {
                message: String::from("email not associated with any customer"),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    let customer = customer.unwrap();
    match verify(&payload.password, &customer.password) {
        Ok(is_valid) => {
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
        },
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

    let token = match create_token(&customer.id) {
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

    let mut redis_conn = match state.redis_connection.get_connection() {
        Ok(redis_conn) => redis_conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: String::from("error connecting to redis"),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let result: Result<bool, RedisError> =
        redis_conn
            .set_ex(token.clone(), &customer.id, 86400);

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
