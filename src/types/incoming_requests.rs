use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignIn {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCustomerRecord {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub class: String,
    pub accepted_terms: bool,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerUpdateName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerUpdatePassword {
    pub old_password: String,
    pub new_password: String,
    pub new_password_confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerAddEmail {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct FetchCustomerByID {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQueryParams {
    pub token: Option<String>,
}
