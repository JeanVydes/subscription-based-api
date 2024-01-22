use std::error::Error;
use crate::types::email::{CreateContact, CreateEmailRequest, Params, SendEmailData, Sender as EmailSender, To};

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

// Verify Email
pub async fn send_verification_email(data: SendEmailData) -> Result<(), Box<dyn Error>> {
    let api_url = "https://api.brevo.com/v3/smtp/email";
    let client = reqwest::Client::new();

    let create_email_request = CreateEmailRequest {
        sender: EmailSender {
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