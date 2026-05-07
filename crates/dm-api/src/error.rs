use reqwest::StatusCode;
use url::Url;

pub type Result<T> = std::result::Result<T, DmApiError>;

#[derive(Debug, thiserror::Error)]
pub enum DmApiError {
    #[error("credentials are incorrect")]
    InvalidCredentials,
    #[error("not authorized")]
    NotAuthorized,
    #[error("XSRF-TOKEN not found in cookie jar")]
    XsrfTokenNotFound,
    #[error("Location header not found for {endpoint}")]
    LocationHeaderMissing { endpoint: Url },
    #[error("invalid Location header for {endpoint}: {location}")]
    InvalidLocationHeader {
        endpoint: Url,
        location: String,
        #[source]
        source: url::ParseError,
    },
    #[error("unexpected status {status} from {endpoint}: {body_snippet:?}")]
    UnexpectedStatus {
        endpoint: Url,
        status: StatusCode,
        body_snippet: Option<String>,
    },
    #[error("unexpected API response from {endpoint}")]
    UnexpectedResponse {
        endpoint: Url,
        #[source]
        source: reqwest::Error,
    },
    #[error("unexpected JSON shape from {endpoint} at {path}")]
    UnexpectedJson {
        endpoint: Url,
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("redirect limit exceeded while requesting {endpoint}; limit: {limit}")]
    RedirectLimitExceeded { endpoint: Url, limit: usize },
    #[error("{kind} download page link not found at {page}")]
    DownloadPageLinkNotFound { page: Url, kind: &'static str },
    #[error("works batch limit exceeded; detected limit: {limit}")]
    BatchLimitExceeded {
        limit: usize,
        body_snippet: Option<String>,
    },
    #[error("cookie store error: {0}")]
    CookieStore(String),
    #[error("HTTP request failed")]
    Request(#[from] reqwest::Error),
    #[error("URL parse failed")]
    Url(#[from] url::ParseError),
    #[error("JSON operation failed")]
    Json(#[from] serde_json::Error),
}
