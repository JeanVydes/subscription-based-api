use crate::{
    utilities::helpers::payload_analyzer,
    lemonsqueezy::subscription::{
        subscription_created, subscription_update_history_logs, subscription_update_status,
        subscription_updated,
    },
    server::AppState,
    types::customer::GenericResponse,
    types::lemonsqueezy::{OrderEvent, SubscriptionEvent},
};

use axum::{extract::rejection::JsonRejection, http::HeaderMap, http::StatusCode, Json};

use hex;
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use serde_json::json;
use std::sync::Arc;
use log::trace;

// built with the help of https://www.linkedin.com/pulse/verifying-custom-headers-hmac-signature-rust-axum-abdurachman--r8ltc
pub async fn signature_verification<T>(
    headers: HeaderMap,
    payload: Json<T>,
    state: Arc<AppState>,
) -> (bool, Json<GenericResponse>)
where
    T: Serialize,
{
    let signature_key = state.lemonsqueezy_webhook_signature_key.clone();
    let signature = match headers.get("X-Signature") {
        Some(signature) => signature,
        None => {
            return (
                false,
                Json(GenericResponse {
                    message: String::from("missing signature"),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    let signature = match signature.to_str() {
        Ok(signature) => signature,
        Err(_) => {
            return (
                false,
                Json(GenericResponse {
                    message: String::from("invalid signature"),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    if signature.len() != 64 {
        return (
            false,
            Json(GenericResponse {
                message: String::from("invalid signature length"),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    let mut mac = match Hmac::<Sha256>::new_from_slice(signature_key.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => {
            return (
                false,
                Json(GenericResponse {
                    message: String::from("invalid signature"),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    let payload_into_bytes = match serde_json::to_vec(&payload.0) {
        Ok(payload_into_bytes) => payload_into_bytes,
        Err(_) => {
            return (
                false,
                Json(GenericResponse {
                    message: String::from("error verifying signature payload"),
                    data: json!({}),
                    exit_code: 1,
                }),
            )
        }
    };

    mac.update(&payload_into_bytes);
    let result = mac.finalize().into_bytes();
    let result = hex::encode(result);

    if result != signature {
        return (
            false,
            Json(GenericResponse {
                message: String::from("invalid signature"),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    return (
        true,
        Json(GenericResponse {
            message: String::from(""),
            data: json!({}),
            exit_code: 0,
        }),
    );
}

pub async fn orders_webhook_events_listener(
    headers: HeaderMap,
    payload_result: Result<Json<OrderEvent>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    let (verified, error_response) =
        signature_verification(headers, payload.clone(), state.clone()).await;
    if !verified {
        return (StatusCode::BAD_REQUEST, error_response);
    }

    // order managing, i dont need this currently

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("captured"),
            data: json!({}),
            exit_code: 0,
        }),
    );
}

pub async fn subscription_webhook_events_listener(
    _headers: HeaderMap,
    payload_result: Result<Json<SubscriptionEvent>, JsonRejection>,
    state: Arc<AppState>,
) -> (StatusCode, Json<GenericResponse>) {
    let payload = match payload_analyzer(payload_result) {
        Ok(payload) => payload,
        Err((status_code, json)) => return (status_code, json),
    };

    //let (verified, error_response) = signature_verification(headers, payload.clone(), state.clone()).await;

    //if !verified {
      //  trace!("Signature Isn't Valid");
      //  return (StatusCode::BAD_REQUEST, error_response);
    //}

    let custom_data = match &payload.meta.custom_data {
        Some(custom_data) => custom_data,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GenericResponse {
                    message: String::from("not custom_data"),
                    data: json!({}),
                    exit_code: 1,
                }),
            );
        }
    };

    trace!("CUSTOM DATA: {:?}", custom_data);

    let customer_id = custom_data.customer_id.clone();
    if customer_id.len() > 100 || customer_id.len() < 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenericResponse {
                message: String::from("missing customer_id"),
                data: json!({}),
                exit_code: 1,
            }),
        );
    }

    trace!("EVENT NAME: {:?}", payload.meta.event_name);
    trace!("CUSTOMER ID: {:?}", customer_id);
    trace!("CUSTOMER EMAIL: {:?}", payload.data.attributes.user_email);

    let event_name = payload.meta.event_name.clone();
    match event_name.as_str() {
        "subscription_created" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_created(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_updated" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_updated(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_cancelled" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_status(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_resumed" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_status(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_expired" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_status(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_paused" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_status(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_unpaused" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_status(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_payment_success" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_history_logs(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_payment_failed" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_history_logs(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        "subscription_payment_recovered" => {
            let state = state.clone();
            let payload = payload.clone();
            match subscription_update_history_logs(payload.0, state).await {
                Ok(_) => (),
                Err(json) => return (StatusCode::BAD_REQUEST, json),
            }
        }
        _ => {}
    }

    return (
        StatusCode::OK,
        Json(GenericResponse {
            message: String::from("captured"),
            data: json!({}),
            exit_code: 0,
        }),
    );
}
