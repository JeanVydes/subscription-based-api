use crate::oauth::google::{get_google_user, request_token};
use crate::utilities::api_messages::{APIMessages, CustomerMessages, EmailMessages, RedisMessages, TokenMessages};
use crate::utilities::helpers::payload_analyzer;
use crate::server::AppState;
use crate::storage::mongo::{build_customer_filter, find_customer};
use crate::utilities::token::{create_token, extract_token_from_headers, get_session_from_redis, get_token_payload, string_to_scopes, validate_token};
use crate::types::customer::{AuthProviders, GenericResponse};
use crate::types::incoming_requests::SignIn;

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{
    extract::rejection::JsonRejection, 
    http::StatusCode, Json
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

use bcrypt::verify;
use redis::{Client, Commands, RedisError};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum SessionScopes {
    ViewPublicID,
    ViewEmailAddresses,
    ViewPublicProfile,
    ViewPrivateSensitiveProfile,
    ViewSubscription,
    
    UpdateName,
    UpdateEmailAddresses,
    UpdatePreferences,

    TotalAccess, // never use this for 3rd party apps
}

impl ToString for SessionScopes {
    fn to_string(&self) -> String {
        match self {
            SessionScopes::ViewPublicID => String::from("view_public_id"),
            SessionScopes::ViewEmailAddresses => String::from("view_email_addresses"),
            SessionScopes::ViewPublicProfile => String::from("view_public_profile"),
            SessionScopes::ViewPrivateSensitiveProfile => String::from("view_private_sensitive_profile"),
            SessionScopes::ViewSubscription => String::from("view_subscription"),
            
            SessionScopes::UpdateName => String::from("update_name"),
            SessionScopes::UpdateEmailAddresses => String::from("update_email_addresses"),
            SessionScopes::UpdatePreferences => String::from("update_preferences"),

            SessionScopes::TotalAccess => String::from("total_access"),
        }
    }
}

impl FromStr for SessionScopes {
    type Err = ();

    fn from_str(input: &str) -> Result<SessionScopes, Self::Err> {
        match input {
            "view_public_id" => Ok(SessionScopes::ViewPublicID),
            "view_email_addresses" => Ok(SessionScopes::ViewEmailAddresses),
            "view_public_profile" => Ok(SessionScopes::ViewPublicProfile),
            "view_private_sensitive_profile" => Ok(SessionScopes::ViewPrivateSensitiveProfile),
            "view_subscription" => Ok(SessionScopes::ViewSubscription),
            
            "update_name" => Ok(SessionScopes::UpdateName),
            "update_email_addresses" => Ok(SessionScopes::UpdateEmailAddresses),
            "update_preferences" => Ok(SessionScopes::UpdatePreferences),

            "total_access" => Ok(SessionScopes::TotalAccess),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub customer_id: String,
    pub scopes: Vec<SessionScopes>,
}

pub async fn get_user_session_from_req(
    headers: HeaderMap,
    redis_connection: &Client,
) -> Result<SessionData, (StatusCode, Json<GenericResponse>)> {
    let token_string = extract_token_from_headers(&headers).await?;
    let _ = match validate_token(token_string) {
        Ok(_) => Ok(()),
        Err(msg) => Err((
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from(format!("unauthorized: {}", msg)),
                data: json!({}),
                exit_code: 1,
            }),
        )),
    };

    let customer_id = match get_session_from_redis(redis_connection, &token_string).await {
        Ok(token) => token,
        Err((status_code, json)) => return Err((status_code, json)),
    };
    
    let token_data = match get_token_payload(&token_string) {
        Ok(token_data) => token_data,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorParsingToken).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            ))
        }
    };

    if customer_id != token_data.claims.sub {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exit_code: 1,
            }),
        ));
    }

    let raw_scopes = token_data.claims.aud;
    let scopes: Vec<SessionScopes> = string_to_scopes(raw_scopes);
    
    let session_data = SessionData {
        customer_id,
        scopes,
    };

    return Ok(session_data);
}

pub async fn get_session(
    headers: HeaderMap,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(id) => id,
        Err((status_code, json)) => return (status_code, json)
    };

    if session_data.customer_id.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: String::from("unauthorized"),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("authorized"),
            data: json!({
                "customer_id": session_data.customer_id,
            }),
            exit_code: 0,
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
                    exit_code: 1,
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
                    exit_code: 1,
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
                exit_code: 1,
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
                    exit_code: 1,
                }),
            )
        }
    };

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Token(TokenMessages::Renewed).to_string(),
            data: json!({}),
            exit_code: 0,
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
                exit_code: 1,
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
                exit_code: 1,
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
                exit_code: 1,
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
                        exit_code: 1,
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
                    exit_code: 1,
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
                exit_code: 1,
            }),
        );
    }

    let token = match create_token(&customer.id, vec![SessionScopes::TotalAccess]) {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorCreating).to_string(),
                    data: json!({}),
                    exit_code: 1,
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
                    exit_code: 1,
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
                    exit_code: 1,
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
            exit_code: 0,
        }),
    );
}

#[derive(Debug, Deserialize)]
pub struct GoogleOAuthQueryParams {
    pub code: Option<String>,
    pub error: Option<String>,
}

pub async fn gooogle_authentication(
    Query(params): Query<GoogleOAuthQueryParams>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    match params.error {
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorRequestingGoogleToken).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        },
        None => (),
    };

    let authorization_code = match params.code {
        Some(token) => token,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::Missing).to_string(),
                    data: json!({}),
                    exit_code: 1,
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
                    exit_code: 1,
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
                    exit_code: 1,
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
                    exit_code: 1,
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
                data: json!({
                    "action": "create_customer_record",
                    "auth_provider": AuthProviders::GOOGLE,
                    "openid": google_user.id,
                    "email": google_user_email,
                    "verified_email": google_user.verified_email,
                    "name": google_user.name,
                    "given_name": google_user.given_name,
                    "family_name": google_user.family_name,
                    "picture": google_user.picture,
                    "locale": google_user.locale,
                }),
                exit_code: 1,
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
                exit_code: 1,
            }),
        );
    }

    let token = match create_token(&customer.id, vec![SessionScopes::TotalAccess]) {
        Ok(token) => token,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Token(TokenMessages::ErrorCreating).to_string(),
                    data: json!({}),
                    exit_code: 1,
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
                    exit_code: 1,
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
                    exit_code: 1,
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
            exit_code: 0,
        }),
    );
}