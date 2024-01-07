use std::{str::FromStr, time::Duration};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::suscription::Suscription;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericResponse {
    pub message: String,
    pub data: Value,
    pub exited_code: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub address: String,
    pub verified: bool,
    pub main: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccountType {
    PERSONAL,
    MANAGER,
    DEVELOPER,
}

impl FromStr for AccountType {
    type Err = ();

    fn from_str(s: &str) -> Result<AccountType, Self::Err> {
        match s {
            "personal" => Ok(AccountType::PERSONAL),
            "manager" => Ok(AccountType::MANAGER),
            "developer" => Ok(AccountType::DEVELOPER),
            _ => Ok(AccountType::PERSONAL),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    // identificators
    pub id: u64,
    pub name: String,
    pub class: AccountType,
    pub emails: Vec<Email>,

    // security
    pub password: String,                   // store hash of password (NEVER PLAIN TEXT)
    pub backup_security_codes: Vec<String>, // store hashes of backup securities

    // miscelaneous
    pub preferences: Preferences,
    pub suscription: Suscription,

    pub created_at: Duration,
    pub updated_at: Duration,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub dark_mode: bool,
    pub language: String,
    pub notifications: bool,
}
