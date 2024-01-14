use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignIn {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub class: String,
    pub accepted_terms: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdateName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdatePassword {
    pub old_password: String,
    pub new_password: String,
    pub new_password_confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAddEmail {
    pub email: String,
}