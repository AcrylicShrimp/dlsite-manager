use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::BTreeMap, fmt};
use url::Url;

pub const DEFAULT_WORKS_BATCH_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkId(String);

impl WorkId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl AsRef<str> for WorkId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for WorkId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentQuery {
    pub last: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub cookies_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Authorized,
    Unauthorized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentCount {
    pub user: u64,
    #[serde(default)]
    pub production: u64,
    #[serde(default)]
    pub page_limit: Option<usize>,
    #[serde(default)]
    pub concurrency: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Purchase {
    #[serde(rename = "workno")]
    pub id: WorkId,
    #[serde(rename = "sales_date", deserialize_with = "deserialize_datetime")]
    pub purchased_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorksResponse {
    pub works: Vec<Work>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    #[serde(rename = "workno")]
    pub id: WorkId,
    pub name: LocalizedText,
    pub maker: Maker,
    #[serde(rename = "work_type")]
    pub work_kind: WorkKind,
    #[serde(rename = "age_category")]
    pub age_category: AgeCategory,
    #[serde(
        rename = "genre_ids",
        default,
        deserialize_with = "deserialize_vec_or_default"
    )]
    pub genre_ids: Vec<i64>,
    #[serde(rename = "work_files")]
    pub thumbnail: WorkThumbnail,
    #[serde(
        rename = "regist_date",
        default,
        deserialize_with = "deserialize_optional_datetime"
    )]
    pub registered_at: Option<DateTime<Utc>>,
    #[serde(
        rename = "sales_date",
        default,
        deserialize_with = "deserialize_optional_datetime"
    )]
    pub published_at: Option<DateTime<Utc>>,
    #[serde(
        rename = "upgrade_date",
        default,
        deserialize_with = "deserialize_optional_datetime"
    )]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub tags: Vec<WorkTag>,
}

