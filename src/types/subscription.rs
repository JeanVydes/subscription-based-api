use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Slug {
    FREE,
    PRO,
}

impl Slug {
    pub fn to_string(&self) -> String {
        match self {
            Slug::FREE => String::from("free"),
            Slug::PRO => String::from("pro"),
        }
    }
}

impl FromStr for Slug {
    type Err = ();

    fn from_str(s: &str) -> Result<Slug, Self::Err> {
        match s {
            "free" => Ok(Slug::FREE),
            "pro" => Ok(Slug::PRO),
            _ => Ok(Slug::FREE),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SubscriptionFrequencyClass {
    MONTHLY,
    ANNUALLY,
    UNDEFINED, // for free tbh
}

impl FromStr for SubscriptionFrequencyClass {
    type Err = ();

    fn from_str(s: &str) -> Result<SubscriptionFrequencyClass, Self::Err> {
        match s {
            "monthly" => Ok(SubscriptionFrequencyClass::MONTHLY),
            "yearly" => Ok(SubscriptionFrequencyClass::ANNUALLY),
            "undefined" => Ok(SubscriptionFrequencyClass::UNDEFINED),
            _ => Ok(SubscriptionFrequencyClass::UNDEFINED),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SubscriptionFeatures {
    CORE,
}

impl FromStr for SubscriptionFeatures {
    type Err = ();

    fn from_str(s: &str) -> Result<SubscriptionFeatures, Self::Err> {
        match s {
            "core" => Ok(SubscriptionFeatures::CORE),
            _ => Ok(SubscriptionFeatures::CORE),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHistoryLog {
    pub event: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub product_id: i64,
    pub variant_id: i64,
    pub slug: String,
    pub frequency: SubscriptionFrequencyClass,
    pub status: String,

    pub created_at: String, // well, this is when the account created the account, the subscription is never deleted, only updated, if end so is free
    pub updated_at: String,

    pub starts_at: String,
    pub ends_at: String,
    pub renews_at: String,

    pub history_logs: Vec<SubscriptionHistoryLog>,
}