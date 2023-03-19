use crate::application_error::{Error, Result};
use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, fmt::Display, sync::Arc};
use strum_macros::EnumString;

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

#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DLsiteProductType {
    Adult,
    Doujinsji,
    Software,
    Game,
    Action,
    Adventure,
    AudioMaterial,
    Comic,
    DigitalNovel,
    Other,
    OtherGame,
    Illust,
    ImageMaterial,
    Manga,
    Anime,
    Music,
    Novel,
    Puzzle,
    Quiz,
    RolePlaying,
    Gekiga, // See https://en.wikipedia.org/wiki/Gekiga
    Simulation,
    Voice,
    Shooter,
    Tabletop,
    Utility,
    Typing,
    SexualNovel,
    #[strum(default)]
    Unknown(String),
}

impl Default for DLsiteProductType {
    fn default() -> Self {
        Self::Unknown("Unknown".to_owned())
    }
}

impl Display for DLsiteProductType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DLsiteProductType::Adult => write!(f, "Adult"),
            DLsiteProductType::Doujinsji => write!(f, "Doujinsji"),
            DLsiteProductType::Software => write!(f, "Software"),
            DLsiteProductType::Game => write!(f, "Game"),
            DLsiteProductType::Action => write!(f, "Action"),
            DLsiteProductType::Adventure => write!(f, "Adventure"),
            DLsiteProductType::AudioMaterial => write!(f, "AudioMaterial"),
            DLsiteProductType::Comic => write!(f, "Comic"),
            DLsiteProductType::DigitalNovel => write!(f, "DigitalNovel"),
            DLsiteProductType::Other => write!(f, "Other"),
            DLsiteProductType::OtherGame => write!(f, "OtherGame"),
            DLsiteProductType::Illust => write!(f, "Illust"),
            DLsiteProductType::ImageMaterial => write!(f, "ImageMaterial"),
            DLsiteProductType::Manga => write!(f, "Manga"),
            DLsiteProductType::Anime => write!(f, "Anime"),
            DLsiteProductType::Music => write!(f, "Music"),
            DLsiteProductType::Novel => write!(f, "Novel"),
            DLsiteProductType::Puzzle => write!(f, "Puzzle"),
            DLsiteProductType::Quiz => write!(f, "Quiz"),
            DLsiteProductType::RolePlaying => write!(f, "RolePlaying"),
            DLsiteProductType::Gekiga => write!(f, "Gekiga"),
            DLsiteProductType::Simulation => write!(f, "Simulation"),
            DLsiteProductType::Voice => write!(f, "Voice"),
            DLsiteProductType::Shooter => write!(f, "Shooter"),
            DLsiteProductType::Tabletop => write!(f, "Tabletop"),
            DLsiteProductType::Utility => write!(f, "Utility"),
            DLsiteProductType::Typing => write!(f, "Typing"),
            DLsiteProductType::SexualNovel => write!(f, "SexualNovel"),
            DLsiteProductType::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl Serialize for DLsiteProductType {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DLsiteProductType::Adult => serializer.serialize_str("Adult"),
            DLsiteProductType::Doujinsji => serializer.serialize_str("Doujinsji"),
            DLsiteProductType::Software => serializer.serialize_str("Software"),
            DLsiteProductType::Game => serializer.serialize_str("Game"),
            DLsiteProductType::Action => serializer.serialize_str("Action"),
            DLsiteProductType::Adventure => serializer.serialize_str("Adventure"),
            DLsiteProductType::AudioMaterial => serializer.serialize_str("AudioMaterial"),
            DLsiteProductType::Comic => serializer.serialize_str("Comic"),
            DLsiteProductType::DigitalNovel => serializer.serialize_str("DigitalNovel"),
            DLsiteProductType::Other => serializer.serialize_str("Other"),
            DLsiteProductType::OtherGame => serializer.serialize_str("OtherGame"),
            DLsiteProductType::Illust => serializer.serialize_str("Illust"),
            DLsiteProductType::ImageMaterial => serializer.serialize_str("ImageMaterial"),
            DLsiteProductType::Manga => serializer.serialize_str("Manga"),
            DLsiteProductType::Anime => serializer.serialize_str("Anime"),
            DLsiteProductType::Music => serializer.serialize_str("Music"),
            DLsiteProductType::Novel => serializer.serialize_str("Novel"),
            DLsiteProductType::Puzzle => serializer.serialize_str("Puzzle"),
            DLsiteProductType::Quiz => serializer.serialize_str("Quiz"),
            DLsiteProductType::RolePlaying => serializer.serialize_str("RolePlaying"),
            DLsiteProductType::Gekiga => serializer.serialize_str("Gekiga"),
            DLsiteProductType::Simulation => serializer.serialize_str("Simulation"),
            DLsiteProductType::Voice => serializer.serialize_str("Voice"),
            DLsiteProductType::Shooter => serializer.serialize_str("Shooter"),
            DLsiteProductType::Tabletop => serializer.serialize_str("Tabletop"),
            DLsiteProductType::Utility => serializer.serialize_str("Utility"),
            DLsiteProductType::Typing => serializer.serialize_str("Typing"),
            DLsiteProductType::SexualNovel => serializer.serialize_str("SexualNovel"),
            DLsiteProductType::Unknown(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for DLsiteProductType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        Ok(match str.as_str() {
            "ADL" => DLsiteProductType::Adult,
            "DOH" => DLsiteProductType::Doujinsji,
            "SOF" => DLsiteProductType::Software,
            "GAM" => DLsiteProductType::Game,
            "ACN" => DLsiteProductType::Action,
            "ADV" => DLsiteProductType::Adventure,
            "AMT" => DLsiteProductType::AudioMaterial,
            "COM" => DLsiteProductType::Comic,
            "DNV" => DLsiteProductType::DigitalNovel,
            "ET3" => DLsiteProductType::Other,
            "ETC" => DLsiteProductType::OtherGame,
            "ICG" => DLsiteProductType::Illust,
            "IMT" => DLsiteProductType::ImageMaterial,
            "MNG" => DLsiteProductType::Manga,
            "MOV" => DLsiteProductType::Anime,
            "MUS" => DLsiteProductType::Music,
            "NRE" => DLsiteProductType::Novel,
            "PZL" => DLsiteProductType::Puzzle,
            "QIZ" => DLsiteProductType::Quiz,
            "RPG" => DLsiteProductType::RolePlaying,
            "SCM" => DLsiteProductType::Gekiga,
            "SLN" => DLsiteProductType::Simulation,
            "SOU" => DLsiteProductType::Voice,
            "STG" => DLsiteProductType::Shooter,
            "TBL" => DLsiteProductType::Tabletop,
            "TOL" => DLsiteProductType::Utility,
            "TYP" => DLsiteProductType::Typing,
            "KSV" => DLsiteProductType::SexualNovel,
            "VCM" => DLsiteProductType::VoiceComic,
            _ => DLsiteProductType::Unknown(str),
        })
    }
}

#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DLsiteProductAgeCategory {
    All,
    R15,
    R18,
    #[strum(default, to_string = "{0}")]
    Unknown(String),
}

impl Default for DLsiteProductAgeCategory {
    fn default() -> Self {
        Self::Unknown("Unknown".to_owned())
    }
}

impl Display for DLsiteProductAgeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DLsiteProductAgeCategory::All => write!(f, "All"),
            DLsiteProductAgeCategory::R15 => write!(f, "R15"),
            DLsiteProductAgeCategory::R18 => write!(f, "R18"),
            DLsiteProductAgeCategory::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl Serialize for DLsiteProductAgeCategory {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DLsiteProductAgeCategory::All => serializer.serialize_str("All"),
            DLsiteProductAgeCategory::R15 => serializer.serialize_str("R15"),
            DLsiteProductAgeCategory::R18 => serializer.serialize_str("R18"),
            DLsiteProductAgeCategory::Unknown(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for DLsiteProductAgeCategory {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        Ok(match str.as_str() {
            "all" => DLsiteProductAgeCategory::All,
            "r15" => DLsiteProductAgeCategory::R15,
            "r18" => DLsiteProductAgeCategory::R18,
            _ => DLsiteProductAgeCategory::Unknown(str),
        })
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
