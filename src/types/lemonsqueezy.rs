use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Products {
    pub pro_product_id: i64,
    pub pro_monthly_variant_id: i64,
    pub pro_annually_variant_id: i64,
}

// events

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub meta: Meta,
    pub data: OrderData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    pub meta: Meta,
    pub data: SubscriptionData,
}

// meta

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub event_name: String,
    pub webhook_id: Option<String>,
    pub custom_data: Option<CustomData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomData {
    pub customer_id: String, // same as acount id, this is using custom data, read more here https://docs.lemonsqueezy.com/help/checkout/passing-custom-data
}

// misc

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationships {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<RelationshipLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<RelationshipLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<RelationshipLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<RelationshipLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<RelationshipLinks>,
    #[serde(rename = "order-item", skip_serializing_if = "Option::is_none")]
    pub order_item: Option<RelationshipLinks>,
    #[serde(rename = "subscription-items", skip_serializing_if = "Option::is_none")]
    pub subscription_item: Option<RelationshipLinks>,
    #[serde(rename = "license-keys", skip_serializing_if = "Option::is_none")]
    pub license_keys: Option<RelationshipLinks>,
    #[serde(
        rename = "discount-redemptions",
        skip_serializing_if = "Option::is_none"
    )]
    pub discount_redemptions: Option<RelationshipLinks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLinksLinks {
    pub related: String,
    #[serde(rename = "self")]
    pub link_self: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLinks {
    pub links: RelationshipLinksLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    pub link_self: String,
}

///////////
// Order //
///////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub r#type: String,
    pub id: String,
    pub attributes: OrderAttributes,
    pub relationships: Option<Relationships>,
    pub links: Links,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAttributes {
    pub store_id: i64,
    pub customer_id: i64,
    pub identifier: String,
    pub order_number: i64,
    pub user_name: String,
    pub user_email: String,
    pub currency: String,
    pub currency_rate: String,
    pub subtotal: i64,
    pub discount_total: i64,
    pub tax: i64,
    pub total: i64,
    pub subtotal_usd: i64,
    pub discount_total_usd: i64,
    pub tax_usd: i64,
    pub total_usd: i64,
    pub tax_name: String,
    pub tax_rate: String,
    pub status: String,
    pub status_formatted: String,
    pub refunded: bool,
    pub refunded_at: String,
    pub subtotal_formatted: String,
    pub discount_total_formatted: String,
    pub tax_formatted: String,
    pub total_formatted: String,
    pub first_order_item: OrderItem,
    pub urls: OrderUrls,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub variant_id: i64,
    pub product_name: String,
    pub variant_name: String,
    pub price: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
    pub test_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUrls {
    pub receipt: String,
}

/////////////////
// Subscription //
/////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionData {
    pub r#type: String,
    pub id: String,
    pub attributes: SubscriptionAttributes,
    pub relationships: Option<Relationships>,
    pub links: Option<Links>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscriptionAttributes {
    pub store_id: i64,
    pub customer_id: i64,
    pub order_id: i64,
    pub order_item_id: i64,
    pub product_id: i64,
    pub variant_id: i64,
    pub product_name: String,
    pub variant_name: String,
    pub user_name: String,
    pub user_email: String,
    pub status: String,
    pub status_formatted: String,
    pub card_brand: String,
    pub card_last_four: String,
    pub pause: Option<String>,
    pub cancelled: bool,
    pub trial_ends_at: Option<String>,
    pub billing_anchor: i64,
    pub first_subscription_item: Option<FirstSubscriptionItem>,
    pub urls: Option<SubscriptionUrls>,
    pub renews_at: String,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub test_mode: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FirstSubscriptionItem {
    pub id: i64,
    pub price_id: i64,
    pub subscription_id: i64,
    pub quantity: i64,
    pub created_at: String,
    pub updated_at: String,
    pub is_usage_based: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubscriptionUrls {
    pub update_payment_method: String,
    pub customer_portal: String,
}
