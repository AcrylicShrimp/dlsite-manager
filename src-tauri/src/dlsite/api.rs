use crate::application_error::{Error, Result};
use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use strum_macros::{EnumString, IntoStaticStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductList {
    pub last: DateTime<Utc>,
    pub limit: usize,
    pub offset: usize,
    pub works: Vec<DLsiteProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProduct {
    #[serde(alias = "workno")]
    pub id: String,
    #[serde(alias = "work_type", default)]
    pub ty: DLsiteProductType,
    #[serde(alias = "age_category", default)]
    pub age: DLsiteProductAgeCategory,
    #[serde(alias = "name")]
    pub title: DLsiteProductLocalizedString,
    #[serde(alias = "maker")]
    pub group: DLsiteProductGroup,
    #[serde(alias = "work_files")]
    pub icon: DLsiteProductIcon,
    #[serde(alias = "regist_date")]
    pub registered_at: Option<DateTime<Utc>>,
    #[serde(alias = "upgrade_date")]
    pub upgraded_at: Option<DateTime<Utc>>,
    #[serde(alias = "sales_date")]
    pub purchased_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductLocalizedString {
    #[serde(alias = "ja_JP")]
    pub japanese: Option<String>,
    #[serde(alias = "en_US")]
    pub english: Option<String>,
    #[serde(alias = "ko_KR")]
    pub korean: Option<String>,
    #[serde(alias = "zh_TW")]
    pub taiwanese: Option<String>,
    #[serde(alias = "zh_CN")]
    pub chinese: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductGroup {
    pub id: String,
    pub name: DLsiteProductLocalizedString,
}

#[derive(
    EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum DLsiteProductType {
    Unknown,
    #[serde(alias = "ADL")]
    Adult,
    #[serde(alias = "DOH")]
    Doujinsji,
    #[serde(alias = "SOF")]
    Software,
    #[serde(alias = "GAM")]
    Game,
    #[serde(alias = "ACN")]
    Action,
    #[serde(alias = "ADV")]
    Adventure,
    #[serde(alias = "AMT")]
    AudioMaterial,
    #[serde(alias = "COM")]
    Comic,
    #[serde(alias = "DNV")]
    DigitalNovel,
    #[serde(alias = "ET3")]
    Other,
    #[serde(alias = "ETC")]
    OtherGame,
    #[serde(alias = "ICG")]
    Illust,
    #[serde(alias = "IMT")]
    ImageMaterial,
    #[serde(alias = "MNG")]
    Manga,
    #[serde(alias = "MOV")]
    Anime,
    #[serde(alias = "MUS")]
    Music,
    #[serde(alias = "NRE")]
    Novel,
    #[serde(alias = "PZL")]
    Puzzle,
    #[serde(alias = "QIZ")]
    Quiz,
    #[serde(alias = "RPG")]
    RolePlaying,
    #[serde(alias = "SCM")]
    Gekiga, // See https://en.wikipedia.org/wiki/Gekiga
    #[serde(alias = "SLN")]
    Simulation,
    #[serde(alias = "SOU")]
    Voice,
    #[serde(alias = "STG")]
    Shooter,
    #[serde(alias = "TBL")]
    Tabletop,
    #[serde(alias = "TOL")]
    Utility,
    #[serde(alias = "TYP")]
    Typing,
    #[serde(alias = "KSV")]
    SexualNovel,
}

impl Default for DLsiteProductType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(
    EnumString, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum DLsiteProductAgeCategory {
    #[serde(alias = "all")]
    All,
    #[serde(alias = "r15")]
    R15,
    #[serde(alias = "r18")]
    R18,
}

impl Default for DLsiteProductAgeCategory {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DLsiteProductIcon {
    pub main: String,
    #[serde(alias = "sam")]
    pub small: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductDetail {
    #[serde(alias = "image_main")]
    pub image: DLsiteProductDetailImage,
    pub contents: Vec<DLsiteProductDetailContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductDetailImage {
    pub file_name: String,
    pub file_size: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductDetailContent {
    pub file_name: String,
    pub file_size: String,
}

pub async fn login(
    username: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<Arc<CookieStoreMutex>> {
    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store.clone())
        .build()?;

    client
        .get("https://www.dlsite.com/maniax/login/=/skip_register/1")
        .send()
        .await?;
    client.get("https://login.dlsite.com/login").send().await?;

    // We must ignore the response of this login request here.
    // The DLsite always responds with normal 200 status even if the login has been failed.
    client
        .post("https://login.dlsite.com/login")
        .form(&[
            ("login_id", username.as_ref()),
            ("password", password.as_ref()),
            ("_token", &{
                let cookie = cookie_store
                    .lock()
                    .unwrap()
                    .get("login.dlsite.com", "/", "XSRF-TOKEN")
                    .ok_or_else(|| Error::DLsiteCookieNotFound {
                        cookie_domain: "login.dlsite.com".to_owned(),
                        cookie_path: "/".to_owned(),
                        cookie_name: "XSRF-TOKEN".to_owned(),
                    })?
                    .value()
                    .to_owned();
                cookie
            }),
        ])
        .send()
        .await?;

    Ok(cookie_store)
}

pub async fn get_product_count(cookie_store: Arc<CookieStoreMutex>) -> Result<usize> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;
    let response = client
        .get("https://play.dlsite.com/api/product_count")
        .send()
        .await?;

    // The body of the response will be a valid json if the login has been succeed.
    match response.json::<HashMap<String, usize>>().await {
        Ok(product_count) => Ok(product_count.get("user").cloned().unwrap_or(0)),
        Err(..) => Err(Error::DLsiteNotAuthenticated),
    }
}

pub async fn get_product(
    cookie_store: Arc<CookieStoreMutex>,
    page: usize,
) -> Result<Vec<DLsiteProduct>> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;
    let response = client
        .get(format!(
            "https://play.dlsite.com/api/purchases?page={}",
            page
        ))
        .send()
        .await?;

    // The body of the response will be a valid json if the login has been succeed.
    match response.json::<DLsiteProductList>().await {
        Ok(product_list) => Ok(product_list.works),
        Err(..) => Err(Error::DLsiteNotAuthenticated),
    }
}

pub async fn get_product_details(
    cookie_store: Arc<CookieStoreMutex>,
    product_id: impl AsRef<str>,
) -> Result<Vec<DLsiteProductDetail>> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;
    let response = client
        .get(format!(
            "https://www.dlsite.com/maniax/api/=/product.json?workno={}",
            product_id.as_ref()
        ))
        .send()
        .await?;

    // The body of the response will be a valid json if the login has been succeed.
    match response.json::<Vec<DLsiteProductDetail>>().await {
        Ok(details) => Ok(details),
        Err(err) => {
            println!("{:#?}", err);
            Err(Error::DLsiteNotAuthenticated)
        }
    }
}
