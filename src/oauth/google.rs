use reqwest::{Client, Url};
use serde::Deserialize;
use std::{error::Error, sync::Arc};

use crate::server::AppState;

#[derive(Deserialize)]
pub struct OAuthResponse {
    pub access_token: String,
    pub id_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserResult {
    pub id: Option<String>,
    pub email: Option<String>,
    pub verified_email: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

pub async fn request_token(
    authorization_code: &String,
    state: &Arc<AppState>,
) -> Result<OAuthResponse, Box<dyn Error>> {
    let redirect_url = state.google_auth.redirect_url.to_owned();
    let client_secret = state.google_auth.client_secret.to_owned();
    let client_id = state.google_auth.client_id.to_owned();

    let root_url = "https://oauth2.googleapis.com/token";
    let client = Client::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_url.as_str()),
        ("client_id", client_id.as_str()),
        ("code", authorization_code),
        ("client_secret", client_secret.as_str()),
    ];
    let response = client.post(root_url).form(&params).send().await?;

    if response.status().is_success() {
        let oauth_response = response.text().await?;
        let oauth_response: OAuthResponse = serde_json::from_str(&oauth_response)?;        
        Ok(oauth_response)
    } else {
        let err = response.text().await?;
        Err(From::from(err))
    }
}

pub async fn get_google_user(
    access_token: &str,
    id_token: &str,
) -> Result<GoogleUserResult, Box<dyn Error>> {
    let client = Client::new();
    let mut url = Url::parse("https://www.googleapis.com/oauth2/v1/userinfo")?;

    url.query_pairs_mut().append_pair("alt", "json");
    url.query_pairs_mut()
        .append_pair("access_token", access_token);


    let response =client.get(url).bearer_auth(id_token).send().await?;
    if response.status().is_success() {
        let user_info = response.text().await?;
        let user_info: GoogleUserResult = serde_json::from_str(&user_info)?;
        Ok(user_info)
    } else {
        let err = match response.text().await {
            Ok(err) => err,
            Err(err) => {
                return Err(From::from(err));
            }
        };
        
        Err(From::from(err))
    }
}