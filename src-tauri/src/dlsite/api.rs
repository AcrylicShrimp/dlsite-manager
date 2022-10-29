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
    #[serde(rename(deserialize = "workno"))]
    pub id: String,
    #[serde(rename(deserialize = "work_type"), default)]
    pub ty: DLsiteProductType,
    #[serde(rename(deserialize = "age_category"), default)]
    pub age: DLsiteProductAgeCategory,
    #[serde(rename(deserialize = "name"))]
    pub title: DLsiteProductLocalizedString,
    #[serde(rename(deserialize = "maker"))]
    pub group: DLsiteProductGroup,
    #[serde(rename(deserialize = "work_files"))]
    pub icon: DLsiteProductIcon,
    #[serde(rename(deserialize = "regist_date"))]
    pub registered_at: Option<DateTime<Utc>>,
    #[serde(rename(deserialize = "upgrade_date"))]
    pub upgraded_at: Option<DateTime<Utc>>,
    #[serde(rename(deserialize = "sales_date"))]
    pub purchased_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLsiteProductLocalizedString {
    #[serde(rename(deserialize = "ja_JP"))]
    pub japanese: Option<String>,
    #[serde(rename(deserialize = "en_US"))]
    pub english: Option<String>,
    #[serde(rename(deserialize = "ko_KR"))]
    pub korean: Option<String>,
    #[serde(rename(deserialize = "zh_TW"))]
    pub taiwanese: Option<String>,
    #[serde(rename(deserialize = "zh_CN"))]
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
    #[serde(rename(deserialize = "ADL"))]
    Adult,
    #[serde(rename(deserialize = "DOH"))]
    Doujinsji,
    #[serde(rename(deserialize = "SOF"))]
    Software,
    #[serde(rename(deserialize = "GAM"))]
    Game,
    #[serde(rename(deserialize = "ACN"))]
    Action,
    #[serde(rename(deserialize = "ADV"))]
    Adventure,
    #[serde(rename(deserialize = "AMT"))]
    AudioMaterial,
    #[serde(rename(deserialize = "COM"))]
    Comic,
    #[serde(rename(deserialize = "DNV"))]
    DigitalNovel,
    #[serde(rename(deserialize = "ET3"))]
    Other,
    #[serde(rename(deserialize = "ETC"))]
    OtherGame,
    #[serde(rename(deserialize = "ICG"))]
    Illust,
    #[serde(rename(deserialize = "IMT"))]
    ImageMaterial,
    #[serde(rename(deserialize = "MNG"))]
    Manga,
    #[serde(rename(deserialize = "MOV"))]
    Anime,
    #[serde(rename(deserialize = "MUS"))]
    Music,
    #[serde(rename(deserialize = "NRE"))]
    Novel,
    #[serde(rename(deserialize = "PZL"))]
    Puzzle,
    #[serde(rename(deserialize = "QIZ"))]
    Quiz,
    #[serde(rename(deserialize = "RPG"))]
    RolePlaying,
    #[serde(rename(deserialize = "SCM"))]
    Gekiga, // See https://en.wikipedia.org/wiki/Gekiga
    #[serde(rename(deserialize = "SLN"))]
    Simulation,
    #[serde(rename(deserialize = "SOU"))]
    Voice,
    #[serde(rename(deserialize = "STG"))]
    Shooter,
    #[serde(rename(deserialize = "TBL"))]
    Tabletop,
    #[serde(rename(deserialize = "TOL"))]
    Utility,
    #[serde(rename(deserialize = "TYP"))]
    Typing,
    #[serde(rename(deserialize = "KSV"))]
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
    #[serde(rename(deserialize = "all"))]
    All,
    #[serde(rename(deserialize = "r15"))]
    R15,
    #[serde(rename(deserialize = "r18"))]
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
    #[serde(rename(deserialize = "sam"))]
    pub small: String,
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
        Err(err) => Err(Error::DLsiteNotAuthenticated),
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
        Err(err) => Err(Error::DLsiteNotAuthenticated),
    }
}
