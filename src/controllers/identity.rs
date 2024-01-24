use crate::oauth::google::{get_google_user, request_token};
use crate::utilities::api_messages::{APIMessages, CustomerMessages, EmailMessages, RedisMessages, TokenMessages};
use crate::utilities::helpers::payload_analyzer;
use crate::server::AppState;
use crate::storage::mongo::{build_customer_filter, find_customer};
use crate::utilities::token::{create_token, extract_token_from_headers, get_session_from_redis, validate_token};
use crate::types::customer::{AuthProviders, GenericResponse};
use crate::types::incoming_requests::SignIn;

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{
    extract::rejection::JsonRejection, 
    http::StatusCode, Json
};
use regex::Regex;
use serde::Deserialize;
use std::sync::Arc;

use bcrypt::verify;
use redis::{Client, Commands, RedisError};
use serde_json::json;

pub async fn get_user_session_from_req(
    headers: HeaderMap,
    redis_connection: &Client,
) -> Result<String, (StatusCode, Json<GenericResponse>)> {
    let token_string = extract_token_from_headers(&headers).await?;
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
    let id = match get_user_session_from_req(headers, &state.redis_connection).await {
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

pub async fn renew_session(
    headers: HeaderMap,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let token_string = match extract_token_from_headers(&headers).await {
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


pub async fn legacy_authentication(
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
    if customer.auth_provider != AuthProviders::LEGACY {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::OnlyLegacyProvider).to_string(),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

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

    if customer.auth_provider != AuthProviders::LEGACY {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::OnlyLegacyProvider).to_string(),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

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

#[derive(Debug, Deserialize)]
pub struct GoogleOAuthQueryParams {
    pub code: Option<String>,
}

pub async fn gooogle_authentication(
    Query(params): Query<GoogleOAuthQueryParams>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let authorization_code = match params.code {
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

    let token_response = match request_token(&authorization_code, &state).await {
        Ok(token_response) => token_response,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorRequestingGoogleToken).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };
    
    let google_user = match get_google_user(&token_response.access_token, &token_response.id_token).await {
        Ok(google_user) => google_user,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorFetchingUserFromGoogle).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    
    };

    let google_user_email = match google_user.email {
        Some(email) => email,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorFetchingUserFromGoogle).to_string(),
                    data: json!({}),
                    exited_code: 0,
                }),
            )
        }
    };

    let filter = build_customer_filter("", &google_user_email).await;
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
    if customer.auth_provider != AuthProviders::GOOGLE {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::OnlyGoogleProvider).to_string(),
                data: json!({}),
                exited_code: 0,
            }),
        );
    }

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