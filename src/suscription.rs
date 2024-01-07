use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Slug {
    FREE,
    PRO,
    PROPLUS,
}

impl FromStr for Slug {
    type Err = ();

    fn from_str(s: &str) -> Result<Slug, Self::Err> {
        match s {
            "free" => Ok(Slug::FREE),
            "pro" => Ok(Slug::PRO),
            "pro_plus" => Ok(Slug::PROPLUS),
            _ => Ok(Slug::FREE),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SuscriptionFrequencyClass {
    MONTHLY,
    YEARLY,
    UNDEFINED, // for free tbh
}

impl FromStr for SuscriptionFrequencyClass {
    type Err = ();

    fn from_str(s: &str) -> Result<SuscriptionFrequencyClass, Self::Err> {
        match s {
            "monthly" => Ok(SuscriptionFrequencyClass::MONTHLY),
            "yearly" => Ok(SuscriptionFrequencyClass::YEARLY),
            "undefined" => Ok(SuscriptionFrequencyClass::UNDEFINED),
            _ => Ok(SuscriptionFrequencyClass::UNDEFINED),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SuscriptionFeatures {
    CORE,
}

impl FromStr for SuscriptionFeatures {
    type Err = ();

    fn from_str(s: &str) -> Result<SuscriptionFeatures, Self::Err> {
        match s {
            "core" => Ok(SuscriptionFeatures::CORE),
            _ => Ok(SuscriptionFeatures::CORE),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuscriptionPlan {
    pub id: String,
    pub product_id: String,
    pub variants: Vec<String>,
    pub slug: Slug,
    pub name: String,
    pub price: u16,
    pub frequency: Vec<SuscriptionFrequencyClass>,
    pub most_popular: bool,
    pub is_active: bool,
    pub created_at: Duration,
    pub updated_at: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuscriptionHistoryLog {
    pub id: String,
    pub suscription_plan_id: String,
    pub frequency: SuscriptionFrequencyClass,
    pub starts_at: Duration,
    pub ends_at: Duration,
    pub renews_at: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suscription {
    pub id: String,
    pub suscription_plan_id: String,
    pub frequency: SuscriptionFrequencyClass,
    pub is_active: bool,

    pub created_at: Duration, // well, this is when the user created the account, the suscription is never deleted, only updated, if end so is free 
    pub updated_at: Duration,

    pub starts_at: Duration,
    pub ends_at: Duration,
    pub renews_at: Duration,

    pub history_logs: Vec<SuscriptionHistoryLog>,
}