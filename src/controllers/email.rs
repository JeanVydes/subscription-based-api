use std::sync::Arc;

use axum::{extract::{rejection::JsonRejection, Query}, http::{HeaderMap, StatusCode}, Json};
use chrono::Utc;
use mongodb::bson::doc;
use redis::{Commands, RedisError};
use serde_json::json;

use crate::{email::brevo_api::send_verification_email, server::AppState, storage::mongo::{build_customer_filter, find_customer, update_customer}, types::{customer::{Email, GenericResponse}, email::SendEmailData, incoming_requests::{CustomerAddEmail, VerifyEmailQueryParams}}, utilities::{api_messages::{APIMessages, CustomerMessages, EmailMessages, RedisMessages, TokenMessages}, helpers::{payload_analyzer, random_string, valid_email}}};

use super::identity::{get_user_session_from_req, SessionScopes};

pub async fn add_email(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerAddEmail>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    if !(session_data.scopes.contains(&SessionScopes::TotalAccess)
        && session_data
            .scopes
            .contains(&SessionScopes::UpdateEmailAddresses))
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::NotAllowedScopesToPerformAction)
                    .to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    let filter = build_customer_filter(session_data.customer_id.as_str(), "").await;
    let (found, customer) = match find_customer(&state.mongo_db, filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json),
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
    };

    let customer = customer.unwrap();

    let mut emails = customer.emails;
    if emails.len() >= 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Email(EmailMessages::MaxEmailsReached).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let email = payload.email.to_lowercase();
    match valid_email(&email).await {
        Ok(_) => (),
        Err((status_code, json)) => return (status_code, json),
    };

    for registered_email in emails.iter() {
        if registered_email.address == email {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Email(EmailMessages::Taken).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }
    }

    let filter = build_customer_filter("", email.as_str()).await;
    let (found, customer_with_current_email) = match find_customer(&state.mongo_db, filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json),
    };

    if found {
        if customer_with_current_email.unwrap().id != customer.id {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Email(EmailMessages::TakenByOtherCustomer).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }

        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Email(EmailMessages::TakenByYou).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    emails.push(Email {
        address: email.clone(),
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

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();

    let filter = build_customer_filter(session_data.customer_id.as_str(), "").await;
    let update = doc! {"$set": {
            "emails": &bson_emails,
            "updated_at": iso8601_string,
        }
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => {
            let api_key = match std::env::var("BREVO_CUSTOMERS_WEBFLOW_API_KEY") {
                Ok(api_key) => api_key,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(GenericResponse {
                            message: APIMessages::Customer(
                                CustomerMessages::ErrorRegisteringCustomerInMarketingPlatform,
                            )
                            .to_string(),
                            data: json!({}),
                            exit_code: 1,
                        }),
                    )
                }
            };
            
            match new_email_verification(
                &state,
                api_key,
                email,
                customer.name,
            ).await {
                Ok(_) => (),
                Err((status, json)) => return (status, json),
            }


            (StatusCode::OK, Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::EmailAdded).to_string(),
                data: json!({}),
                exit_code: 0,
            }))
        },
        Err((status, json)) => return (status, json),
    }
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

    let customer_email_address: String = match redis_conn.get(token.clone()) {
        Ok(customer_email_address) => customer_email_address,
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

    if customer_email_address.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Unauthorized.to_string(),
                data: json!({}),
                exit_code: 1,
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
        Err((status, json)) => return (status, json),
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
                    exit_code: 1,
                }),
            )
        }
    };

    (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Email(EmailMessages::Verified).to_string(),
            data: json!({}),
            exit_code: 0,
        }),
    )
}

pub async fn new_email_verification(
    state: &Arc<AppState>,
    api_key: String,
    customer_email: String,
    customer_name: String,
) -> Result<(), (StatusCode, Json<GenericResponse>)> {
    let new_token = random_string(30).await;
    let mut redis_conn = match state.redis_connection.get_connection() {
        Ok(redis_conn) => redis_conn,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::FailedToConnect).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            ))
        }
    };

    let result: Result<bool, RedisError> = redis_conn.set_ex(
        new_token.clone(),
        &customer_email,
        state.api_tokens_expiration_time.try_into().unwrap_or(86000),
    );

    match result {
        Ok(_) => (),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Redis(RedisMessages::ErrorSettingKey).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            ))
        }
    };

    let greetings_title = format!("Welcome to Test App {}", customer_name);
    let verification_link = format!("{}?token={}", state.google_auth.redirect_url, new_token);
    let send_email_data = SendEmailData {
        api_key,
        subject: "Verify Your New Email Address".to_string(),
        template_id: state.email_provider_settings.email_verification_template_id,
        customer_email: customer_email,
        customer_name: customer_name.clone(),
        verification_link,
        greetings_title,
        sender_email: state.master_email_entity.email.clone(),
        sender_name: state.master_email_entity.name.clone(),
    };

    match send_verification_email(send_email_data).await {
        Ok(_) => (),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Email(EmailMessages::ErrorSendingVerificationEmail)
                        .to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            ))
        }
    };

    Ok(())
}
