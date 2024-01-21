use serde::Serialize;
use std::error::Error;
use log::warn;

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
struct Sender {
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct To {
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct Params {
    verification_link: String,
    greetings_title: String,
}

#[derive(Debug, Serialize)]
struct MessageVersion {
    to: To,
    params: Params,
    subject: String,
}

#[derive(Debug, Serialize)]
struct CreateEmailRequest {
    sender: Sender,
    subject: Option<String>,
    #[serde(rename = "templateId")]
    template_id: u32,
    params: Params,
    #[serde(rename = "messageVersions")]
    message_versions: Vec<MessageVersion>,
}

// Verify Email
pub async fn send_verification_email(api_key: &String, template_id: u32, customer_email: &String, customer_name: &String, verification_link: String, greetings_title: String) -> Result<(), Box<dyn Error>> {
    let api_url = "https://api.brevo.com/v3/smtp/email";
    let client = reqwest::Client::new();

    let create_email_request = CreateEmailRequest {
        sender: Sender {
            email: "contact@nazi.email".to_string(),
            name: "Jean Services".to_string(),
        },
        subject: Some("Verify Your Email".to_string()),
        template_id: template_id,
        params: Params {
            verification_link: verification_link.clone(),
            greetings_title: greetings_title.clone(),
        },
        message_versions: vec![MessageVersion {
            to: To {
                email: customer_email.to_owned(),
                name: customer_name.to_owned(),
            },
            params: Params {
                verification_link,
                greetings_title,
            },
            subject: "Verify Your Email".to_string(),
        }],
    };

    warn!("create_email_request: {:?}", create_email_request);
    let json_body = serde_json::to_value(create_email_request)?;
    warn!("json_body: {:?}", json_body);

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
        warn!("Error sending verification email: {}", error_message);
        return Err(Box::from(error_message));
    }
    
    Ok(())
}