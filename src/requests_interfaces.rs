use serde::{Deserialize, Serialize};

use crate::lemonsqueezy::SubscriptionEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignIn {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignUp {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub class: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventIncoming {
    pub data: SubscriptionEvent,
}