pub type LocalizedText = BTreeMap<Language, String>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    English,
    Japanese,
    Korean,
    Taiwanese,
    Chinese,
    Unknown(String),
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Self::English => "en_US",
            Self::Japanese => "ja_JP",
            Self::Korean => "ko_KR",
            Self::Taiwanese => "zh_TW",
            Self::Chinese => "zh_CN",
            Self::Unknown(code) => code,
        }
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "en_US" => Self::English,
            "ja_JP" => Self::Japanese,
            "ko_KR" => Self::Korean,
            "zh_TW" => Self::Taiwanese,
            "zh_CN" => Self::Chinese,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Maker {
    pub id: String,
    pub name: LocalizedText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkKind {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgeCategory {
    All,
    R15,
    R18,
    Unknown(String),
}

impl Serialize for AgeCategory {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Self::All => "all",
            Self::R15 => "r15",
            Self::R18 => "r18",
            Self::Unknown(value) => value,
        };
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for AgeCategory {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "all" => Self::All,
            "r15" => Self::R15,
            "r18" => Self::R18,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkThumbnail {
    #[serde(rename = "main")]
    pub full: Url,
    #[serde(rename = "sam")]
    pub small_square: Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkTag {
    #[serde(rename = "class")]
    pub key: String,
    #[serde(rename = "name")]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadResolution {
    Direct {
        stream_request: DownloadStreamRequest,
    },
    Split {
        location: Url,
    },
    SerialRequired {
        location: Url,
    },
    UnknownRedirect {
        location: Url,
    },
    Unavailable {
        reason: DownloadUnavailableReason,
    },
}

impl DownloadResolution {
    pub fn from_redirect_location(location: Url) -> Self {
        let host = location.host_str().unwrap_or_default();
        let path = location.path();

        if host == "www.dlsite.com"
            && (path.starts_with("/home/download/split") || path.starts_with("/home/split"))
        {
            return Self::Split { location };
        }

        if host == "www.dlsite.com"
            && (path.starts_with("/home/download/serial") || path.starts_with("/home/serial"))
        {
            return Self::SerialRequired { location };
        }

        if host == "www.dlsite.com" && path.starts_with("/home/download") {
            return Self::Direct {
                stream_request: DownloadStreamRequest { url: location },
            };
        }

        Self::UnknownRedirect { location }
    }

    pub fn location(&self) -> Option<&Url> {
        match self {
            Self::Direct { stream_request } => Some(&stream_request.url),
            Self::Split { location }
            | Self::SerialRequired { location }
            | Self::UnknownRedirect { location } => Some(location),
            Self::Unavailable { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadStreamRequest {
    pub url: Url,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitDownloadPage {
    pub page_url: Url,
    pub parts: Vec<SplitDownloadPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitDownloadPart {
    pub number: u32,
    pub stream_request: DownloadStreamRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialDownloadPage {
    pub page_url: Url,
    pub serial_numbers: Vec<SerialNumber>,
    pub stream_request: DownloadStreamRequest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerialNumber {
    pub label: String,
    pub value: String,
}

impl fmt::Debug for SerialNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SerialNumber")
            .field("label", &self.label)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadUnavailableReason {
    NotAuthorized,
    NotFound,
    UnexpectedStatus {
        status: u16,
        body_snippet: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadByteRange {
    start: u64,
    end_inclusive: Option<u64>,
}

impl DownloadByteRange {
    pub const fn from_start(start: u64) -> Self {
        Self {
            start,
            end_inclusive: None,
        }
    }

    pub const fn first_byte() -> Self {
        Self {
            start: 0,
            end_inclusive: Some(0),
        }
    }

    pub fn inclusive(start: u64, end_inclusive: u64) -> Option<Self> {
        if end_inclusive < start {
            return None;
        }

        Some(Self {
            start,
            end_inclusive: Some(end_inclusive),
        })
    }

    pub fn header_value(&self) -> String {
        match self.end_inclusive {
            Some(end) => format!("bytes={}-{}", self.start, end),
            None => format!("bytes={}-", self.start),
        }
    }
}

pub(crate) fn deserialize_datetime<'de, D>(
    deserializer: D,
) -> std::result::Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_dlsite_datetime(&value).map_err(D::Error::custom)
}

pub(crate) fn deserialize_optional_datetime<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .as_deref()
        .map(parse_dlsite_datetime)
        .transpose()
        .map_err(D::Error::custom)
}

pub(crate) fn deserialize_vec_or_default<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

fn parse_dlsite_datetime(value: &str) -> std::result::Result<DateTime<Utc>, String> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(parsed, Utc));
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(parsed, Utc));
    }

    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let datetime = parsed
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("invalid date value: {value}"))?;
        return Ok(DateTime::from_naive_utc_and_offset(datetime, Utc));
    }

    Err(format!("unsupported DLsite datetime value: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_download_redirects() {
        let direct =
            Url::parse("https://www.dlsite.com/home/download/=/product_id/RJ123456.html").unwrap();
        assert!(matches!(
            DownloadResolution::from_redirect_location(direct),
            DownloadResolution::Direct { .. }
        ));

        let split =
            Url::parse("https://www.dlsite.com/home/split/=/product_id/RJ123456.html").unwrap();
        assert!(matches!(
            DownloadResolution::from_redirect_location(split),
            DownloadResolution::Split { .. }
        ));

        let split_download =
            Url::parse("https://www.dlsite.com/home/download/split/=/product_id/RJ123456.html")
                .unwrap();
        assert!(matches!(
            DownloadResolution::from_redirect_location(split_download),
            DownloadResolution::Split { .. }
        ));

        let serial =
            Url::parse("https://www.dlsite.com/home/serial/=/product_id/RJ123456.html").unwrap();
        assert!(matches!(
            DownloadResolution::from_redirect_location(serial),
            DownloadResolution::SerialRequired { .. }
        ));

        let serial_download =
            Url::parse("https://www.dlsite.com/home/download/serial/=/product_id/RJ123456.html")
                .unwrap();
        assert!(matches!(
            DownloadResolution::from_redirect_location(serial_download),
            DownloadResolution::SerialRequired { .. }
        ));
    }

    #[test]
    fn decodes_content_count_limits() {
        let count: ContentCount = serde_json::from_str(
            r#"{"user":1223,"production":0,"page_limit":50,"concurrency":500}"#,
        )
        .unwrap();

        assert_eq!(count.user, 1223);
        assert_eq!(count.production, 0);
        assert_eq!(count.page_limit, Some(50));
        assert_eq!(count.concurrency, Some(500));
    }

    #[test]
    fn formats_download_byte_ranges() {
        assert_eq!(DownloadByteRange::first_byte().header_value(), "bytes=0-0");
        assert_eq!(
            DownloadByteRange::from_start(1024).header_value(),
            "bytes=1024-"
        );
        assert_eq!(
            DownloadByteRange::inclusive(10, 20).unwrap().header_value(),
            "bytes=10-20"
        );
        assert!(DownloadByteRange::inclusive(20, 10).is_none());
    }

    #[test]
    fn decodes_work_fixture() {
        let json = r#"
        {
          "workno": "RJ123456",
          "name": { "ja_JP": "作品", "en_US": "Work" },
          "maker": { "id": "RG00000", "name": { "ja_JP": "サークル" } },
          "work_type": "SOU",
          "age_category": "r18",
          "genre_ids": [1, 2, 3],
          "work_files": {
            "main": "https://img.dlsite.jp/modpub/images2/work/doujin/RJ123000/RJ123456_img_main.jpg",
            "sam": "https://img.dlsite.jp/modpub/images2/work/doujin/RJ123000/RJ123456_img_sam.jpg"
          },
          "regist_date": "2024-01-02T03:04:05Z",
          "sales_date": "2024-01-03",
          "upgrade_date": null,
          "tags": [{ "class": "genre", "name": "tag" }]
        }"#;

        let work: Work = serde_json::from_str(json).unwrap();
        assert_eq!(work.id.as_ref(), "RJ123456");
        assert_eq!(work.name.get(&Language::English).unwrap(), "Work");
        assert_eq!(work.work_kind.code, "SOU");
        assert_eq!(work.age_category, AgeCategory::R18);
        assert_eq!(work.tags[0].value, "tag");
    }
}
