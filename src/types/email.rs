use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CreateContact {
    #[serde(rename = "updateEnabled")]
    pub update_enabled: bool,
    pub email: String,
    pub ext_id: String,
    #[serde(rename = "emailBlacklisted")]
    pub email_blacklisted: bool,
    #[serde(rename = "smsBlacklisted")]
    pub sms_blacklisted: bool,
    #[serde(rename = "listIds")]
    pub list_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct Sender {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct To {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Params {
    pub verification_link: String,
    pub greetings_title: String,
}

#[derive(Debug, Serialize)]
pub struct MessageVersion {
    pub to: To,
    pub params: Params,
    pub subject: String,
}

#[derive(Debug, Serialize)]
pub struct CreateEmailRequest {
    pub sender: Sender,
    pub subject: Option<String>,
    #[serde(rename = "templateId")]
    pub template_id: u32,
    pub params: Params,
    pub to: Vec<To>,
    #[serde(rename = "replyTo")]
    pub reply_to: To,
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