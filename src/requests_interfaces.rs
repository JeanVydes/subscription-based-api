use serde::{Deserialize, Serialize};

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
