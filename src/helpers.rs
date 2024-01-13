use crate::types::account::GenericResponse;
use axum::{
    extract::rejection::JsonRejection,
    http::{StatusCode, Uri},
    Json,
};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde_json::json;

pub fn payload_analyzer<T>(
    payload_result: Result<Json<T>, JsonRejection>,
) -> Result<Json<T>, (StatusCode, Json<GenericResponse>)> {
    let payload = match payload_result {
        Ok(payload) => payload,
        Err(err) => {
            let message = format!("invalid payload: {}", err);
            let json = Json(GenericResponse {
                message,
                data: json!({}),
                exited_code: 1,
            });

            return Err((StatusCode::INTERNAL_SERVER_ERROR, json));
        }
    };

    Ok(payload)
}

pub async fn fallback(uri: Uri) -> (StatusCode, Json<GenericResponse>) {
    let message = format!("invalid endpoint: {}", uri.path());
    (
        StatusCode::NOT_FOUND,
        Json(GenericResponse {
            message,
            data: json!({}),
            exited_code: 1,
        }),
    )
}

pub async fn random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
