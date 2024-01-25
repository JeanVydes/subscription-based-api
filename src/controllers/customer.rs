use crate::email::brevo_api::send_create_contact_request;
use crate::storage::mongo::{build_customer_filter, find_customer, update_customer};
use crate::types::customer::{
    AuthProviders, Customer, Email, Preferences, PrivateSensitiveCustomer,
};
use crate::types::incoming_requests::{
    CreateCustomerRecord, CustomerUpdateName, CustomerUpdatePassword, FetchCustomerByID
};
use crate::types::subscription::{Slug, Subscription, SubscriptionFrequencyClass};
use crate::utilities::api_messages::{
    APIMessages, CustomerMessages, EmailMessages, InputMessages, MongoMessages,
    TokenMessages,
};
use crate::utilities::helpers::{
    parse_class, payload_analyzer, random_string, valid_email, valid_password,
};
use crate::{server::AppState, types::customer::GenericResponse};

use axum::extract::Query;
use axum::http::HeaderMap;
use axum::{extract::rejection::JsonRejection, http::StatusCode, Json};
use chrono::Utc;
use mongodb::bson::doc;
use serde_json::json;
use std::sync::Arc;

use bcrypt::{hash, verify, DEFAULT_COST};

use super::email::new_email_verification;
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
                    message: APIMessages::Customer(
                        CustomerMessages::PasswordConfirmationDoesNotMatch,
                    )
                    .to_string(),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }

        if payload.email.to_lowercase() == payload.password.to_lowercase() {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: APIMessages::Email(EmailMessages::EmailAndPasswordMustBeDifferent)
                        .to_string(),
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
                        message: APIMessages::Customer(CustomerMessages::ErrorHashingPassword)
                            .to_string(),
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
        Err((status, json)) => return (status, json),
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
        match send_create_contact_request(
            &api_key,
            vec![created_customer_list],
            &customer.id,
            &customer.emails[0].address,
        )
        .await
        {
            Ok(_) => (),
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

        if state.enabled_email_integration {
            match new_email_verification(
                &state,
                api_key,
                customer.emails[0].address.clone(),
                customer.name.clone(),
            ).await {
                Ok(_) => (),
                Err((status, json)) => return (status, json),
            }
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

    if !session_data
        .scopes
        .contains(&SessionScopes::ViewEmailAddresses)
    {
        shared_customer_data.emails = None;
    }

    if !session_data
        .scopes
        .contains(&SessionScopes::ViewSubscription)
    {
        shared_customer_data.subscription = None;
    }

    if !session_data
        .scopes
        .contains(&SessionScopes::ViewPublicProfile)
    {
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

    if !(session_data.scopes.contains(&SessionScopes::TotalAccess)
        && session_data.scopes.contains(&SessionScopes::UpdateName))
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
        Err((status, json)) => return (status, json),
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
                message: APIMessages::Token(TokenMessages::NotAllowedScopesToPerformAction)
                    .to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

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
                message: APIMessages::Input(
                    InputMessages::NewPasswordAndOldPasswordMustBeDifferent,
                )
                .to_string(),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    if payload.new_password != payload.new_password_confirmation {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: APIMessages::Input(InputMessages::NewPasswordConfirmationMustMatch)
                    .to_string(),
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
                    message: APIMessages::Customer(CustomerMessages::ErrorHashingPassword)
                        .to_string(),
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
                        message: APIMessages::Customer(CustomerMessages::IncorrectPassword)
                            .to_string(),
                        data: json!({}),
                        exit_code: 1,
                    }),
                );
            }
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenericResponse {
                    message: APIMessages::Customer(CustomerMessages::ErrorVerifyingPassword)
                        .to_string(),
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
        Err((status, json)) => return (status, json),
    }
}