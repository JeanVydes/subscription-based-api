use crate::email::actions::{send_create_contact_request, send_verification_email};
use crate::types::email::SendEmailData;
use crate::utilities::api_messages::{APIMessages, CustomerMessages, EmailMessages, InputMessages, MongoMessages, RedisMessages, TokenMessages};
use crate::utilities::helpers::{payload_analyzer, random_string, valid_password, valid_email, parse_class};
use crate::storage::mongo::{build_customer_filter, find_customer, update_customer};
use crate::types::customer::{AuthProviders, Customer, Email, Preferences, PrivateSensitiveCustomer};
use crate::types::incoming_requests::{CreateCustomerRecord, CustomerUpdateName, CustomerUpdatePassword, CustomerAddEmail};
use crate::types::subscription::{Slug, Subscription, SubscriptionFrequencyClass};
use crate::{server::AppState, types::customer::GenericResponse};

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use chrono::Utc;
use mongodb::bson::doc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use redis::{Commands, RedisError};

use bcrypt::{hash, DEFAULT_COST, verify};

use super::identity::{get_user_session_from_req, SessionScopes};

pub async fn create_customer_record(
    payload_result: Result<Json<CreateCustomerRecord>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if !payload.accepted_terms {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::NotAcceptedTerms).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let auth_provider: AuthProviders;
    match payload.provider.to_lowercase().as_str() {
        "legacy" => auth_provider = AuthProviders::LEGACY,
        "google" => auth_provider = AuthProviders::GOOGLE,
        _ => auth_provider = AuthProviders::LEGACY,
    }

    if payload.name.len() < 2 || payload.name.len() > 25 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::InvalidNameLength).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    match valid_email(&payload.email).await {
        Ok(_) => (),
        Err((status_code, json)) => return (status_code, json),
    };

    let mut hashed_password = "".to_string();
    if auth_provider == AuthProviders::LEGACY {
        match valid_password(&payload.password).await {
            Ok(_) => (),
            Err((status_code, json)) => return (status_code, json),
        };

        if payload.password != payload.password_confirmation {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Customer(CustomerMessages::PasswordConfirmationDoesNotMatch).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }
    

        if payload.email.to_lowercase() == payload.password.to_lowercase() {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Email(EmailMessages::EmailAndPasswordMustBeDifferent).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }

        hashed_password = match hash(&payload.password, DEFAULT_COST) {
            Ok(hashed_password) => hashed_password,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericResponse {
                        message: APIMessages::Customer(CustomerMessages::ErrorHashingPassword).to_string(),
                        data: json!({}),
                        exit_code: 1,
                    }),
                )
            }
        };    
    }

    let filter = build_customer_filter("", payload.email.to_lowercase().as_str()).await;
    let (found, _) = match find_customer(&state.mongo_db, filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
    };

    if found {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Email(EmailMessages::Taken).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let emails = vec![Email {
        address: payload.email.to_lowercase(),
        verified: false,
        main: true,
    }];

    let class = match parse_class(&payload.class).await {
        Ok(class) => class,
        Err((status_code, json)) => return (status_code, json),
    };

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();
    let subscription_id = random_string(10).await;
    let subscription = Subscription {
        id: subscription_id,
        product_id: 0,
        variant_id: 0,
        slug: Slug::FREE.to_string(),
        frequency: SubscriptionFrequencyClass::UNDEFINED,
        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        starts_at: "".to_string(),
        ends_at: "".to_string(),
        renews_at: "".to_string(),
        status: "".to_string(),
        history_logs: vec![],
    };

    let id = random_string(30).await;
    let customer = Customer {
        id,
        name: payload.name.clone(),
        class,
        emails,
        auth_provider,

        password: hashed_password,
        backup_security_codes: vec![],

        preferences: Preferences {
            dark_mode: false,
            language: String::from("en"),
            notifications: true,
        },
        subscription,

        created_at: iso8601_string.clone(),
        updated_at: iso8601_string.clone(),
        deleted: false,
    };

    let created_customer_list = std::env::var("BREVO_CUSTOMERS_LIST_ID");
    let api_key = std::env::var("BREVO_CUSTOMERS_WEBFLOW_API_KEY");
    
    if created_customer_list.is_ok() && api_key.is_ok() {
        let created_customer_list = match created_customer_list.unwrap().parse::<u32>() {
            Ok(list_id) => list_id,
            Err(_) => 1,
        };

        let api_key = api_key.unwrap();
        match send_create_contact_request(&api_key, vec![created_customer_list], &customer.id, &customer.emails[0].address).await {
            Ok(_) => (),
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GenericResponse {
                        message: APIMessages::Customer(CustomerMessages::ErrorRegisteringCustomerInMarketingPlatform).to_string(),
                        data: json!({}),
                        exit_code: 1,
                    }),
                )
            }
        };

        if state.enabled_email_integration {
            let new_token = random_string(30).await;
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
                    .set_ex(new_token.clone(), &customer.emails[0].address, state.api_tokens_expiration_time.try_into().unwrap_or(86000));

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

            let greetings_title = format!("Welcome to Test App {}", customer.name);
            let verification_link = format!("{}?token={}", state.google_auth.redirect_url, new_token);
            let send_email_data = SendEmailData {
                api_key,
                subject: "Verify Your Email Address To Start Using Test App".to_string(),
                template_id: state.email_provider_settings.email_verification_template_id,
                customer_email: customer.emails[0].address.clone(),
                customer_name: customer.name.clone(),
                verification_link,
                greetings_title,
                sender_email: state.master_email_entity.email.clone(),
                sender_name: state.master_email_entity.name.clone(),
            };

            match send_verification_email(send_email_data).await {
                Ok(_) => (),
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(GenericResponse {
                            message: APIMessages::Email(EmailMessages::ErrorSendingVerificationEmail).to_string(),
                            data: json!({}),
                            exit_code: 1,
                        }),
                    )
                }
            };
        }
    }

    let collection = state.mongo_db.collection("customers");
    match collection.insert_one(customer.clone(), None).await {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Mongo(MongoMessages::ErrorInserting).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    }
    
    (
        StatusCode::CREATED,
        Json(GenericResponse {
            message: APIMessages::Customer(CustomerMessages::Created).to_string(),
            data: json!(customer),
            exit_code: 0,
        }),
    )
}

