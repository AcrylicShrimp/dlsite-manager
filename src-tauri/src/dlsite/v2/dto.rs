use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProduct {
    pub id: String,
    pub ty: DLsiteProductType,
    pub age: DLsiteProductAgeCategory,
    pub title: String,
    pub thumbnail: String,
    pub group_id: String,
    pub group_name: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductListFromOwnerApi {
    pub limit: u32,
    pub offset: u32,
    pub works: Vec<DLsiteProductFromOwnerApi>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductFromOwnerApi {
    #[serde(alias = "workno")]
    pub id: String,
    #[serde(rename = "work_type")]
    pub ty: DLsiteProductType,
    #[serde(rename = "age_category")]
    pub age: DLsiteProductAgeCategory,
    #[serde(alias = "name")]
    pub title: DLsiteProductI18nString,
    #[serde(alias = "work_files")]
    pub icon: DLsiteProductIcon,
    #[serde(alias = "maker")]
    pub group: DLsiteProductGroup,
    #[serde(rename = "regist_date")]
    pub registered_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductI18nString {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductIcon {
    pub main: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductGroup {
    pub id: String,
    pub name: DLsiteProductI18nString,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DLsiteProductFromNonOwnerApi {
    #[serde(rename = "work_type")]
    pub ty: DLsiteProductType,
    #[serde(rename = "age_category")]
    pub age: DLsiteProductAgeCategory,
    #[serde(rename = "work_name")]
    pub title: String,
    #[serde(rename = "work_image")]
    pub thumbnail: String,
    #[serde(rename = "maker_id")]
    pub group_id: String,
    #[serde(rename = "regist_date")]
    /// NOTE: `regist_date` from DLsite response has wrong format `YYYY-MM-DD HH:MM:SS`, so it's unable to parse as `DateTime<Utc>` directly.
    /// Since this struct is only used for parsing JSON, it's okay to keep it as `String` here.
    /// This field will be parsed and converted to `DateTime<Utc>` later in `get_product` function.
    pub registered_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    VoiceComic,
    Unknown(String),
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
            DLsiteProductType::VoiceComic => write!(f, "VoiceComic"),
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
            DLsiteProductType::VoiceComic => serializer.serialize_str("VoiceComic"),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DLsiteProductAgeCategory {
    All, // 1 as integer form
    R15, // 2 as integer form
    R18, // 3 as integer form
    UnknownInt(u64),
    UnknownStr(String),
}

impl Display for DLsiteProductAgeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DLsiteProductAgeCategory::All => write!(f, "All"),
            DLsiteProductAgeCategory::R15 => write!(f, "R15"),
            DLsiteProductAgeCategory::R18 => write!(f, "R18"),
            DLsiteProductAgeCategory::UnknownInt(i) => write!(f, "{}", i),
            DLsiteProductAgeCategory::UnknownStr(s) => write!(f, "{}", s),
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
            DLsiteProductAgeCategory::UnknownInt(i) => serializer.serialize_u64(*i),
            DLsiteProductAgeCategory::UnknownStr(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for DLsiteProductAgeCategory {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = DLsiteProductAgeCategory;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a positive integer or a string")
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    1 => DLsiteProductAgeCategory::All,
                    2 => DLsiteProductAgeCategory::R15,
                    3 => DLsiteProductAgeCategory::R18,
                    _ => DLsiteProductAgeCategory::UnknownInt(value),
                })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "all" => DLsiteProductAgeCategory::All,
                    "r15" => DLsiteProductAgeCategory::R15,
                    "r18" => DLsiteProductAgeCategory::R18,
                    _ => DLsiteProductAgeCategory::UnknownStr(value.to_string()),
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}
