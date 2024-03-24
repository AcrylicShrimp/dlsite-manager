use crate::dlsite::api::{
    DLsiteProduct, DLsiteProductAgeCategory, DLsiteProductGroup, DLsiteProductIcon,
    DLsiteProductLocalizedString, DLsiteProductType,
};
use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};
use strum_macros::{EnumString, IntoStaticStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for Account {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            username: row.get("username")?,
            password: row.get("password")?,
            memo: row.get("memo")?,
            product_count: row.get("product_count")?,
            cookie_json: row.get("cookie_json")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAccount {
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedAccount {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLanguageSetting {
    pub languages: Vec<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LatestProductQuery {
    pub query: ProductQuery,
    pub download: Option<ProductDownloadState>,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for LatestProductQuery {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            query: ProductQuery {
                query: row.get::<_, Option<String>>("query")?,
                ty: row
                    .get_ref("ty")?
                    .as_str_or_null()?
                    .map(|ty| {
                        <_>::from_str(ty).map_err(|err: strum::ParseError| {
                            rusqlite::Error::FromSqlConversionFailure(
                                row.as_ref().column_index("ty").unwrap(),
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })
                    })
                    .transpose()?,
                age: row
                    .get_ref("age")?
                    .as_str_or_null()?
                    .map(|age| {
                        <_>::from_str(age).map_err(|err: strum::ParseError| {
                            rusqlite::Error::FromSqlConversionFailure(
                                row.as_ref().column_index("age").unwrap(),
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })
                    })
                    .transpose()?,
                order_by: <_>::from_str(row.get_ref("order_by")?.as_str()?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("order_by").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
            },
            download: row
                .get_ref("download")?
                .as_str_or_null()?
                .map(|download| {
                    <_>::from_str(download).map_err(|err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("download").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })
                })
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: i64,
    pub account: Account,
    pub product: DLsiteProduct,
    pub download: Option<ProductDownload>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownload {
    pub id: i64,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for ProductDownload {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            path: PathBuf::from(row.get::<_, String>("path")?),
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(
    EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ProductQueryOrderBy {
    IdAsc,
    IdDesc,
    TitleAsc,
    TitleDesc,
    GroupAsc,
    GroupDesc,
    RegistrationDateAsc,
    RegistrationDateDesc,
    PurchaseDateAsc,
    PurchaseDateDesc,
}

#[derive(
    EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ProductDownloadState {
    NotDownloaded,
    Downloading,
    Downloaded,
    DownloadingAndDownloaded,
}

impl Default for ProductQueryOrderBy {
    fn default() -> Self {
        Self::PurchaseDateDesc
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuery {
    pub query: Option<String>,
    pub ty: Option<DLsiteProductType>,
    pub age: Option<DLsiteProductAgeCategory>,
    pub order_by: ProductQueryOrderBy,
}

#[derive(Debug, Clone)]
pub struct InsertedProduct {
    pub account_id: i64,
    pub product: DLsiteProduct,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for Product {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            account: Account {
                id: row.get("account_id")?,
                username: row.get("account_username")?,
                password: row.get("account_password")?,
                memo: row.get("account_memo")?,
                product_count: row.get("account_product_count")?,
                cookie_json: row.get("account_cookie_json")?,
                created_at: row.get("account_created_at")?,
                updated_at: row.get("account_updated_at")?,
            },
            product: DLsiteProduct {
                id: row.get("product_id")?,
                ty: <_>::from_str(&row.get::<_, String>("product_type")?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("product_type").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
                age: <_>::from_str(&row.get::<_, String>("product_age")?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("product_type").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
                title: DLsiteProductLocalizedString {
                    japanese: row.get("product_title_ja")?,
                    english: row.get("product_title_en")?,
                    korean: row.get("product_title_ko")?,
                    taiwanese: row.get("product_title_tw")?,
                    chinese: row.get("product_title_cn")?,
                },
                group: DLsiteProductGroup {
                    id: row.get("product_group_id")?,
                    name: DLsiteProductLocalizedString {
                        japanese: row.get("product_group_name_ja")?,
                        english: row.get("product_group_name_en")?,
                        korean: row.get("product_group_name_ko")?,
                        taiwanese: row.get("product_group_name_tw")?,
                        chinese: row.get("product_group_name_cn")?,
                    },
                },
                icon: DLsiteProductIcon {
                    main: row.get("product_icon_main")?,
                    small: row.get("product_icon_small")?,
                },
                registered_at: row.get("registered_at")?,
                upgraded_at: row.get("upgraded_at")?,
                purchased_at: row.get("purchased_at")?,
            },
            download: {
                if let Some(id) = row.get("download_id")? {
                    Some(ProductDownload {
                        id,
                        path: PathBuf::from(row.get::<_, String>("download_path")?),
                        created_at: row.get("download_created_at")?,
                    })
                } else {
                    None
                }
            },
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub download_root_dir: Option<PathBuf>,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for Setting {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            download_root_dir: row
                .get::<_, Option<String>>("download_root_dir")?
                .map(|path| PathBuf::from(path)),
        })
    }
}
