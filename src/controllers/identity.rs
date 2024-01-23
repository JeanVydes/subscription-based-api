use crate::utilities::api_messages::{APIMessages, CustomerMessages, EmailMessages, RedisMessages, TokenMessages};
use crate::utilities::helpers::payload_analyzer;
use crate::server::AppState;
use crate::storage::mongo::{build_customer_filter, find_customer, update_customer};
use crate::utilities::token::{create_token, validate_token};
use crate::types::customer::GenericResponse;
use crate::types::incoming_requests::SignIn;

use axum::http::HeaderMap;
use axum::{
    extract::{Query, rejection::JsonRejection}, 
    http::StatusCode, Json
};
use mongodb::bson::doc;
use regex::Regex;
use std::sync::Arc;

use bcrypt::verify;
use redis::{Client, Commands, RedisError};
use serde::Deserialize;
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
    redis_connection: &Client,
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
    redis_connection: &Client,
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
    let id = match get_user_id_from_req(headers, &state.redis_connection).await {
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
                message: APIMessages::Email(EmailMessages::Invalid).to_string(),
                data: json!({}),
                exited_code: 1,
            }),
        );
    }

    let filter = build_customer_filter("", payload.email.as_str()).await;
    let (found, customer) = match find_customer(&state.mongo_db, filter).await {
        Ok((found, customer)) => (found, customer),
        Err((status_code, json)) => return (status_code, json),
    };

    if !found {
        return (
            StatusCode::NOT_FOUND,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::NotFound).to_string(),
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
                        message: APIMessages::Unauthorized.to_string(),
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
                    message: APIMessages::InternalServerError.to_string(),
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
                    message: APIMessages::Token(TokenMessages::ErrorCreating).to_string(),
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
                    message: APIMessages::Redis(RedisMessages::FailedToConnect).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let result: Result<bool, RedisError> =
        redis_conn
            .set_ex(token.clone(), &customer.id, 604800);

    match result {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::ErrorSettingKey).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Token(TokenMessages::Created).to_string(),
            data: json!({
                "token": token,
            }),
            exited_code: 0,
        }),
    );
}

pub async fn renew_session(
    headers: HeaderMap,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let token_string = match extract_token_string(&headers) {
        Ok(token_string) => token_string,
        Err((status_code, json)) => return (status_code, json),
    };

    let mut redis_conn = match state.redis_connection.get_connection() {
        Ok(redis_conn) => redis_conn,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::FailedToConnect).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let customer_id: String = match redis_conn.get(token_string.to_string()) {
        Ok(customer_id) => customer_id,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::ErrorFetching).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    if customer_id.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::Expired).to_string(),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    let result: Result<bool, RedisError> =
        redis_conn
            .set_ex(token_string.to_string(), customer_id, 604800);

    match result {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorRenewing).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Token(TokenMessages::Renewed).to_string(),
            data: json!({}),
            exited_code: 0,
        }),
    );
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQueryParams {
    pub token: Option<String>,
}

pub async fn verify_email(
    Query(params): Query<VerifyEmailQueryParams>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let token = match params.token {
        Some(token) => token,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::Missing).to_string(),
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
                    message: APIMessages::Redis(RedisMessages::FailedToConnect).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let customer_email_address: String = match redis_conn.get(token.clone()) {
        Ok(customer_email_address) => customer_email_address,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::ErrorFetching).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    if customer_email_address.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Unauthorized.to_string(),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

    let filter = doc! {
        "emails.address": customer_email_address,
    };
    
    let update = doc! {
        "$set": {
            "emails.$.verified": true,
        }
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => (),
        Err((status, json)) => return (status, json)
    };

    let result: Result<bool, RedisError> = redis_conn.del(token.clone());
    match result {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::ErrorDeleting).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Email(EmailMessages::Verified).to_string(),
            data: json!({}),
            exited_code: 0,
        }),
    )
}