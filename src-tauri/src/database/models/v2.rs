use crate::dlsite::v2::{DLsiteProductAgeCategory, DLsiteProductType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub id: String,
    pub account_id: i32,
    pub ty: DLsiteProductType,
    pub age: DLsiteProductAgeCategory,
    pub title: String,
    pub thumbnail: String,
    pub group_id: String,
    pub group_name: String,
    pub registered_at: DateTime<Utc>,
}
