use crate::dlsite::v2::{DLsiteProductAgeCategory, DLsiteProductType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
    pub product_count: i32,
    pub cookie_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatingAccount<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub memo: Option<&'a str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub id: String,
    pub account_id: i64,
    pub ty: DLsiteProductType,
    pub age: DLsiteProductAgeCategory,
    pub title: String,
    pub thumbnail: String,
    pub group_id: String,
    pub group_name: String,
    pub registered_at: DateTime<Utc>,
}