#[derive(Debug, Deserialize)]
pub struct FetchCustomerByID {
    pub id: Option<String>,
}

pub async fn fetch_customer_record_by_id(
    headers: HeaderMap,
    Query(params): Query<FetchCustomerByID>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    let customer_id = match params.id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Customer(CustomerMessages::NotFoundByID).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    let filter = build_customer_filter(customer_id.as_str(), "").await;
    let (found, customer) = match find_customer(&state.mongo_db, filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
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

    let mut shared_customer_data = PrivateSensitiveCustomer {
        id: Some(customer_id),
        name: Some(customer.name),
        class: Some(customer.class),
        emails: Some(customer.emails),
        auth_provider: Some(customer.auth_provider),
        preferences: Some(customer.preferences),
        subscription: Some(customer.subscription),
        created_at: Some(customer.created_at),
        updated_at: Some(customer.updated_at),
        deleted: Some(customer.deleted),
    };

    if session_data.scopes.contains(&SessionScopes::TotalAccess) {
        return (
            StatusCode::OK,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::Found).to_string(),
                data: json!(shared_customer_data),
                exit_code: 1,
            }),
        );
    }

    if !session_data.scopes.contains(&SessionScopes::ViewPublicID) {
        shared_customer_data.id = None;
    }

    if !session_data.scopes.contains(&SessionScopes::ViewEmailAddresses) {
        shared_customer_data.emails = None;
    }

    if !session_data.scopes.contains(&SessionScopes::ViewSubscription) {
        shared_customer_data.subscription = None;
    }

    if !session_data.scopes.contains(&SessionScopes::ViewPublicProfile) {
        shared_customer_data.name = None;
        shared_customer_data.class = None;
        shared_customer_data.preferences = None;
        shared_customer_data.created_at = None;
        shared_customer_data.updated_at = None;
        shared_customer_data.deleted = None;
    }

    (
        StatusCode::OK,
        Json(GenericResponse {
            message: APIMessages::Customer(CustomerMessages::Found).to_string(),
            data: json!(shared_customer_data),
            exit_code: 0,
        }),
    )
}

