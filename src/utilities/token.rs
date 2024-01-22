use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use super::api_messages::{APIMessages, TokenMessages};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub aud: String,
    pub sub: String,
    pub exp: usize,
}

pub fn create_token(id: &String) -> Result<std::string::String, String> {
    let expiration_time = env::var("API_TOKENS_EXPIRATION_TIME").unwrap_or(String::from("86400"));
    let header = Header::new(Algorithm::HS512);
    let claims = Claims {
        aud: String::from("myself"),
        sub: id.to_string(),
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + expiration_time.parse::<usize>().unwrap(),
    };

    let signing_key = match env::var("API_TOKENS_SIGNING_KEY") {
        Ok(key) => key,
        Err(_) => return Err(APIMessages::Token(TokenMessages::NotSigningKeyFound).to_string()),
    };

    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(signing_key.as_ref()),
    ) {
        Ok(t) => Ok(t),
        Err(_) => Err(APIMessages::Token(TokenMessages::ErrorCreating).to_string()),
    }
}

pub fn validate_token(token: &str) -> Result<TokenData<Claims>, String> {
    let validation = Validation::new(Algorithm::HS512);

    let signing_key = match env::var("API_TOKENS_SIGNING_KEY") {
        Ok(key) => key,
        Err(_) => return Err(APIMessages::Token(TokenMessages::ErrorValidating).to_string()),
    };

    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(signing_key.as_ref()),
        &validation,
    ) {
        Ok(t) => t,
        Err(_) => return Err(APIMessages::Token(TokenMessages::ErrorValidating).to_string()),
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    if now.as_secs() > token_data.claims.exp as u64 {
        return Err(APIMessages::Token(TokenMessages::Expired).to_string());
    }

    Ok(token_data)
}
