use serde::Serialize;
use std::error::Error;

#[derive(Debug, Serialize)]
pub struct CreateContact {
    #[serde(rename = "updateEnabled")]
    update_enabled: bool,
    email: String,
    ext_id: String,
    #[serde(rename = "emailBlacklisted")]
    email_blacklisted: bool,
    #[serde(rename = "smsBlacklisted")]
    sms_blacklisted: bool,
    #[serde(rename = "listIds")]
    list_ids: Vec<u32>,
}


// add customer to campaign list in Brevo
pub async fn send_create_contact_request(api_key: &String, list_ids: Vec<u32>, ext_id: &String, email: &String) -> Result<(), Box<dyn Error>> {
    let api_url = "https://api.brevo.com/v3/contacts";
    let client = reqwest::Client::new();

    let create_contact = CreateContact {
        update_enabled: false,
        email: email.to_owned(),
        ext_id: ext_id.to_owned(),
        email_blacklisted: false,
        sms_blacklisted: false,
        list_ids,
    };

    let json_body = serde_json::to_value(create_contact)?;

    let response = client
        .post(api_url)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("api-key", api_key)
        .body(json_body.to_string())
        .send()
        .await?;

    if !response.status().is_success() {
        let error_message = response.text().await?;
        return Err(Box::from(error_message));
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Sender {
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
pub struct To {
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
pub struct Params {
    verification_link: String,
    greetings_title: String,
}

#[derive(Debug, Serialize)]
pub struct MessageVersion {
    to: To,
    params: Params,
    subject: String,
}

#[derive(Debug, Serialize)]
pub struct CreateEmailRequest {
    sender: Sender,
    subject: Option<String>,
    #[serde(rename = "templateId")]
    template_id: u32,
    params: Params,
    to: Vec<To>,
    #[serde(rename = "replyTo")]
    reply_to: To,
}

pub struct SendEmailData {
    pub api_key: String,
    pub template_id: u32,
    pub subject: String,
    pub sender_email: String,
    pub sender_name: String,
    pub customer_email: String,
    pub customer_name: String,
    pub verification_link: String,
    pub greetings_title: String,
}

// Verify Email
pub async fn send_verification_email(data: SendEmailData) -> Result<(), Box<dyn Error>> {
    let api_url = "https://api.brevo.com/v3/smtp/email";
    let client = reqwest::Client::new();

    let create_email_request = CreateEmailRequest {
        sender: Sender {
            email: data.sender_email.clone(),
            name: data.sender_name.clone(),
        },
        subject: Some(data.subject),
        template_id: data.template_id,
        params: Params {
            verification_link: data.verification_link,
            greetings_title: data.greetings_title,
        },
        to: vec![To{
                email: data.customer_email,
                name: data.customer_name,
        }],
        reply_to: To{
            email: data.sender_email,
            name: data.sender_name,
        },
    };

    let json_body = serde_json::to_value(create_email_request)?;

    let response = client
        .post(api_url)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("api-key", data.api_key)
        .body(json_body.to_string())
        .send()
        .await?;

    if !response.status().is_success() {
        let error_message = response.text().await?;
        return Err(Box::from(error_message));
    }
    
    Ok(())
}