pub async fn update_name(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerUpdateName>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    if !(session_data.scopes.contains(&SessionScopes::TotalAccess) && session_data.scopes.contains(&SessionScopes::UpdateName)) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::NotAllowedScopesToPerformAction).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.name.len() < 2 || payload.name.len() > 25 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::InvalidNameLength).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();

    let filter = build_customer_filter(session_data.customer_id.as_str(), "").await;
    let update = doc! {"$set": {
            "name": &payload.name,
            "updated_at": iso8601_string,
        }
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::NameUpdated).to_string(),
                data: json!({}),
                exit_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
}

pub async fn update_password(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerUpdatePassword>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    if !session_data.scopes.contains(&SessionScopes::TotalAccess) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::NotAllowedScopesToPerformAction).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let filter = build_customer_filter(session_data.customer_id.as_str(), "").await;
    let (found, customer) = match find_customer(&state.mongo_db, filter).await {
        Ok(customer) => customer,
        Err((status, json)) => return (status, json)
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

    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.old_password.len() < 8 || payload.old_password.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::InvalidOldPasswordLength).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    if payload.new_password.len() < 8 || payload.new_password.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::InvalidNewPasswordLength).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    match valid_password(&payload.new_password).await {
        Ok(_) => (),
        Err((status_code, json)) => return (status_code, json),
    };

    if payload.new_password == payload.old_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::NewPasswordAndOldPasswordMustBeDifferent).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    if payload.new_password != payload.new_password_confirmation {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::NewPasswordConfirmationMustMatch).to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let hashed_new_password = match hash(&payload.new_password, DEFAULT_COST) {
        Ok(hashed_password) => hashed_password,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Customer(CustomerMessages::ErrorHashingPassword).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    let customer = customer.unwrap();

    match verify(&payload.old_password, &customer.password) {
        Ok(is_valid) => {
            if !is_valid {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(GenericResponse {
                        message: APIMessages::Customer(CustomerMessages::IncorrectPassword).to_string(),
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
                    message: APIMessages::Customer(CustomerMessages::ErrorVerifyingPassword).to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    let current_datetime = Utc::now();
    let iso8601_string = current_datetime.to_rfc3339();

    let filter = build_customer_filter(session_data.customer_id.as_str(), "").await;
    let update = doc! {"$set": {
            "password": hashed_new_password,
            "updated_at": iso8601_string,
        }
    };

    match update_customer(&state.mongo_db, filter, update).await {
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::PasswordUpdated).to_string(),
                data: json!({}),
                exit_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
}

pub async fn add_email(
    headers: HeaderMap,
    payload_result: Result<Json<CustomerAddEmail>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let session_data = match get_user_session_from_req(headers, &state.redis_connection).await {
        Ok(customer_id) => customer_id,
        Err((status_code, json)) => return (status_code, json),
    };

    if !(session_data.scopes.contains(&SessionScopes::TotalAccess) && session_data.scopes.contains(&SessionScopes::UpdateEmailAddresses)) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(GenericResponse {
                message: APIMessages::Token(TokenMessages::NotAllowedScopesToPerformAction).to_string(),
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
        Err((status, json)) => return (status, json)
    };

    if !found {
        return (
            StatusCode::NOT_FOUND,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::NotFound).to_string(),
                data: json!({}),
                exit_code: 1
                ,
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
        Err((status, json)) => return (status, json)
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
        address: email,
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
        Ok(_) => (
            StatusCode::OK,
            Json(GenericResponse {
                message: APIMessages::Customer(CustomerMessages::EmailAdded).to_string(),
                data: json!({}),
                exit_code: 0,
            }),
        ),
        Err((status, json)) => return (status, json)
    }
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