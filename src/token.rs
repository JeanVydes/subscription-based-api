use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub aud: String,
    pub sub: String,
    pub exp: usize,
}

pub fn create_token(id: &u64) -> Result<std::string::String, String> {
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
        Err(_) => return Err(String::from("not signing key found")),
    };

    match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(signing_key.as_ref()),
    ) {
        Ok(t) => Ok(t),
        Err(_) => Err(String::from("error creating token")),
    }
}

pub fn validate_token(token: &String) -> Result<TokenData<Claims>, String> {
    let validation = Validation::new(Algorithm::HS512);

    let signing_key = match env::var("API_TOKENS_SIGNING_KEY") {
        Ok(key) => key,
        Err(_) => return Err(String::from("not signing key found")),
    };

    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(signing_key.as_ref()),
        &validation,
    ) {
        Ok(t) => t,
        Err(_) => return Err(String::from("error validating token")),
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    if now.as_secs() > token_data.claims.exp as u64 {
        return Err(String::from("expired token"));
    }

    Ok(token_data)
}
