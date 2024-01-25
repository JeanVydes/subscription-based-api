use crate::types::subscription::Subscription;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericResponse {
    pub message: String,
    pub data: Value,
    pub exit_code: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub address: String,
    pub verified: bool,
    pub main: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CustomerType {
    PERSONAL,
    MANAGER,
    DEVELOPER,
}

impl ToString for CustomerType {
    fn to_string(&self) -> String {
        match self {
            CustomerType::PERSONAL => String::from("personal"),
            CustomerType::MANAGER => String::from("manager"),
            CustomerType::DEVELOPER => String::from("developer"),
        }
    }
}

impl FromStr for CustomerType {
    type Err = ();

    fn from_str(s: &str) -> Result<CustomerType, Self::Err> {
        match s {
            "personal" => Ok(CustomerType::PERSONAL),
            "manager" => Ok(CustomerType::MANAGER),
            "developer" => Ok(CustomerType::DEVELOPER),
            _ => Ok(CustomerType::PERSONAL),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthProviders {
    GOOGLE,
    LEGACY,
}

impl ToString for AuthProviders {
    fn to_string(&self) -> String {
        match self {
            AuthProviders::GOOGLE => String::from("GOOGLE"),
            AuthProviders::LEGACY => String::from("LEGACY"),
        }
    }
}


impl FromStr for AuthProviders {
    type Err = ();

    fn from_str(s: &str) -> Result<AuthProviders, Self::Err> {
        match s {
            "google" => Ok(AuthProviders::GOOGLE),
            "legacy" => Ok(AuthProviders::LEGACY),
            _ => Ok(AuthProviders::LEGACY),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub class: CustomerType,
    pub emails: Vec<Email>,
    pub auth_provider: AuthProviders,

    // security
    pub password: String, // store the hashed password
    pub backup_security_codes: Vec<String>, // stire hashed backup security codes

    // miscelaneous
    pub preferences: Preferences,
    pub subscription: Subscription,

    pub created_at: String,
    pub updated_at: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCustomer {
    pub id: String,
    pub name: String,
    pub class: CustomerType,
    
    pub preferences: Preferences,
    pub subscription: Subscription,

    pub created_at: String,
    pub updated_at: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateSensitiveCustomer {
    pub id: Option<String>,
    pub name: Option<String>,
    pub class: Option<CustomerType>,
    pub emails: Option<Vec<Email>>,
    pub auth_provider: Option<AuthProviders>,

    // miscelaneous
    pub preferences: Option<Preferences>,
    pub subscription: Option<Subscription>,

    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub deleted: Option<bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub dark_mode: bool,
    pub language: String,
    pub notifications: bool,
}
