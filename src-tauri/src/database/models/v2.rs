use crate::dlsite::dto::{DLsiteProductAgeCategory, DLsiteProductType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
    pub product_count: i32,
    pub cookie_json: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleAccount {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatingAccount<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub memo: Option<&'a str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatingAccount<'a> {
    pub id: i64,
    pub username: &'a str,
    pub password: &'a str,
    pub memo: Option<&'a str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub id: String,
    /// it can be `NULL` if the product is not owned by any account (found in local)
    pub account_id: Option<i64>,
    pub ty: DLsiteProductType,
    pub age: DLsiteProductAgeCategory,
    pub title: String,
    pub thumbnail: String,
    pub group_id: String,
    pub group_name: String,
    pub registered_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatingProduct<'a> {
    pub id: &'a str,
    /// it can be `NULL` if the product is not owned by any account (found in local)
    pub account_id: Option<i64>,
    pub ty: DLsiteProductType,
    pub age: DLsiteProductAgeCategory,
    pub title: &'a str,
    pub thumbnail: &'a str,
    pub group_id: &'a str,
    pub group_name: &'a str,
    pub registered_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductDownload {
    pub product_id: String,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatingProductDownload<'a> {
    pub product_id: &'a str,
    pub path: &'a Path,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Setting {
    pub download_root_dir: Option<PathBuf>,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            download_root_dir: None,
        }
    }
}
