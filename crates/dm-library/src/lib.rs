use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use dm_api::{
    ContentCount, ContentQuery, Credentials, DlsiteClient, DmApiError, DownloadFile, DownloadPlan,
    Language, LocalizedText, LoginOutcome, PublicWork, Purchase, SerialNumber, SessionSnapshot,
    SessionStatus, TwoFactorChallenge, Work, WorkId,
};
use dm_credentials::{CredentialRef, CredentialStore, CredentialsError};
use dm_download::{
    DownloadFileMetadata, DownloadJobRequest, DownloadProgress, DownloadedWork, UnpackPolicy,
};
pub use dm_jobs::CancellationToken;
use dm_storage::{
    Account, AccountSyncCommit, AccountUpsert, AccountWork, CachedWork, LocalWorkDownloadImport,
    ProductCustomTag, ProductDetail, ProductFilterFacets, ProductListPage, ProductListQuery,
    Storage, StorageError, SyncCancellation, SyncFailure, WorkDownloadState, WorkDownloadStatus,
    WorkDownloadUpdate,
};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, LibraryError>;

const BULK_DOWNLOAD_PAGE_LIMIT: u32 = 500;
const DOWNLOAD_CANCELLATION_POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(50);
const SERIAL_INFORMATION_FILE_NAME: &str = "dlsite-manager-serial.txt";
const SERIAL_INFORMATION_NUMBERED_PREFIX: &str = "dlsite-manager-serial-";
const SERIAL_INFORMATION_MARKER: &str = "# dlsite-manager serial information";
mod auth;
mod finalization;
use auth::acquire_login_lock;
pub use auth::{
    DlsiteAuthSource, TwoFactorPrompt, TwoFactorPromptRequest, TwoFactorPromptResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("storage error")]
    Storage(#[from] StorageError),
    #[error("credential error: {0}")]
    Credentials(#[from] CredentialsError),
    #[error("dlsite api error")]
    Api(#[from] DmApiError),
    #[error("sync source error: {0}")]
    SyncSource(String),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("account is disabled: {0}")]
    AccountDisabled(String),
    #[error("account has no login name: {0}")]
    MissingLoginName(String),
    #[error("account has no available password: {0}")]
    MissingPassword(String),
    #[error("two-factor verification is required but no prompt is available: {0}")]
    TwoFactorPromptUnavailable(String),
    #[error("two-factor verification was cancelled: {0}")]
    TwoFactorCancelled(String),
    #[error("two-factor verification timed out: {0}")]
    TwoFactorTimedOut(String),
    #[error("two-factor verification failed after {attempts} attempts: {account_id}")]
    TwoFactorRejected { account_id: String, attempts: u32 },
    #[error("sync was cancelled")]
    Cancelled,
    #[error("bulk download interrupted: {cause}")]
    BulkInterrupted {
        cause: Box<LibraryError>,
        report: Box<BulkWorkDownloadReport>,
        interrupted_work: Option<String>,
    },
    #[error("download error")]
    Download(#[from] dm_download::DownloadError),
    #[error("work is not owned by an enabled account: {0}")]
    DownloadAccountNotFound(String),
    #[error("download final path already exists: {0}")]
    DownloadTargetExists(PathBuf),
    #[error("download path is outside configured roots: {0}")]
    DownloadPathOutsideRoots(PathBuf),
    #[error("download path is not a directory: {0}")]
    DownloadPathNotDirectory(PathBuf),
    #[error("download recovery required: {0}")]
    RecoveryRequired(String),
    #[error("download attempt left recoverable data at {retained_path}")]
    RetainedDownload {
        source: Box<LibraryError>,
        retained_path: PathBuf,
    },
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("json error")]
    Json(#[from] serde_json::Error),
}

impl LibraryError {
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled
            | Self::TwoFactorCancelled(_)
            | Self::Download(dm_download::DownloadError::Cancelled) => true,
            Self::BulkInterrupted { cause, .. } => cause.is_cancelled(),
            Self::RetainedDownload { source, .. } => source.is_cancelled(),
            _ => false,
        }
    }

    pub fn failure_code(&self) -> &'static str {
        if self.is_cancelled() {
            return "cancelled";
        }
        match self {
            Self::BulkInterrupted { cause, .. } => cause.failure_code(),
            Self::RetainedDownload { source, .. } => source.failure_code(),
            Self::RecoveryRequired(_) => "download_recovery_required",
            Self::Storage(_) => "storage",
            Self::Credentials(_) => "credentials",
            Self::Api(_) => "api",
            Self::SyncSource(_) => "sync_source",
            Self::AccountNotFound(_) => "account_not_found",
            Self::AccountDisabled(_) => "account_disabled",
            Self::MissingLoginName(_) => "missing_login_name",
            Self::MissingPassword(_) => "missing_password",
            Self::TwoFactorPromptUnavailable(_) => "two_factor_prompt_unavailable",
            // Reported as a plain cancellation so `dm-jobs` finishes the job as Cancelled
            // rather than Failed; `support_details` keeps the two-factor specifics.
            Self::TwoFactorCancelled(_) => "cancelled",
            Self::TwoFactorTimedOut(_) => "two_factor_timed_out",
            Self::TwoFactorRejected { .. } => "two_factor_rejected",
            Self::Cancelled => "cancelled",
            Self::Download(_) => "download",
            Self::DownloadAccountNotFound(_) => "download_account_not_found",
            Self::DownloadTargetExists(_) => "download_target_exists",
            Self::DownloadPathOutsideRoots(_) => "download_path_outside_roots",
            Self::DownloadPathNotDirectory(_) => "download_path_not_directory",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
        }
    }

    pub fn support_message(&self) -> String {
        match self {
            Self::BulkInterrupted { cause, .. } => cause.support_message(),
            Self::RetainedDownload { source, retained_path } => format!("{}; recoverable data retained at {}", source.support_message(), retained_path.display()),
            Self::RecoveryRequired(message) => message.clone(),
            Self::Storage(error) => format!("Storage error: {error}"),
            Self::Credentials(error) => format!("Credential error: {error}"),
            Self::Api(error) => dm_api_error_support_message(error),
            Self::SyncSource(error) => format!("Sync source error: {error}"),
            Self::AccountNotFound(account_id) => format!("Account not found: {account_id}"),
            Self::AccountDisabled(account_id) => format!("Account is disabled: {account_id}"),
            Self::MissingLoginName(account_id) => {
                format!("Account has no login name: {account_id}")
            }
            Self::MissingPassword(account_id) => {
                format!("Account has no saved password: {account_id}")
            }
            Self::TwoFactorPromptUnavailable(account_id) => format!(
                "Account {account_id} needs a two-factor code, but this operation cannot ask for one"
            ),
            Self::TwoFactorCancelled(account_id) => {
                format!("Two-factor verification was cancelled for account {account_id}")
            }
            Self::TwoFactorTimedOut(account_id) => format!(
                "No two-factor code was entered for account {account_id} before the prompt expired"
            ),
            Self::TwoFactorRejected {
                account_id,
                attempts,
            } => format!(
                "DLsite rejected the two-factor code for account {account_id} after {attempts} attempts"
            ),
            Self::Cancelled => "Operation was cancelled".to_owned(),
            Self::Download(error) => download_error_support_message(error),
            Self::DownloadAccountNotFound(work_id) => {
                format!("Work is not owned by an enabled account: {work_id}")
            }
            Self::DownloadTargetExists(path) => {
                format!("Download final path already exists: {}", path.display())
            }
            Self::DownloadPathOutsideRoots(path) => {
                format!(
                    "Download path is outside configured roots: {}",
                    path.display()
                )
            }
            Self::DownloadPathNotDirectory(path) => {
                format!("Download path is not a directory: {}", path.display())
            }
            Self::Io(error) => format!("I/O error: {error}"),
            Self::Json(error) => format!("JSON error: {error}"),
        }
    }

    pub fn support_details(&self) -> Value {
        match self {
            Self::BulkInterrupted { cause, .. } => cause.support_details(),
            Self::RetainedDownload {
                source,
                retained_path,
            } => {
                let mut details = value_object(source.support_details());
                details.insert("retainedPath".to_owned(), json!(retained_path));
                Value::Object(details)
            }
            Self::RecoveryRequired(message) => {
                json!({ "failureKind": "download_recovery_required", "message": message })
            }
            Self::Storage(error) => json!({
                "failureKind": "storage",
                "message": error.to_string(),
            }),
            Self::Credentials(error) => json!({
                "failureKind": "credentials",
                "message": error.to_string(),
            }),
            Self::Api(error) => dm_api_error_support_details(error),
            Self::SyncSource(error) => json!({
                "failureKind": "sync_source",
                "message": error,
            }),
            Self::AccountNotFound(account_id) => json!({
                "failureKind": "account_not_found",
                "accountId": account_id,
            }),
            Self::AccountDisabled(account_id) => json!({
                "failureKind": "account_disabled",
                "accountId": account_id,
            }),
            Self::MissingLoginName(account_id) => json!({
                "failureKind": "missing_login_name",
                "accountId": account_id,
            }),
            Self::MissingPassword(account_id) => json!({
                "failureKind": "missing_password",
                "accountId": account_id,
            }),
            Self::TwoFactorPromptUnavailable(account_id) => json!({
                "failureKind": "two_factor_prompt_unavailable",
                "accountId": account_id,
            }),
            Self::TwoFactorCancelled(account_id) => json!({
                "failureKind": "two_factor_cancelled",
                "accountId": account_id,
            }),
            Self::TwoFactorTimedOut(account_id) => json!({
                "failureKind": "two_factor_timed_out",
                "accountId": account_id,
            }),
            Self::TwoFactorRejected {
                account_id,
                attempts,
            } => json!({
                "failureKind": "two_factor_rejected",
                "accountId": account_id,
                "attempts": attempts,
            }),
            Self::Cancelled => json!({
                "failureKind": "cancelled",
            }),
            Self::Download(error) => download_error_support_details(error),
            Self::DownloadAccountNotFound(work_id) => json!({
                "failureKind": "download_account_not_found",
                "workId": work_id,
            }),
            Self::DownloadTargetExists(path) => json!({
                "failureKind": "download_target_exists",
                "path": path.to_string_lossy(),
            }),
            Self::DownloadPathOutsideRoots(path) => json!({
                "failureKind": "download_path_outside_roots",
                "path": path.to_string_lossy(),
            }),
            Self::DownloadPathNotDirectory(path) => json!({
                "failureKind": "download_path_not_directory",
                "path": path.to_string_lossy(),
            }),
            Self::Io(error) => json!({
                "failureKind": "io",
                "message": error.to_string(),
            }),
            Self::Json(error) => json!({
                "failureKind": "json",
                "message": error.to_string(),
            }),
        }
    }
}

fn dm_api_error_support_message(error: &DmApiError) -> String {
    match error {
        DmApiError::InvalidCredentials => "DLsite rejected the account credentials".to_owned(),
        DmApiError::NotAuthorized => "DLsite session is not authorized".to_owned(),
        DmApiError::TwoFactorRequired => {
            "DLsite requires a two-factor verification code for this account".to_owned()
        }
        DmApiError::InvalidTwoFactorCode => {
            "DLsite rejected the two-factor verification code".to_owned()
        }
        DmApiError::TwoFactorFormNotRecognized { page, .. } => {
            format!(
                "DLsite two-factor challenge page could not be read at {}",
                safe_url_for_log(page.as_str())
            )
        }
        DmApiError::XsrfTokenNotFound => "DLsite session did not provide an XSRF token".to_owned(),
        DmApiError::LocationHeaderMissing { endpoint } => {
            format!(
                "DLsite redirect response did not include a Location header at {}",
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::InvalidLocationHeader { endpoint, .. } => {
            format!(
                "DLsite returned an invalid redirect Location header at {}",
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::UnexpectedStatus {
            endpoint, status, ..
        } => {
            format!(
                "DLsite returned HTTP {} from {}",
                status.as_u16(),
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::UnexpectedResponse { endpoint, .. } => {
            format!(
                "DLsite returned an unexpected response from {}",
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::UnexpectedJson { endpoint, path, .. } => {
            format!(
                "DLsite returned an unexpected JSON shape at {path} from {}",
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::RedirectLimitExceeded { endpoint, limit } => {
            format!(
                "DLsite redirect limit exceeded after {limit} redirects from {}",
                safe_url_for_log(endpoint.as_str())
            )
        }
        DmApiError::DownloadPageLinkNotFound { kind, page } => {
            format!(
                "DLsite {kind} download page did not contain an expected download link at {}",
                safe_url_for_log(page.as_str())
            )
        }
        DmApiError::DownloadUnavailable { work_id, reason } => {
            format!("DLsite reports download unavailable for {work_id}: {reason:?}")
        }
        DmApiError::DownloadUnknownRedirect { work_id, location } => {
            format!(
                "DLsite returned an unknown download redirect for {work_id}: {}",
                safe_url_for_log(location.as_str())
            )
        }
        DmApiError::BatchLimitExceeded { limit, .. } => {
            format!("DLsite works batch limit exceeded; detected limit: {limit}")
        }
        DmApiError::CookieStore(error) => format!("DLsite cookie store error: {error}"),
        DmApiError::Request(error) => format!("DLsite HTTP request failed: {error}"),
        DmApiError::Url(error) => format!("DLsite URL parse failed: {error}"),
        DmApiError::Json(error) => format!("DLsite JSON operation failed: {error}"),
    }
}

fn dm_api_error_support_details(error: &DmApiError) -> Value {
    match error {
        DmApiError::InvalidCredentials => json!({
            "failureKind": "api",
            "apiErrorKind": "invalid_credentials",
        }),
        DmApiError::NotAuthorized => json!({
            "failureKind": "api",
            "apiErrorKind": "not_authorized",
        }),
        DmApiError::TwoFactorRequired => json!({
            "failureKind": "api",
            "apiErrorKind": "two_factor_required",
        }),
        DmApiError::InvalidTwoFactorCode => json!({
            "failureKind": "api",
            "apiErrorKind": "invalid_two_factor_code",
        }),
        DmApiError::TwoFactorFormNotRecognized { page, .. } => json!({
            "failureKind": "api",
            "apiErrorKind": "two_factor_form_not_recognized",
            "page": safe_url_for_log(page.as_str()),
        }),
        DmApiError::XsrfTokenNotFound => json!({
            "failureKind": "api",
            "apiErrorKind": "xsrf_token_not_found",
        }),
        DmApiError::LocationHeaderMissing { endpoint } => json!({
            "failureKind": "api",
            "apiErrorKind": "location_header_missing",
            "endpoint": safe_url_for_log(endpoint.as_str()),
        }),
        DmApiError::InvalidLocationHeader {
            endpoint,
            location,
            source,
        } => json!({
            "failureKind": "api",
            "apiErrorKind": "invalid_location_header",
            "endpoint": safe_url_for_log(endpoint.as_str()),
            "location": safe_url_for_log(location),
            "parseError": source.to_string(),
        }),
        DmApiError::UnexpectedStatus {
            endpoint,
            status,
            body_snippet,
        } => {
            let mut details = Map::new();
            details.insert("failureKind".to_owned(), json!("api"));
            details.insert("apiErrorKind".to_owned(), json!("unexpected_status"));
            details.insert(
                "endpoint".to_owned(),
                json!(safe_url_for_log(endpoint.as_str())),
            );
            details.insert("httpStatus".to_owned(), json!(status.as_u16()));
            details.insert(
                "httpStatusText".to_owned(),
                json!(status.canonical_reason().unwrap_or("unknown")),
            );

            if let Some(snippet) = body_snippet.as_deref().and_then(safe_body_snippet_for_log) {
                details.insert("bodySnippet".to_owned(), json!(snippet));
            }

            Value::Object(details)
        }
        DmApiError::UnexpectedResponse { endpoint, source } => json!({
            "failureKind": "api",
            "apiErrorKind": "unexpected_response",
            "endpoint": safe_url_for_log(endpoint.as_str()),
            "source": source.to_string(),
        }),
        DmApiError::UnexpectedJson {
            endpoint,
            path,
            source,
        } => json!({
            "failureKind": "api",
            "apiErrorKind": "unexpected_json",
            "endpoint": safe_url_for_log(endpoint.as_str()),
            "jsonPath": path,
            "source": source.to_string(),
        }),
        DmApiError::RedirectLimitExceeded { endpoint, limit } => json!({
            "failureKind": "api",
            "apiErrorKind": "redirect_limit_exceeded",
            "endpoint": safe_url_for_log(endpoint.as_str()),
            "limit": limit,
        }),
        DmApiError::DownloadPageLinkNotFound { page, kind } => json!({
            "failureKind": "api",
            "apiErrorKind": "download_page_link_not_found",
            "page": safe_url_for_log(page.as_str()),
            "pageKind": kind,
        }),
        DmApiError::DownloadUnavailable { work_id, reason } => json!({
            "failureKind": "api",
            "apiErrorKind": "download_unavailable",
            "workId": work_id.as_ref(),
            "reason": format!("{reason:?}"),
        }),
        DmApiError::DownloadUnknownRedirect { work_id, location } => json!({
            "failureKind": "api",
            "apiErrorKind": "download_unknown_redirect",
            "workId": work_id.as_ref(),
            "location": safe_url_for_log(location.as_str()),
        }),
        DmApiError::BatchLimitExceeded {
            limit,
            body_snippet,
        } => {
            let mut details = Map::new();
            details.insert("failureKind".to_owned(), json!("api"));
            details.insert("apiErrorKind".to_owned(), json!("batch_limit_exceeded"));
            details.insert("limit".to_owned(), json!(limit));

            if let Some(snippet) = body_snippet.as_deref().and_then(safe_body_snippet_for_log) {
                details.insert("bodySnippet".to_owned(), json!(snippet));
            }

            Value::Object(details)
        }
        DmApiError::CookieStore(error) => json!({
            "failureKind": "api",
            "apiErrorKind": "cookie_store",
            "source": error,
        }),
        DmApiError::Request(error) => {
            let mut details = Map::new();
            details.insert("failureKind".to_owned(), json!("api"));
            details.insert("apiErrorKind".to_owned(), json!("request"));
            details.insert("source".to_owned(), json!(error.to_string()));
            details.insert("isTimeout".to_owned(), json!(error.is_timeout()));
            details.insert("isConnect".to_owned(), json!(error.is_connect()));
            details.insert("isStatus".to_owned(), json!(error.is_status()));

            if let Some(status) = error.status() {
                details.insert("httpStatus".to_owned(), json!(status.as_u16()));
            }

            if let Some(url) = error.url() {
                details.insert("endpoint".to_owned(), json!(safe_url_for_log(url.as_str())));
            }

            Value::Object(details)
        }
        DmApiError::Url(error) => json!({
            "failureKind": "api",
            "apiErrorKind": "url",
            "source": error.to_string(),
        }),
        DmApiError::Json(error) => json!({
            "failureKind": "api",
            "apiErrorKind": "json",
            "source": error.to_string(),
        }),
    }
}

fn download_error_support_message(error: &dm_download::DownloadError) -> String {
    match error {
        dm_download::DownloadError::Api(error) => {
            format!(
                "Download failed while calling DLsite: {}",
                dm_api_error_support_message(error)
            )
        }
        dm_download::DownloadError::Archive(error) => {
            format!("Archive extraction error: {error}")
        }
        dm_download::DownloadError::Io(error) => format!("Download I/O error: {error}"),
        other => format!("Download error: {other}"),
    }
}

fn download_error_support_details(error: &dm_download::DownloadError) -> Value {
    match error {
        dm_download::DownloadError::Cancelled => json!({
            "failureKind": "download",
            "downloadErrorKind": "cancelled",
        }),
        dm_download::DownloadError::InvalidDownloadResponse { reason } => json!({
            "failureKind": "download",
            "downloadErrorKind": "invalid_response",
            "reason": reason.to_string(),
        }),
        dm_download::DownloadError::FileNameUnknown { file_index } => json!({
            "failureKind": "download",
            "downloadErrorKind": "file_name_unknown",
            "fileIndex": file_index,
        }),
        dm_download::DownloadError::InvalidFileName { file_name } => json!({
            "failureKind": "download",
            "downloadErrorKind": "invalid_file_name",
            "fileName": file_name,
        }),
        dm_download::DownloadError::TargetAlreadyExists { path } => json!({
            "failureKind": "download",
            "downloadErrorKind": "target_already_exists",
            "path": path.to_string_lossy(),
        }),
        dm_download::DownloadError::PlanWorkMismatch {
            job_work_id,
            plan_work_id,
        } => json!({
            "failureKind": "download",
            "downloadErrorKind": "plan_work_mismatch",
            "jobWorkId": job_work_id.as_ref(),
            "planWorkId": plan_work_id.as_ref(),
        }),
        dm_download::DownloadError::IncompleteDownload { expected, actual } => json!({
            "failureKind": "download",
            "downloadErrorKind": "incomplete_download",
            "expectedBytes": expected,
            "actualBytes": actual,
        }),
        dm_download::DownloadError::SizeExceeded { expected, actual } => json!({
            "failureKind": "download",
            "downloadErrorKind": "size_exceeded",
            "expectedBytes": expected,
            "actualBytes": actual,
        }),
        dm_download::DownloadError::Stream(error) => json!({
            "failureKind": "download",
            "downloadErrorKind": "stream",
            "source": error,
        }),
        dm_download::DownloadError::Api(error) => {
            let mut details = value_object(dm_api_error_support_details(error));
            details.insert("downloadErrorKind".to_owned(), json!("api"));
            Value::Object(details)
        }
        dm_download::DownloadError::Archive(error) => json!({
            "failureKind": "download",
            "downloadErrorKind": "archive",
            "source": error.to_string(),
        }),
        dm_download::DownloadError::Io(error) => json!({
            "failureKind": "download",
            "downloadErrorKind": "io",
            "source": error.to_string(),
        }),
    }
}

fn value_object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => {
            let mut map = Map::new();
            map.insert("detail".to_owned(), other);
            map
        }
    }
}

fn safe_url_for_log(value: &str) -> String {
    let without_fragment = value.split_once('#').map_or(value, |(prefix, _)| prefix);
    let without_query = without_fragment
        .split_once('?')
        .map_or(without_fragment, |(prefix, _)| prefix);

    if without_query.contains("download.dlsite")
        || without_query.contains("/get/")
        || without_query.contains("/file/")
    {
        if let Some((origin, _)) = without_query.split_once("/get/") {
            return format!("{origin}/<download-url-redacted>");
        }

        if let Some((origin, _)) = without_query.split_once("/file/") {
            return format!("{origin}/<download-url-redacted>");
        }

        return "<download-url-redacted>".to_owned();
    }

    without_query.to_owned()
}

fn safe_body_snippet_for_log(snippet: &str) -> Option<String> {
    let lower = snippet.to_ascii_lowercase();
    if lower.contains("serial") || snippet.contains("\u{30b7}\u{30ea}\u{30a2}\u{30eb}") {
        return Some("[redacted because response may contain serial information]".to_owned());
    }

    let collapsed = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }

    let sanitized = redact_urls_in_text(&collapsed);

    Some(truncate_chars(&sanitized, 300))
}

fn redact_urls_in_text(value: &str) -> String {
    let mut output = String::new();
    let mut rest = value;

    loop {
        let Some((start, marker_len)) = earliest_url_marker(rest) else {
            output.push_str(rest);
            return output;
        };

        output.push_str(&rest[..start]);
        output.push_str("[redacted-url]");

        let url_start = start + marker_len;
        let url_end = rest[url_start..]
            .char_indices()
            .find_map(|(index, ch)| {
                (ch.is_whitespace() || matches!(ch, '"' | '\'' | '<' | '>' | ')' | ']' | '}'))
                    .then_some(url_start + index)
            })
            .unwrap_or(rest.len());

        rest = &rest[url_end..];
    }
}

fn earliest_url_marker(value: &str) -> Option<(usize, usize)> {
    [("https://", 8usize), ("http://", 7usize)]
        .into_iter()
        .filter_map(|(marker, len)| value.find(marker).map(|index| (index, len)))
        .min_by_key(|(index, _)| *index)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();

    if value.chars().count() > max_chars {
        output.push_str("...");
    }

    output
}

#[derive(Clone)]
pub struct Library {
    storage: Storage,
    credentials: Arc<dyn CredentialStore>,
    /// One lock per account, held across authentication only.
    ///
    /// Without it, jobs started together would each begin their own login and each raise its
    /// own two-factor prompt. Serializing lets the first login store a session the others can
    /// then reuse.
    login_locks: Arc<std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    work_locks: Arc<std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl Library {
    pub fn new(storage: Storage, credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            storage,
            credentials,
            login_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
            work_locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    fn work_lock(&self, work_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.work_locks
            .lock()
            .expect("work lock registry")
            .entry(work_id.to_owned())
            .or_default()
            .clone()
    }

    async fn recover_download(&self, work_id: &str, roots: &[&Path]) -> Result<()> {
        self.recover_download_inner(work_id, roots, false).await
    }

    async fn recover_download_inner(
        &self,
        work_id: &str,
        roots: &[&Path],
        remove_previous_staging: bool,
    ) -> Result<()> {
        let Some(record) = self.storage.download_finalization(work_id).await? else {
            return Ok(());
        };
        let installation = finalization::Installation { record };
        installation
            .validate(roots)
            .map_err(|error| installation.required(&error.support_message()))?;
        let result = if installation.record.committed {
            installation.cleanup().await
        } else {
            installation.rollback().await
        };
        result.map_err(|error| installation.required(&error.support_message()))?;
        if Path::new(&installation.record.staging_path).try_exists()? {
            let state = self.storage.work_download_state(work_id).await?;
            if state.status == WorkDownloadStatus::NotDownloaded {
                return Err(installation.required("missing download row for staging removal"));
            }
            if state.staging_path.as_deref() != Some(&installation.record.staging_path) {
                // Keep intent until the row durably tracks restored staging. Only deletion
                // may remove a different staging path before replacing its reference.
                if let Some(previous) = state.staging_path.as_deref().map(Path::new) {
                    if previous.try_exists()? {
                        let previous =
                            finalization::checked_child(&previous.canonicalize()?, roots)?;
                        let staging = Path::new(&installation.record.staging_path);
                        if previous != staging {
                            if !remove_previous_staging {
                                return Err(installation.required(&format!(
                                    "previous staging still exists at {}; deletion with root authority is required",
                                    previous.display()
                                )));
                            }
                            for path in [
                                Some(staging),
                                Some(Path::new(&installation.record.final_path)),
                                Some(Path::new(&installation.record.temporary_path)),
                                installation.record.old_path.as_deref().map(Path::new),
                            ]
                            .into_iter()
                            .flatten()
                            {
                                if previous.starts_with(path) || path.starts_with(&previous) {
                                    return Err(installation
                                        .required("previous staging overlaps recovery material"));
                                }
                            }
                            remove_existing_download_path(&previous, roots).await?;
                        }
                    }
                }
                self.save_download_staging(
                    work_id,
                    &state,
                    Path::new(&installation.record.staging_path),
                )
                .await?;
            }
        }
        self.storage
            .clear_download_finalization(work_id, &installation.record.operation_id)
            .await?;
        Ok(())
    }

    async fn save_download_staging(
        &self,
        work_id: &str,
        state: &WorkDownloadState,
        staging: &Path,
    ) -> Result<()> {
        let incomplete = || {
            LibraryError::RecoveryRequired(format!(
                "incomplete download row for {work_id}; retained staging: {}",
                staging.display()
            ))
        };
        self.storage
            .save_work_download(&WorkDownloadUpdate {
                work_id: work_id.to_owned(),
                status: state.status,
                local_path: state.local_path.clone(),
                staging_path: Some(staging.to_string_lossy().into_owned()),
                unpack_policy: state.unpack_policy.clone().ok_or_else(incomplete)?,
                bytes_received: state.bytes_received,
                bytes_total: state.bytes_total,
                error_code: state.error_code.clone(),
                error_message: state.error_message.clone(),
                started_at: state.started_at.clone(),
                completed_at: state.completed_at.clone(),
                updated_at: state.updated_at.clone().ok_or_else(incomplete)?,
            })
            .await?;
        Ok(())
    }

    pub async fn accounts(&self) -> Result<Vec<Account>> {
        Ok(self.storage.accounts().await?)
    }

    pub async fn save_account(&self, request: SaveAccountRequest) -> Result<Account> {
        let account_id = request
            .id
            .clone()
            .unwrap_or_else(|| format!("account-{}", Uuid::new_v4()));
        let lock = self.login_lock(&account_id);
        let _guard = lock.lock().await;
        let existing_account = match request.id.as_deref() {
            Some(account_id) => match self.find_account(account_id).await {
                Ok(account) => Some(account),
                Err(LibraryError::AccountNotFound(_)) => None,
                Err(error) => return Err(error),
            },
            None => None,
        };
        let credential_ref = self.save_account_credential(
            &account_id,
            request.password.as_deref(),
            request.remember_password,
            existing_account.as_ref().and_then(|account| {
                account
                    .credential_ref
                    .as_deref()
                    .and_then(|value| CredentialRef::new(value.to_owned()).ok())
            }),
        )?;

        let account = AccountUpsert {
            id: account_id.clone(),
            label: request.label,
            login_name: request.login_name,
            credential_ref: credential_ref.map(|value| value.to_string()),
            enabled: existing_account
                .as_ref()
                .map_or(request.enabled, |account| account.enabled),
        };

        // Changed sign-in details invalidate whatever session was stored for this account.
        let identity_changed = request.password.is_some()
            || existing_account
                .as_ref()
                .is_none_or(|existing| existing.login_name != account.login_name);

        if identity_changed || !request.remember_password {
            self.forget_account_session(&account_id)?;
        }

        self.storage.save_account(&account).await?;
        self.find_account(&account_id).await
    }

    pub async fn set_account_enabled(&self, account_id: &str, enabled: bool) -> Result<()> {
        let lock = self.login_lock(account_id);
        let _guard = lock.lock().await;
        Ok(self
            .storage
            .set_account_enabled(account_id, enabled)
            .await?)
    }

    pub async fn remove_account(&self, account_id: &str) -> Result<AccountRemovalReport> {
        let lock = self.login_lock(account_id);
        let _guard = lock.lock().await;
        let account = self.find_account(account_id).await?;
        let credential_ref = account
            .credential_ref
            .as_deref()
            .map(|value| CredentialRef::new(value.to_owned()))
            .transpose()?;
        let credential_deleted = if let Some(credential_ref) = credential_ref {
            self.credentials.delete_password(&credential_ref)?;
            true
        } else {
            false
        };

        self.forget_account_session(account_id)?;
        self.storage.delete_account(account_id).await?;

        Ok(AccountRemovalReport {
            account_id: account.id,
            label: account.label,
            credential_deleted,
        })
    }

    pub async fn list_products(&self, query: &ProductListQuery) -> Result<ProductListPage> {
        Ok(self.storage.list_products(query).await?)
    }

    pub async fn product_filter_facets(
        &self,
        query: &ProductListQuery,
    ) -> Result<ProductFilterFacets> {
        Ok(self.storage.product_filter_facets(query).await?)
    }

    pub async fn product_detail(&self, work_id: &str) -> Result<ProductDetail> {
        Ok(self.storage.product_detail(work_id).await?)
    }

    pub async fn set_product_custom_tags(
        &self,
        work_id: &str,
        tags: &[String],
    ) -> Result<Vec<ProductCustomTag>> {
        Ok(self.storage.set_work_custom_tags(work_id, tags).await?)
    }

    pub fn account_has_saved_password(&self, account: &Account) -> Result<bool> {
        let Some(credential_ref) = account.credential_ref.as_deref() else {
            return Ok(false);
        };
        let credential_ref = CredentialRef::new(credential_ref.to_owned())?;

        Ok(self.credentials.load_password(&credential_ref)?.is_some())
    }

    pub async fn download_work_with_source<S>(
        &self,
        request: WorkDownloadRequest<'_>,
        source: &S,
    ) -> Result<WorkDownloadReport>
    where
        S: WorkDownloadSource + Sync,
    {
        let lock = self.work_lock(request.work_id);
        let _guard = acquire_login_lock(&lock, request.cancellation_token).await?;
        self.recover_download(
            request.work_id,
            &[request.library_root, request.download_root],
        )
        .await?;
        let previous = self.storage.work_download_state(request.work_id).await?;
        let preserve_previous = previous.status == WorkDownloadStatus::Downloaded;
        let (staging_dir, final_dir) =
            finalization::paths(request.work_id, request.library_root, request.download_root)?;
        if let Some(old) = previous
            .local_path
            .as_deref()
            .map(Path::new)
            .filter(|p| p.exists())
        {
            if std::fs::symlink_metadata(old)?.file_type().is_symlink() {
                return Err(LibraryError::DownloadPathOutsideRoots(old.to_owned()));
            }
            let old = old.canonicalize()?;
            finalization::checked_child(&old, &[request.library_root, request.download_root])?;
            if old.starts_with(&staging_dir) || staging_dir.starts_with(&old) {
                return Err(LibraryError::DownloadPathOutsideRoots(old));
            }
        }
        let account = self
            .storage
            .download_account_for_work(request.work_id, request.account_id)
            .await
            .map_err(|error| match error {
                StorageError::NotFound { .. } => {
                    LibraryError::DownloadAccountNotFound(request.work_id.to_owned())
                }
                error => LibraryError::Storage(error),
            })?;
        let started_at = now_string();
        let work_id = WorkId::from(request.work_id.to_owned());
        let unpack_policy = request.unpack_policy;
        let result = self
            .download_work_inner(
                &account,
                &work_id,
                &staging_dir,
                &final_dir,
                &started_at,
                request,
                source,
            )
            .await;

        if let Err(error) = &result {
            if preserve_previous {
                return result;
            }
            let completed_at = now_string();
            let status = if error.is_cancelled() {
                WorkDownloadStatus::Cancelled
            } else {
                WorkDownloadStatus::Failed
            };
            let _ = self
                .storage
                .save_work_download(&WorkDownloadUpdate {
                    work_id: request.work_id.to_owned(),
                    status,
                    local_path: Some(final_dir.to_string_lossy().into_owned()),
                    staging_path: Some(staging_dir.to_string_lossy().into_owned()),
                    unpack_policy: unpack_policy_storage_value(unpack_policy).to_owned(),
                    bytes_received: 0,
                    bytes_total: None,
                    error_code: Some(error.failure_code().to_owned()),
                    error_message: Some(error.support_message()),
                    started_at: Some(started_at),
                    completed_at: Some(completed_at.clone()),
                    updated_at: completed_at,
                })
                .await;
        }

        result
    }

    pub async fn download_products_with_source<S>(
        &self,
        request: BulkWorkDownloadRequest<'_>,
        source: &S,
    ) -> Result<BulkWorkDownloadReport>
    where
        S: WorkDownloadSource + Sync,
    {
        request.check_cancelled()?;
        request.emit(BulkWorkDownloadProgress::Selecting);

        let selection = self
            .bulk_download_selection(
                &request.query,
                request.skip_downloaded,
                request.work_ids.as_deref(),
                request.cancellation_token,
            )
            .await?;

        let requested_count = selection.items.len();
        request.emit(BulkWorkDownloadProgress::Selected {
            total_count: selection.total_count,
            requested_count,
            skipped_downloaded_count: selection.skipped_downloaded_count,
        });

        let mut report = BulkWorkDownloadReport {
            total_count: selection.total_count,
            requested_count,
            skipped_downloaded_count: selection.skipped_downloaded_count,
            succeeded_count: 0,
            failed_count: 0,
            succeeded_works: Vec::new(),
            failed_works: Vec::new(),
        };

        for (index, item) in selection.items.into_iter().enumerate() {
            if let Err(cause) = request.check_cancelled() {
                return Err(LibraryError::BulkInterrupted {
                    cause: Box::new(cause),
                    report: Box::new(report),
                    interrupted_work: None,
                });
            }
            let current = index + 1;
            let work_id = item.work_id;

            request.emit(BulkWorkDownloadProgress::WorkStarted {
                work_id: work_id.clone(),
                current,
                total: requested_count,
            });

            let work_result = self
                .download_work_with_source(
                    WorkDownloadRequest {
                        work_id: &work_id,
                        account_id: request.query.account_id.as_deref(),
                        password: None,
                        library_root: request.library_root,
                        download_root: request.download_root,
                        unpack_policy: request.unpack_policy,
                        replace_existing: false,
                        cancellation_token: request.cancellation_token,
                        progress_sink: None,
                        two_factor_prompt: request.two_factor_prompt,
                    },
                    source,
                )
                .await;

            match work_result {
                Ok(report_item) => {
                    report.succeeded_count += 1;
                    report.succeeded_works.push(BulkWorkDownloadSuccess {
                        work_id: report_item.work_id.clone(),
                        local_path: report_item.local_path,
                        file_count: report_item.file_count,
                        archive_extracted: report_item.archive_extracted,
                        warnings: report_item.warnings,
                    });
                    request.emit(BulkWorkDownloadProgress::WorkCompleted {
                        work_id,
                        current,
                        total: requested_count,
                    });
                }
                Err(error) if error.is_cancelled() => {
                    return Err(LibraryError::BulkInterrupted {
                        cause: Box::new(error),
                        report: Box::new(report),
                        interrupted_work: Some(work_id),
                    });
                }
                Err(error) => {
                    let error_code = error.failure_code().to_owned();
                    let error_message = error.support_message();

                    report.failed_count += 1;
                    report.failed_works.push(BulkWorkDownloadFailure {
                        work_id: work_id.clone(),
                        error_code: error_code.clone(),
                        error_message: error_message.clone(),
                    });
                    request.emit(BulkWorkDownloadProgress::WorkFailed {
                        work_id,
                        current,
                        total: requested_count,
                        error_code,
                        error_message,
                    });
                }
            }
        }

        request.emit(BulkWorkDownloadProgress::Completed {
            report: report.clone(),
        });

        Ok(report)
    }

    pub async fn preview_download_products_with_source<S>(
        &self,
        request: BulkWorkDownloadPreviewRequest<'_>,
        _source: &S,
    ) -> Result<BulkWorkDownloadPreview>
    where
        S: WorkDownloadSource + Sync,
    {
        request.check_cancelled()?;
        request.emit(BulkWorkDownloadPreviewProgress::Selecting);
        let selection = self
            .bulk_download_selection(
                &request.query,
                request.skip_downloaded,
                request.work_ids.as_deref(),
                request.cancellation_token,
            )
            .await?;
        let requested_count = selection.items.len();
        request.emit(BulkWorkDownloadPreviewProgress::Selected {
            total_count: selection.total_count,
            requested_count,
            skipped_downloaded_count: selection.skipped_downloaded_count,
        });
        let mut preview = BulkWorkDownloadPreview {
            total_count: selection.total_count,
            requested_count,
            skipped_downloaded_count: selection.skipped_downloaded_count,
            planned_count: 0,
            failed_count: 0,
            known_expected_bytes: 0,
            total_expected_bytes: Some(0),
            unknown_size_count: 0,
            works: Vec::new(),
            failed_works: Vec::new(),
        };
        for (index, item) in selection.items.into_iter().enumerate() {
            request.check_cancelled()?;
            let current = index + 1;
            let work_id = item.work_id;
            request.emit(BulkWorkDownloadPreviewProgress::WorkStarted {
                work_id: work_id.clone(),
                current,
                total: requested_count,
            });

            if let Some(content_size_bytes) = item.content_size_bytes {
                let work =
                    self.preview_work_download_from_cached_size(&work_id, content_size_bytes);

                preview.planned_count += 1;
                preview.known_expected_bytes = preview
                    .known_expected_bytes
                    .saturating_add(work.known_expected_bytes);
                preview.total_expected_bytes = preview
                    .total_expected_bytes
                    .map(|total| total.saturating_add(content_size_bytes));
                preview.works.push(work);
                request.emit(BulkWorkDownloadPreviewProgress::WorkPlanned {
                    work_id,
                    current,
                    total: requested_count,
                    known_expected_bytes: content_size_bytes,
                    total_expected_bytes: Some(content_size_bytes),
                    unknown_size_count: 0,
                });
                continue;
            }

            let work = self.preview_work_download_from_unknown_cached_size(&work_id);
            preview.planned_count += 1;
            preview.unknown_size_count = preview.unknown_size_count.saturating_add(1);
            preview.total_expected_bytes = None;
            preview.works.push(work);
            request.emit(BulkWorkDownloadPreviewProgress::WorkPlanned {
                work_id,
                current,
                total: requested_count,
                known_expected_bytes: 0,
                total_expected_bytes: None,
                unknown_size_count: 1,
            });
        }

        request.emit(BulkWorkDownloadPreviewProgress::Completed {
            planned_count: preview.planned_count,
            failed_count: preview.failed_count,
            known_expected_bytes: preview.known_expected_bytes,
            total_expected_bytes: preview.total_expected_bytes,
            unknown_size_count: preview.unknown_size_count,
        });

        Ok(preview)
    }

    pub async fn remove_work_download(
        &self,
        request: WorkDownloadRemovalRequest<'_>,
    ) -> Result<WorkDownloadState> {
        let lock = self.work_lock(request.work_id);
        let _guard = lock.lock().await;
        self.recover_download_inner(
            request.work_id,
            &[request.library_root, request.download_root],
            true,
        )
        .await?;
        let state = self.storage.work_download_state(request.work_id).await?;
        let allowed_roots = [request.library_root, request.download_root];

        remove_download_path_from_state(state.local_path.as_deref(), &allowed_roots).await?;
        remove_download_path_from_state(state.staging_path.as_deref(), &allowed_roots).await?;
        self.storage.delete_work_download(request.work_id).await?;
        Ok(self.storage.work_download_state(request.work_id).await?)
    }

    pub async fn mark_work_downloaded(
        &self,
        request: WorkDownloadMarkRequest<'_>,
    ) -> Result<WorkDownloadState> {
        let lock = self.work_lock(request.work_id);
        let _guard = lock.lock().await;
        self.recover_download(request.work_id, &[request.library_root])
            .await?;
        let canonical_path = canonicalize_existing_directory(request.local_path)?;
        let canonical_root = request.library_root.canonicalize()?;

        if !path_is_download_child_of_any_root(&canonical_path, &[canonical_root]) {
            return Err(LibraryError::DownloadPathOutsideRoots(canonical_path));
        }

        let completed_at = now_string();
        self.storage
            .save_work_download(&WorkDownloadUpdate {
                work_id: request.work_id.to_owned(),
                status: WorkDownloadStatus::Downloaded,
                local_path: Some(canonical_path.to_string_lossy().into_owned()),
                staging_path: None,
                unpack_policy: "manual".to_owned(),
                bytes_received: 0,
                bytes_total: None,
                error_code: None,
                error_message: None,
                started_at: Some(completed_at.clone()),
                completed_at: Some(completed_at.clone()),
                updated_at: completed_at,
            })
            .await?;

        Ok(self.storage.work_download_state(request.work_id).await?)
    }

    pub async fn import_local_work_downloads(
        &self,
        request: LocalWorkImportRequest<'_>,
    ) -> Result<LocalWorkImportReport> {
        self.import_local_work_downloads_internal(request, None)
            .await
    }

    pub async fn import_local_work_downloads_with_metadata_source<S>(
        &self,
        request: LocalWorkImportRequest<'_>,
        metadata_source: &S,
    ) -> Result<LocalWorkImportReport>
    where
        S: LocalWorkMetadataSource,
    {
        self.import_local_work_downloads_internal(
            request,
            Some(metadata_source as &dyn LocalWorkMetadataSource),
        )
        .await
    }

    async fn import_local_work_downloads_internal(
        &self,
        request: LocalWorkImportRequest<'_>,
        metadata_source: Option<&dyn LocalWorkMetadataSource>,
    ) -> Result<LocalWorkImportReport> {
        let library_root = canonicalize_existing_directory(request.library_root)?;
        let mut entries = tokio::fs::read_dir(&library_root).await?;
        let scanned_at = now_string();
        let mut imports = Vec::new();
        let mut imported_works = Vec::new();
        let mut metadata_candidate_ids = BTreeSet::<WorkId>::new();
        let mut scanned_directories = 0;
        let mut skipped_no_id = 0;
        let mut skipped_ambiguous = 0;
        let mut skipped_non_utf8 = 0;
        let mut skipped_existing = 0;
        let mut recovery_errors = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;

            if !file_type.is_dir() {
                continue;
            }

            scanned_directories += 1;

            let folder_name = match entry.file_name().into_string() {
                Ok(folder_name) => folder_name,
                Err(_) => {
                    skipped_non_utf8 += 1;
                    continue;
                }
            };
            let detected = detect_work_ids_in_text(&folder_name);
            if folder_name.starts_with(finalization::PREFIX) {
                continue;
            }

            if detected.is_empty() {
                skipped_no_id += 1;
                continue;
            }

            if detected.len() > 1 {
                skipped_ambiguous += 1;
                continue;
            }

            let work_id = detected[0].clone();
            let work_id_string = work_id.as_ref().to_owned();
            let lock = self.work_lock(&work_id_string);
            let _guard = lock.lock().await;
            if let Err(error) = self
                .recover_download(&work_id_string, &[&library_root])
                .await
            {
                recovery_errors.push(error.support_message());
                continue;
            }
            if self
                .storage
                .work_download_state(&work_id_string)
                .await?
                .status
                != WorkDownloadStatus::NotDownloaded
            {
                skipped_existing += 1;
                if self
                    .storage
                    .work_needs_local_metadata_hydration(&work_id_string)
                    .await?
                {
                    metadata_candidate_ids.insert(work_id);
                }
                continue;
            }

            let local_path = entry.path().canonicalize()?;
            let work = cached_work_from_local_folder(&work_id_string, &folder_name, &scanned_at)?;
            let download = WorkDownloadUpdate {
                work_id: work_id_string.clone(),
                status: WorkDownloadStatus::Downloaded,
                local_path: Some(local_path.to_string_lossy().into_owned()),
                staging_path: None,
                unpack_policy: "manual".to_owned(),
                bytes_received: 0,
                bytes_total: None,
                error_code: None,
                error_message: None,
                started_at: Some(scanned_at.clone()),
                completed_at: Some(scanned_at.clone()),
                updated_at: scanned_at.clone(),
            };

            metadata_candidate_ids.insert(work_id);
            imports.push(LocalWorkDownloadImport { work, download });
            imported_works.push(LocalWorkImportItem {
                work_id: work_id_string,
                local_path,
            });
        }

        let metadata_candidate_count = if metadata_source.is_some() {
            metadata_candidate_ids.len()
        } else {
            0
        };
        let mut metadata_by_id = BTreeMap::<String, CachedWork>::new();
        let mut metadata_error = None;

        if let Some(source) = metadata_source.filter(|_| !metadata_candidate_ids.is_empty()) {
            let ids = metadata_candidate_ids.iter().cloned().collect::<Vec<_>>();

            match source.works(&ids).await {
                Ok(works) => {
                    for work in works {
                        if !metadata_candidate_ids.contains(&work.id) {
                            continue;
                        }

                        let cached = cached_work_from_public_api(work, &scanned_at)?;
                        metadata_by_id.insert(cached.work_id.clone(), cached);
                    }
                }
                Err(error) => {
                    metadata_error = Some(error.support_message());
                }
            }
        }

        let metadata_works = metadata_by_id.values().cloned().collect::<Vec<_>>();

        imported_works.clear();
        for import in imports {
            let work_id = &import.work.work_id;
            let lock = self.work_lock(work_id);
            let _guard = lock.lock().await;
            if let Err(error) = self.recover_download(work_id, &[&library_root]).await {
                recovery_errors.push(error.support_message());
                continue;
            }
            if self.storage.work_download_state(work_id).await?.status
                != WorkDownloadStatus::NotDownloaded
            {
                skipped_existing += 1;
                continue;
            }
            let local_path =
                PathBuf::from(import.download.local_path.as_ref().expect("import path"));
            if !local_path.is_dir() {
                continue;
            }
            self.storage
                .import_local_work_downloads_with_metadata(std::slice::from_ref(&import), &[])
                .await?;
            imported_works.push(LocalWorkImportItem {
                work_id: work_id.clone(),
                local_path,
            });
        }
        self.storage
            .import_local_work_downloads_with_metadata(&[], &metadata_works)
            .await?;

        Ok(LocalWorkImportReport {
            scanned_directories,
            imported_count: imported_works.len(),
            skipped_no_id,
            skipped_ambiguous,
            skipped_non_utf8,
            skipped_existing,
            metadata_candidate_count,
            metadata_updated_count: metadata_by_id.len(),
            metadata_missing_count: metadata_candidate_count.saturating_sub(metadata_by_id.len()),
            metadata_error,
            recovery_errors,
            imported_works,
        })
    }

    pub async fn sync_account_with_source<S>(
        &self,
        request: AccountSyncRequest<'_>,
        source: &S,
    ) -> Result<AccountSyncReport>
    where
        S: AccountSyncSource + Sync,
    {
        let account = self.find_account(request.account_id).await?;

        if !account.enabled {
            return Err(LibraryError::AccountDisabled(account.id));
        }

        let sync_run_id = format!("sync-{}", Uuid::new_v4());
        let started_at = now_string();
        let result = self
            .sync_account_inner(&account, &sync_run_id, &started_at, request, source)
            .await;

        if let Err(error) = &result {
            let completed_at = now_string();
            if error.is_cancelled() {
                let _ = self
                    .storage
                    .record_sync_cancellation(&SyncCancellation {
                        sync_run_id,
                        account_id: account.id,
                        started_at,
                        completed_at,
                    })
                    .await;
            } else {
                let _ = self
                    .storage
                    .record_sync_failure(&SyncFailure {
                        sync_run_id,
                        account_id: account.id,
                        started_at,
                        completed_at,
                        error_code: Some(error.failure_code().to_owned()),
                        error_message: Some(error.support_message()),
                    })
                    .await;
            }
        }

        result
    }

    async fn bulk_download_selection(
        &self,
        query: &ProductListQuery,
        skip_downloaded: bool,
        work_ids: Option<&[String]>,
        cancellation_token: Option<&CancellationToken>,
    ) -> Result<BulkWorkDownloadSelection> {
        if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
            return Err(LibraryError::Cancelled);
        }

        let allowed_work_ids =
            work_ids.map(|work_ids| work_ids.iter().map(String::as_str).collect::<BTreeSet<_>>());
        let total_count = self.storage.list_products(query).await?.total_count;
        let mut query = query.clone();
        query.limit = BULK_DOWNLOAD_PAGE_LIMIT;
        query.offset = 0;

        let mut work_ids = Vec::new();
        let mut skipped_downloaded_count = 0usize;

        loop {
            if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
                return Err(LibraryError::Cancelled);
            }

            let page = self.storage.list_products(&query).await?;
            let page_len = page.products.len();

            for product in page.products {
                if skip_downloaded && product.download.status == WorkDownloadStatus::Downloaded {
                    skipped_downloaded_count += 1;
                    continue;
                }

                if let Some(allowed_work_ids) = &allowed_work_ids {
                    if !allowed_work_ids.contains(product.work_id.as_str()) {
                        continue;
                    }
                }

                work_ids.push(BulkWorkDownloadSelectionItem {
                    work_id: product.work_id,
                    content_size_bytes: product.content_size_bytes,
                });
            }

            if page_len == 0 || page_len < query.limit as usize {
                break;
            }

            query.offset = query.offset.saturating_add(page_len as u32);
        }

        Ok(BulkWorkDownloadSelection {
            total_count,
            skipped_downloaded_count,
            items: work_ids,
        })
    }

    fn preview_work_download_from_cached_size(
        &self,
        work_id: &str,
        content_size_bytes: u64,
    ) -> BulkWorkDownloadPreviewWork {
        BulkWorkDownloadPreviewWork {
            work_id: work_id.to_owned(),
            file_count: 1,
            known_expected_bytes: content_size_bytes,
            total_expected_bytes: Some(content_size_bytes),
            unknown_size_count: 0,
            files: vec![BulkWorkDownloadPreviewFile {
                file_index: 0,
                file_name: "DLsite content".to_owned(),
                expected_size: Some(content_size_bytes),
            }],
        }
    }

    fn preview_work_download_from_unknown_cached_size(
        &self,
        work_id: &str,
    ) -> BulkWorkDownloadPreviewWork {
        BulkWorkDownloadPreviewWork {
            work_id: work_id.to_owned(),
            file_count: 0,
            known_expected_bytes: 0,
            total_expected_bytes: None,
            unknown_size_count: 1,
            files: Vec::new(),
        }
    }

    async fn sync_account_inner<S>(
        &self,
        account: &Account,
        sync_run_id: &str,
        started_at: &str,
        request: AccountSyncRequest<'_>,
        source: &S,
    ) -> Result<AccountSyncReport>
    where
        S: AccountSyncSource + Sync,
    {
        request.check_cancelled()?;
        request.emit(SyncProgress::LoggingIn);

        self.authenticate_account(
            account,
            request.password,
            request.two_factor_prompt,
            request.cancellation_token,
            source,
        )
        .await?;

        request.check_cancelled()?;
        request.emit(SyncProgress::LoadingCount);
        let count = source.content_count().await?;

        request.check_cancelled()?;
        request.emit(SyncProgress::LoadingPurchases);
        let purchases = source.purchases().await?;
        let purchased_ids = purchases
            .iter()
            .map(|purchase| purchase.id.clone())
            .collect::<Vec<_>>();

        request.check_cancelled()?;
        request.emit(SyncProgress::LoadingWorks {
            work_count: purchased_ids.len(),
        });
        let works = source.works(&purchased_ids).await?;

        let completed_at = now_string();
        let storage_sync = build_storage_sync(
            &account.id,
            sync_run_id,
            started_at,
            &completed_at,
            purchases,
            works,
        )?;

        request.check_cancelled()?;
        request.emit(SyncProgress::Committing {
            work_count: storage_sync.commit.works.len(),
        });
        self.storage
            .commit_account_sync(&storage_sync.commit)
            .await?;

        let report = AccountSyncReport {
            account_id: account.id.clone(),
            sync_run_id: sync_run_id.to_owned(),
            purchased_count: storage_sync.commit.account_works.len(),
            cached_work_count: storage_sync.commit.works.len(),
            missing_detail_count: storage_sync.missing_detail_count,
            page_limit: count.page_limit,
            concurrency: count.concurrency,
        };

        request.emit(SyncProgress::Completed {
            sync_run_id: sync_run_id.to_owned(),
            cached_work_count: report.cached_work_count,
        });

        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_work_inner<S>(
        &self,
        account: &Account,
        work_id: &WorkId,
        staging_dir: &Path,
        final_dir: &Path,
        started_at: &str,
        request: WorkDownloadRequest<'_>,
        source: &S,
    ) -> Result<WorkDownloadReport>
    where
        S: WorkDownloadSource + Sync,
    {
        request.check_cancelled()?;
        request.emit(WorkDownloadProgress::LoggingIn);

        self.authenticate_account(
            account,
            request.password,
            request.two_factor_prompt,
            request.cancellation_token,
            source,
        )
        .await?;

        request.check_cancelled()?;
        request.emit(WorkDownloadProgress::ResolvingPlan);
        let plan = source.download_plan(work_id).await?;

        request.check_cancelled()?;
        let previous = self.storage.work_download_state(request.work_id).await?;
        if previous.status != WorkDownloadStatus::Downloaded {
            self.storage
                .save_work_download(&WorkDownloadUpdate {
                    work_id: request.work_id.to_owned(),
                    status: WorkDownloadStatus::Downloading,
                    local_path: Some(final_dir.to_string_lossy().into_owned()),
                    staging_path: Some(staging_dir.to_string_lossy().into_owned()),
                    unpack_policy: unpack_policy_storage_value(request.unpack_policy).to_owned(),
                    bytes_received: 0,
                    bytes_total: None,
                    error_code: None,
                    error_message: None,
                    started_at: Some(started_at.to_owned()),
                    completed_at: None,
                    updated_at: now_string(),
                })
                .await?;
        } else if previous.staging_path.as_deref() != staging_dir.to_str() {
            // Track operational staging before any transfer writes, without changing
            // the usable content's status, paths, bytes, policy, errors, or timestamps.
            if let Some(path) = previous.staging_path.as_deref().map(Path::new) {
                if path.try_exists()? && path.canonicalize()? != staging_dir {
                    return Err(LibraryError::RecoveryRequired(format!(
                        "previous staging still exists at {}; retain it before starting staging at {}",
                        path.display(), staging_dir.display()
                    )));
                }
            }
            self.save_download_staging(request.work_id, &previous, staging_dir)
                .await?;
        }

        if request.replace_existing {
            remove_existing_download_path(staging_dir, &[request.download_root]).await?;
        }

        let download_cancellation = dm_download::CancellationToken::new();
        let _download_cancellation_forwarder =
            request.forward_download_cancellation(&download_cancellation);
        let job = DownloadJobRequest {
            work_id: work_id.clone(),
            target_root: request.download_root.to_path_buf(),
            unpack_policy: request.unpack_policy,
        };
        let downloaded = source
            .download_files(&job, &plan, &download_cancellation, &mut |progress| {
                if request.is_cancelled() {
                    download_cancellation.cancel();
                }

                request.emit(WorkDownloadProgress::Download(progress));
            })
            .await?;

        request.check_cancelled()?;
        request.emit(WorkDownloadProgress::Finalizing);
        write_serial_information_file(
            staging_dir,
            work_id.as_ref(),
            &plan.serial_numbers,
            &now_string(),
        )
        .await?;

        let completed_at = now_string();
        let bytes_received = downloaded
            .files
            .iter()
            .map(|file| file.bytes_written)
            .sum::<u64>();

        let update = WorkDownloadUpdate {
            work_id: request.work_id.to_owned(),
            status: WorkDownloadStatus::Downloaded,
            local_path: Some(final_dir.to_string_lossy().into_owned()),
            staging_path: Some(staging_dir.to_string_lossy().into_owned()),
            unpack_policy: unpack_policy_storage_value(request.unpack_policy).to_owned(),
            bytes_received,
            bytes_total: Some(bytes_received),
            error_code: None,
            error_message: None,
            started_at: Some(started_at.to_owned()),
            completed_at: Some(completed_at.clone()),
            updated_at: completed_at,
        };
        let old_path = previous
            .local_path
            .as_deref()
            .map(Path::new)
            .filter(|p| p.is_dir())
            .map(std::fs::canonicalize)
            .transpose()?;
        let installation = finalization::Installation::new(
            request.work_id,
            staging_dir,
            final_dir,
            old_path.as_deref(),
            request.replace_existing,
        )?;
        installation.validate(&[request.library_root, request.download_root])?;
        self.storage
            .begin_download_finalization(&installation.record)
            .await?;
        let result = async {
            installation.prepare().await?;
            request.check_cancelled()?;
            // Once swapping starts, finish installation/commit or rollback before cancellation.
            installation.install().await?;
            self.storage
                .commit_download_finalization(&installation.record.operation_id, &update)
                .await?;
            Ok::<(), LibraryError>(())
        }
        .await;
        if let Err(error) = result {
            // A commit error may have an uncertain outcome. Inspect the durable marker
            // before deciding whether it is safe to undo filesystem installation.
            match self.storage.download_finalization(request.work_id).await {
                Ok(Some(record)) if record.committed => {}
                Ok(Some(_)) => {
                    if let Err(recovery) = self
                        .recover_download(
                            request.work_id,
                            &[request.library_root, request.download_root],
                        )
                        .await
                    {
                        return Err(installation.required(&format!(
                            "{}; rollback: {}",
                            error.support_message(),
                            recovery.support_message()
                        )));
                    }
                    if Path::new(&installation.record.temporary_path).exists() {
                        return Err(LibraryError::RetainedDownload {
                            source: Box::new(error),
                            retained_path: PathBuf::from(&installation.record.temporary_path),
                        });
                    }
                    return Err(error);
                }
                _ => {
                    return Err(installation.required(&format!(
                        "{}; commit outcome unavailable",
                        error.support_message()
                    )))
                }
            }
        }
        let mut warnings = Vec::new();
        if let Err(error) = self
            .recover_download(
                request.work_id,
                &[request.library_root, request.download_root],
            )
            .await
        {
            request.emit(WorkDownloadProgress::Warning(error.support_message()));
            warnings.push(error.support_message());
        }

        request.emit(WorkDownloadProgress::Completed);

        Ok(WorkDownloadReport {
            work_id: request.work_id.to_owned(),
            account_id: account.id.clone(),
            local_path: final_dir.to_path_buf(),
            file_count: downloaded.files.len(),
            archive_extracted: downloaded.archive_extraction.is_some(),
            download_state: update.into(),
            warnings,
        })
    }

    async fn find_account(&self, account_id: &str) -> Result<Account> {
        self.storage
            .accounts()
            .await?
            .into_iter()
            .find(|account| account.id == account_id)
            .ok_or_else(|| LibraryError::AccountNotFound(account_id.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAccountRequest {
    pub id: Option<String>,
    pub label: String,
    pub login_name: Option<String>,
    pub password: Option<String>,
    pub remember_password: bool,
    /// Initial state for new accounts. Existing accounts use `set_account_enabled`.
    pub enabled: bool,
}

impl SaveAccountRequest {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: None,
            label: label.into(),
            login_name: None,
            password: None,
            remember_password: false,
            enabled: true,
        }
    }
}

#[derive(Clone, Copy)]
pub struct AccountSyncRequest<'a> {
    pub account_id: &'a str,
    pub password: Option<&'a str>,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub progress_sink: Option<&'a dyn SyncProgressSink>,
    pub two_factor_prompt: Option<&'a dyn TwoFactorPrompt>,
}

impl<'a> AccountSyncRequest<'a> {
    pub fn new(account_id: &'a str) -> Self {
        Self {
            account_id,
            password: None,
            cancellation_token: None,
            progress_sink: None,
            two_factor_prompt: None,
        }
    }

    fn check_cancelled(&self) -> Result<()> {
        if self
            .cancellation_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(LibraryError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn emit(&self, progress: SyncProgress) {
        if let Some(sink) = self.progress_sink {
            sink.emit(progress);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSyncReport {
    pub account_id: String,
    pub sync_run_id: String,
    pub purchased_count: usize,
    pub cached_work_count: usize,
    pub missing_detail_count: usize,
    pub page_limit: Option<usize>,
    pub concurrency: Option<usize>,
}

#[derive(Clone, Copy)]
pub struct WorkDownloadRequest<'a> {
    pub work_id: &'a str,
    pub account_id: Option<&'a str>,
    pub password: Option<&'a str>,
    pub library_root: &'a Path,
    pub download_root: &'a Path,
    pub unpack_policy: UnpackPolicy,
    pub replace_existing: bool,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub progress_sink: Option<&'a dyn WorkDownloadProgressSink>,
    pub two_factor_prompt: Option<&'a dyn TwoFactorPrompt>,
}

impl<'a> WorkDownloadRequest<'a> {
    pub fn new(work_id: &'a str, library_root: &'a Path, download_root: &'a Path) -> Self {
        Self {
            work_id,
            account_id: None,
            password: None,
            library_root,
            download_root,
            unpack_policy: UnpackPolicy::UnpackWhenRecognized,
            replace_existing: false,
            cancellation_token: None,
            progress_sink: None,
            two_factor_prompt: None,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .is_some_and(CancellationToken::is_cancelled)
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(LibraryError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn emit(&self, progress: WorkDownloadProgress) {
        if let Some(sink) = self.progress_sink {
            sink.emit(progress);
        }
    }

    fn forward_download_cancellation(
        &self,
        download_cancellation: &dm_download::CancellationToken,
    ) -> Option<DownloadCancellationForwarder> {
        let job_cancellation = self.cancellation_token?.clone();
        let download_cancellation = download_cancellation.clone();

        if job_cancellation.is_cancelled() {
            download_cancellation.cancel();
        }

        let handle = tokio::spawn(async move {
            while !job_cancellation.is_cancelled() {
                tokio::time::sleep(DOWNLOAD_CANCELLATION_POLL_INTERVAL).await;
            }

            download_cancellation.cancel();
        });

        Some(DownloadCancellationForwarder { handle })
    }
}

struct DownloadCancellationForwarder {
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for DownloadCancellationForwarder {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub struct WorkDownloadRemovalRequest<'a> {
    pub work_id: &'a str,
    pub library_root: &'a Path,
    pub download_root: &'a Path,
}

impl<'a> WorkDownloadRemovalRequest<'a> {
    pub fn new(work_id: &'a str, library_root: &'a Path, download_root: &'a Path) -> Self {
        Self {
            work_id,
            library_root,
            download_root,
        }
    }
}

pub struct WorkDownloadMarkRequest<'a> {
    pub work_id: &'a str,
    pub library_root: &'a Path,
    pub local_path: &'a Path,
}

impl<'a> WorkDownloadMarkRequest<'a> {
    pub fn new(work_id: &'a str, library_root: &'a Path, local_path: &'a Path) -> Self {
        Self {
            work_id,
            library_root,
            local_path,
        }
    }
}

pub struct LocalWorkImportRequest<'a> {
    pub library_root: &'a Path,
}

impl<'a> LocalWorkImportRequest<'a> {
    pub fn new(library_root: &'a Path) -> Self {
        Self { library_root }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorkImportReport {
    pub recovery_errors: Vec<String>,
    pub scanned_directories: usize,
    pub imported_count: usize,
    pub skipped_no_id: usize,
    pub skipped_ambiguous: usize,
    pub skipped_non_utf8: usize,
    pub skipped_existing: usize,
    pub metadata_candidate_count: usize,
    pub metadata_updated_count: usize,
    pub metadata_missing_count: usize,
    pub metadata_error: Option<String>,
    pub imported_works: Vec<LocalWorkImportItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorkImportItem {
    pub work_id: String,
    pub local_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRemovalReport {
    pub account_id: String,
    pub label: String,
    pub credential_deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkDownloadReport {
    pub warnings: Vec<String>,
    pub work_id: String,
    pub account_id: String,
    pub local_path: PathBuf,
    pub file_count: usize,
    pub archive_extracted: bool,
    pub download_state: WorkDownloadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkDownloadProgress {
    Warning(String),
    LoggingIn,
    ResolvingPlan,
    Download(DownloadProgress),
    Finalizing,
    Completed,
}

pub trait WorkDownloadProgressSink: Send + Sync {
    fn emit(&self, progress: WorkDownloadProgress);
}

struct BulkWorkDownloadSelection {
    total_count: u64,
    skipped_downloaded_count: usize,
    items: Vec<BulkWorkDownloadSelectionItem>,
}

struct BulkWorkDownloadSelectionItem {
    work_id: String,
    content_size_bytes: Option<u64>,
}

#[derive(Clone)]
pub struct BulkWorkDownloadRequest<'a> {
    pub query: ProductListQuery,
    pub work_ids: Option<Vec<String>>,
    pub library_root: &'a Path,
    pub download_root: &'a Path,
    pub unpack_policy: UnpackPolicy,
    pub skip_downloaded: bool,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub progress_sink: Option<&'a dyn BulkWorkDownloadProgressSink>,
    pub two_factor_prompt: Option<&'a dyn TwoFactorPrompt>,
}

impl<'a> BulkWorkDownloadRequest<'a> {
    pub fn new(query: ProductListQuery, library_root: &'a Path, download_root: &'a Path) -> Self {
        Self {
            query,
            work_ids: None,
            library_root,
            download_root,
            unpack_policy: UnpackPolicy::UnpackWhenRecognized,
            skip_downloaded: true,
            cancellation_token: None,
            progress_sink: None,
            two_factor_prompt: None,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .is_some_and(CancellationToken::is_cancelled)
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(LibraryError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn emit(&self, progress: BulkWorkDownloadProgress) {
        if let Some(sink) = self.progress_sink {
            sink.emit(progress);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadReport {
    pub total_count: u64,
    pub requested_count: usize,
    pub skipped_downloaded_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub succeeded_works: Vec<BulkWorkDownloadSuccess>,
    pub failed_works: Vec<BulkWorkDownloadFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadSuccess {
    pub warnings: Vec<String>,
    pub work_id: String,
    pub local_path: PathBuf,
    pub file_count: usize,
    pub archive_extracted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadFailure {
    pub work_id: String,
    pub error_code: String,
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkWorkDownloadProgress {
    Selecting,
    Selected {
        total_count: u64,
        requested_count: usize,
        skipped_downloaded_count: usize,
    },
    WorkStarted {
        work_id: String,
        current: usize,
        total: usize,
    },
    WorkCompleted {
        work_id: String,
        current: usize,
        total: usize,
    },
    WorkFailed {
        work_id: String,
        current: usize,
        total: usize,
        error_code: String,
        error_message: String,
    },
    Completed {
        report: BulkWorkDownloadReport,
    },
}

pub trait BulkWorkDownloadProgressSink: Send + Sync {
    fn emit(&self, progress: BulkWorkDownloadProgress);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkWorkDownloadPreviewProgress {
    Selecting,
    Selected {
        total_count: u64,
        requested_count: usize,
        skipped_downloaded_count: usize,
    },
    WorkStarted {
        work_id: String,
        current: usize,
        total: usize,
    },
    WorkPlanned {
        work_id: String,
        current: usize,
        total: usize,
        known_expected_bytes: u64,
        total_expected_bytes: Option<u64>,
        unknown_size_count: usize,
    },
    WorkFailed {
        work_id: String,
        current: usize,
        total: usize,
        error_code: String,
        error_message: String,
    },
    Completed {
        planned_count: usize,
        failed_count: usize,
        known_expected_bytes: u64,
        total_expected_bytes: Option<u64>,
        unknown_size_count: usize,
    },
}

pub trait BulkWorkDownloadPreviewProgressSink: Send + Sync {
    fn emit(&self, progress: BulkWorkDownloadPreviewProgress);
}

#[derive(Clone)]
pub struct BulkWorkDownloadPreviewRequest<'a> {
    pub query: ProductListQuery,
    pub work_ids: Option<Vec<String>>,
    pub skip_downloaded: bool,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub progress_sink: Option<&'a dyn BulkWorkDownloadPreviewProgressSink>,
}

impl BulkWorkDownloadPreviewRequest<'_> {
    fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .is_some_and(CancellationToken::is_cancelled)
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(LibraryError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn emit(&self, progress: BulkWorkDownloadPreviewProgress) {
        if let Some(sink) = self.progress_sink {
            sink.emit(progress);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadPreview {
    pub total_count: u64,
    pub requested_count: usize,
    pub skipped_downloaded_count: usize,
    pub planned_count: usize,
    pub failed_count: usize,
    pub known_expected_bytes: u64,
    pub total_expected_bytes: Option<u64>,
    pub unknown_size_count: usize,
    pub works: Vec<BulkWorkDownloadPreviewWork>,
    pub failed_works: Vec<BulkWorkDownloadFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadPreviewWork {
    pub work_id: String,
    pub file_count: usize,
    pub known_expected_bytes: u64,
    pub total_expected_bytes: Option<u64>,
    pub unknown_size_count: usize,
    pub files: Vec<BulkWorkDownloadPreviewFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkWorkDownloadPreviewFile {
    pub file_index: usize,
    pub file_name: String,
    pub expected_size: Option<u64>,
}

#[async_trait]
pub trait WorkDownloadSource: DlsiteAuthSource {
    async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan>;
    async fn download_file_metadata(
        &self,
        file_index: usize,
        file: &DownloadFile,
    ) -> Result<DownloadFileMetadata>;
    async fn download_files(
        &self,
        job: &DownloadJobRequest,
        plan: &DownloadPlan,
        cancellation: &dm_download::CancellationToken,
        progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
    ) -> Result<DownloadedWork>;
}

#[derive(Clone)]
pub struct DlsiteWorkDownloadSource {
    client: DlsiteClient,
}

impl DlsiteWorkDownloadSource {
    pub fn new(client: DlsiteClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DlsiteAuthSource for DlsiteWorkDownloadSource {
    async fn restore_session(&self, session: &SessionSnapshot) -> Result<()> {
        Ok(self.client.import_session(session)?)
    }

    async fn validate_session(&self) -> Result<bool> {
        Ok(self.client.validate_session().await? == SessionStatus::Authorized)
    }

    async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome> {
        Ok(self.client.begin_login(credentials).await?)
    }

    async fn complete_two_factor(
        &self,
        challenge: &TwoFactorChallenge,
        code: &str,
    ) -> Result<SessionSnapshot> {
        Ok(self.client.complete_two_factor(challenge, code).await?)
    }
}

#[async_trait]
impl WorkDownloadSource for DlsiteWorkDownloadSource {
    async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
        Ok(self.client.download_plan(work_id).await?)
    }

    async fn download_file_metadata(
        &self,
        file_index: usize,
        file: &DownloadFile,
    ) -> Result<DownloadFileMetadata> {
        Ok(dm_download::probe_download_file_metadata(&self.client, file_index, file).await?)
    }

    async fn download_files(
        &self,
        job: &DownloadJobRequest,
        plan: &DownloadPlan,
        cancellation: &dm_download::CancellationToken,
        progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
    ) -> Result<DownloadedWork> {
        Ok(dm_download::download_work_files(
            self.client.clone(),
            job,
            plan,
            cancellation,
            progress_sink,
        )
        .await?)
    }
}

#[async_trait]
pub trait LocalWorkMetadataSource: Send + Sync {
    async fn works(&self, ids: &[WorkId]) -> Result<Vec<PublicWork>>;
}

#[derive(Clone)]
pub struct DlsitePublicMetadataSource {
    client: DlsiteClient,
}

impl DlsitePublicMetadataSource {
    pub fn new(client: DlsiteClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LocalWorkMetadataSource for DlsitePublicMetadataSource {
    async fn works(&self, ids: &[WorkId]) -> Result<Vec<PublicWork>> {
        Ok(self.client.public_works(ids).await?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncProgress {
    LoggingIn,
    LoadingCount,
    LoadingPurchases,
    LoadingWorks {
        work_count: usize,
    },
    Committing {
        work_count: usize,
    },
    Completed {
        sync_run_id: String,
        cached_work_count: usize,
    },
}

pub trait SyncProgressSink: Send + Sync {
    fn emit(&self, progress: SyncProgress);
}

#[async_trait]
pub trait AccountSyncSource: DlsiteAuthSource {
    async fn content_count(&self) -> Result<ContentCount>;
    async fn purchases(&self) -> Result<Vec<Purchase>>;
    async fn works(&self, ids: &[WorkId]) -> Result<Vec<Work>>;
}

#[derive(Clone)]
pub struct DlsiteSyncSource {
    client: DlsiteClient,
}

impl DlsiteSyncSource {
    pub fn new(client: DlsiteClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &DlsiteClient {
        &self.client
    }
}

#[async_trait]
impl DlsiteAuthSource for DlsiteSyncSource {
    async fn restore_session(&self, session: &SessionSnapshot) -> Result<()> {
        Ok(self.client.import_session(session)?)
    }

    async fn validate_session(&self) -> Result<bool> {
        Ok(self.client.validate_session().await? == SessionStatus::Authorized)
    }

    async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome> {
        Ok(self.client.begin_login(credentials).await?)
    }

    async fn complete_two_factor(
        &self,
        challenge: &TwoFactorChallenge,
        code: &str,
    ) -> Result<SessionSnapshot> {
        Ok(self.client.complete_two_factor(challenge, code).await?)
    }
}

#[async_trait]
impl AccountSyncSource for DlsiteSyncSource {
    async fn content_count(&self) -> Result<ContentCount> {
        Ok(self.client.content_count(ContentQuery::default()).await?)
    }

    async fn purchases(&self) -> Result<Vec<Purchase>> {
        Ok(self.client.sales(ContentQuery::default()).await?)
    }

    async fn works(&self, ids: &[WorkId]) -> Result<Vec<Work>> {
        Ok(self.client.works(ids).await?)
    }
}

fn build_storage_sync(
    account_id: &str,
    sync_run_id: &str,
    started_at: &str,
    completed_at: &str,
    purchases: Vec<Purchase>,
    works: Vec<Work>,
) -> Result<AccountSyncBuild> {
    let works_by_id = works
        .into_iter()
        .map(|work| (work.id.as_ref().to_owned(), work))
        .collect::<BTreeMap<_, _>>();
    let mut storage_works = Vec::with_capacity(purchases.len());
    let mut account_works = Vec::with_capacity(purchases.len());
    let mut missing_detail_count = 0;

    for purchase in purchases {
        let work_id = purchase.id.as_ref().to_owned();

        if let Some(work) = works_by_id.get(&work_id) {
            storage_works.push(cached_work_from_api(work.clone(), completed_at)?);
        } else {
            missing_detail_count += 1;
            storage_works.push(cached_work_from_purchase_placeholder(
                &purchase,
                completed_at,
            )?);
        }

        account_works.push(AccountWork {
            work_id,
            purchased_at: Some(datetime_to_string(purchase.purchased_at)),
        });
    }

    Ok(AccountSyncBuild {
        commit: AccountSyncCommit {
            sync_run_id: sync_run_id.to_owned(),
            account_id: account_id.to_owned(),
            started_at: started_at.to_owned(),
            completed_at: completed_at.to_owned(),
            works: storage_works,
            account_works,
        },
        missing_detail_count,
    })
}

struct AccountSyncBuild {
    commit: AccountSyncCommit,
    missing_detail_count: usize,
}

fn cached_work_from_api(work: Work, synced_at: &str) -> Result<CachedWork> {
    let title = preferred_localized_text(&work.name)
        .cloned()
        .unwrap_or_else(|| work.id.to_string());
    let maker_name = preferred_localized_text(&work.maker.name).cloned();
    let age_category = serde_json::to_value(&work.age_category)?
        .as_str()
        .map(str::to_owned);

    Ok(CachedWork {
        work_id: work.id.as_ref().to_owned(),
        title,
        title_json: serde_json::to_string(&work.name)?,
        maker_id: Some(work.maker.id.clone()),
        maker_name,
        maker_json: Some(serde_json::to_string(&work.maker.name)?),
        work_type: Some(work.work_kind.code.clone()),
        age_category,
        thumbnail_url: Some(work.thumbnail.full.to_string()),
        registered_at: work.registered_at.map(datetime_to_string),
        published_at: work.published_at.map(datetime_to_string),
        updated_at: work.updated_at.map(datetime_to_string),
        raw_json: serde_json::to_string(&work)?,
        last_detail_sync_at: synced_at.to_owned(),
    })
}

fn cached_work_from_public_api(work: PublicWork, synced_at: &str) -> Result<CachedWork> {
    let work_id = work.id.as_ref().to_owned();
    let title = work
        .name
        .as_deref()
        .and_then(non_empty_trimmed)
        .unwrap_or(work_id.as_str())
        .to_owned();
    let title_json = localized_japanese_json(&title)?;
    let maker_name = work
        .maker_name
        .as_deref()
        .and_then(non_empty_trimmed)
        .map(str::to_owned);
    let maker_json = maker_name
        .as_deref()
        .map(localized_japanese_json)
        .transpose()?;
    let content_size = public_work_content_size(&work);

    Ok(CachedWork {
        work_id,
        title,
        title_json,
        maker_id: work
            .maker_id
            .as_deref()
            .and_then(non_empty_trimmed)
            .map(str::to_owned),
        maker_name,
        maker_json,
        work_type: work
            .work_kind
            .as_deref()
            .and_then(non_empty_trimmed)
            .map(str::to_owned),
        age_category: public_work_age_category(&work),
        thumbnail_url: public_work_thumbnail_url(&work),
        registered_at: work
            .registered_at
            .as_deref()
            .and_then(normalize_public_datetime),
        published_at: work
            .published_at
            .as_deref()
            .or(work.registered_at.as_deref())
            .and_then(normalize_public_datetime),
        updated_at: work
            .updated_at
            .as_deref()
            .and_then(normalize_public_datetime),
        raw_json: public_work_raw_json(&work, content_size)?,
        last_detail_sync_at: synced_at.to_owned(),
    })
}

fn cached_work_from_purchase_placeholder(
    purchase: &Purchase,
    synced_at: &str,
) -> Result<CachedWork> {
    let work_id = purchase.id.as_ref().to_owned();
    let raw_json = serde_json::to_string(&serde_json::json!({
        "workno": work_id,
        "source": "content/sales",
        "detail_status": "missing_from_content_works",
        "sales_date": datetime_to_string(purchase.purchased_at),
    }))?;

    Ok(CachedWork {
        work_id: work_id.clone(),
        title: work_id.clone(),
        title_json: serde_json::to_string(&serde_json::json!({ "en_US": work_id }))?,
        maker_id: None,
        maker_name: None,
        maker_json: None,
        work_type: None,
        age_category: None,
        thumbnail_url: None,
        registered_at: None,
        published_at: None,
        updated_at: None,
        raw_json,
        last_detail_sync_at: synced_at.to_owned(),
    })
}

fn cached_work_from_local_folder(
    work_id: &str,
    folder_name: &str,
    scanned_at: &str,
) -> Result<CachedWork> {
    let title = folder_name.trim();
    let title = if title.is_empty() { work_id } else { title };
    let raw_json = serde_json::to_string(&serde_json::json!({
        "workno": work_id,
        "source": "local_scan",
        "detail_status": "local_only",
        "folder_name": folder_name,
    }))?;

    Ok(CachedWork {
        work_id: work_id.to_owned(),
        title: title.to_owned(),
        title_json: serde_json::to_string(&serde_json::json!({ "en_US": title }))?,
        maker_id: None,
        maker_name: None,
        maker_json: None,
        work_type: None,
        age_category: None,
        thumbnail_url: None,
        registered_at: None,
        published_at: None,
        updated_at: None,
        raw_json,
        last_detail_sync_at: scanned_at.to_owned(),
    })
}

fn preferred_localized_text(text: &LocalizedText) -> Option<&String> {
    text.get(&Language::English)
        .or_else(|| text.get(&Language::Japanese))
        .or_else(|| text.get(&Language::Korean))
        .or_else(|| text.get(&Language::Taiwanese))
        .or_else(|| text.get(&Language::Chinese))
        .or_else(|| text.values().next())
}

fn localized_japanese_json(value: &str) -> Result<String> {
    let text = BTreeMap::from([(Language::Japanese, value.to_owned())]);

    Ok(serde_json::to_string(&text)?)
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn public_work_age_category(work: &PublicWork) -> Option<String> {
    public_age_category_value(work.age_category.as_ref())
        .or_else(|| public_age_category_label(work.age_category_label.as_deref()))
}

fn public_age_category_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Number(number) => match number.as_u64()? {
            1 => Some("all".to_owned()),
            2 => Some("r15".to_owned()),
            3 => Some("r18".to_owned()),
            _ => None,
        },
        Value::String(value) => public_age_category_label(Some(value)),
        _ => None,
    }
}

fn public_age_category_label(value: Option<&str>) -> Option<String> {
    let normalized = value?
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_', ' '], "");

    if normalized.is_empty() {
        None
    } else if normalized.contains("adult") || normalized.contains("r18") || normalized == "18" {
        Some("r18".to_owned())
    } else if normalized.contains("r15") || normalized == "15" {
        Some("r15".to_owned())
    } else if normalized.contains("general") || normalized.contains("all") {
        Some("all".to_owned())
    } else {
        None
    }
}

fn public_work_content_size(work: &PublicWork) -> Option<u64> {
    work.content_size.as_ref().and_then(json_value_as_u64)
}

fn public_work_thumbnail_url(work: &PublicWork) -> Option<String> {
    work.image_main_url()
        .or(work.image_thumb.as_deref())
        .and_then(normalize_public_url)
}

fn normalize_public_url(value: &str) -> Option<String> {
    let value = value.trim();

    if value.is_empty() {
        None
    } else if value.starts_with("//") {
        Some(format!("https:{value}"))
    } else if value.starts_with("http://") || value.starts_with("https://") {
        Some(value.to_owned())
    } else if value.starts_with('/') {
        Some(format!("https://www.dlsite.com{value}"))
    } else {
        Some(format!("https://img.dlsite.jp/{value}"))
    }
}

fn normalize_public_datetime(value: &str) -> Option<String> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    if let Ok(value) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime_to_string(value.with_timezone(&Utc)));
    }

    if let Ok(value) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(datetime_to_string(
            DateTime::<Utc>::from_naive_utc_and_offset(value, Utc),
        ));
    }

    if let Ok(value) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let value = value.and_hms_opt(0, 0, 0)?;
        return Some(datetime_to_string(
            DateTime::<Utc>::from_naive_utc_and_offset(value, Utc),
        ));
    }

    Some(value.to_owned())
}

fn public_work_raw_json(work: &PublicWork, content_size: Option<u64>) -> Result<String> {
    let mut value = serde_json::to_value(work)?;
    let tags = public_work_tags(&value);

    if let Value::Object(map) = &mut value {
        map.insert("source".to_owned(), json!("public_product"));

        if let Some(content_size) = content_size {
            map.insert("content_size".to_owned(), json!(content_size));
        }

        if !tags.is_empty() {
            map.insert("tags".to_owned(), Value::Array(tags));
        }
    }

    Ok(serde_json::to_string(&value)?)
}

fn public_work_tags(value: &Value) -> Vec<Value> {
    let mut tags = BTreeSet::<(String, String)>::new();

    if let Some(existing_tags) = value.get("tags").and_then(Value::as_array) {
        for tag in existing_tags {
            if let (Some(class), Some(name)) = (
                tag.get("class").and_then(Value::as_str),
                tag.get("name").and_then(Value::as_str),
            ) {
                push_public_tag(&mut tags, class, name);
            }
        }
    }

    if let Some(genres) = value.get("genres").and_then(Value::as_array) {
        for genre in genres {
            let name = genre
                .get("name")
                .or_else(|| genre.get("name_base"))
                .and_then(Value::as_str);

            if let Some(name) = name {
                push_public_tag(&mut tags, "genre", name);
            }
        }
    }

    if let Some(creators) = value.get("creaters").and_then(Value::as_object) {
        for (class, values) in creators {
            push_public_credit_values(&mut tags, class, values);
        }
    }

    for class in [
        "voice_by",
        "illust_by",
        "scenario_by",
        "created_by",
        "music_by",
        "other_by",
    ] {
        if let Some(values) = value.get(class) {
            push_public_credit_values(&mut tags, class, values);
        }
    }

    tags.into_iter()
        .map(|(class, name)| json!({ "class": class, "name": name }))
        .collect()
}

fn push_public_credit_values(
    tags: &mut BTreeSet<(String, String)>,
    fallback_class: &str,
    value: &Value,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                push_public_credit_values(tags, fallback_class, value);
            }
        }
        Value::Object(map) => {
            if let Some(name) = map.get("name").and_then(Value::as_str) {
                let class = map
                    .get("classification")
                    .and_then(Value::as_str)
                    .unwrap_or(fallback_class);
                push_public_tag(tags, class, name);
            } else {
                for value in map.values() {
                    push_public_credit_values(tags, fallback_class, value);
                }
            }
        }
        Value::String(name) => push_public_tag(tags, fallback_class, name),
        _ => {}
    }
}

fn push_public_tag(tags: &mut BTreeSet<(String, String)>, class: &str, name: &str) {
    let class = class.trim();
    let name = name.trim();

    if !class.is_empty() && !name.is_empty() {
        tags.insert((class.to_owned(), name.to_owned()));
    }
}

fn json_value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

fn datetime_to_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn unpack_policy_storage_value(policy: UnpackPolicy) -> &'static str {
    match policy {
        UnpackPolicy::KeepArchives => "keep_archives",
        UnpackPolicy::UnpackWhenRecognized => "unpack_when_recognized",
    }
}

async fn write_serial_information_file(
    target_dir: &Path,
    work_id: &str,
    serial_numbers: &[SerialNumber],
    generated_at: &str,
) -> Result<Option<PathBuf>> {
    if serial_numbers.is_empty() {
        remove_owned_serial_information_files(target_dir).await?;
        return Ok(None);
    }

    let path = serial_information_path(target_dir).await?;
    let content = serial_information_content(work_id, serial_numbers, generated_at);

    tokio::fs::write(&path, content).await?;
    Ok(Some(path))
}

async fn serial_information_path(target_dir: &Path) -> Result<PathBuf> {
    let base = target_dir.join(SERIAL_INFORMATION_FILE_NAME);

    if serial_information_path_available(&base).await? {
        return Ok(base);
    }

    for index in 1..100 {
        let candidate = target_dir.join(format!("{SERIAL_INFORMATION_NUMBERED_PREFIX}{index}.txt"));

        if serial_information_path_available(&candidate).await? {
            return Ok(candidate);
        }
    }

    Err(LibraryError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not choose a serial information file name",
    )))
}

async fn serial_information_path_available(path: &Path) -> Result<bool> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(content.starts_with(SERIAL_INFORMATION_MARKER)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

async fn remove_owned_serial_information_files(target_dir: &Path) -> Result<()> {
    remove_owned_serial_information_file(&target_dir.join(SERIAL_INFORMATION_FILE_NAME)).await?;

    for index in 1..100 {
        remove_owned_serial_information_file(
            &target_dir.join(format!("{SERIAL_INFORMATION_NUMBERED_PREFIX}{index}.txt")),
        )
        .await?;
    }

    Ok(())
}

async fn remove_owned_serial_information_file(path: &Path) -> Result<()> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) if content.starts_with(SERIAL_INFORMATION_MARKER) => {
            tokio::fs::remove_file(path).await?;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn serial_information_content(
    work_id: &str,
    serial_numbers: &[SerialNumber],
    generated_at: &str,
) -> String {
    let mut content = String::new();

    content.push_str(SERIAL_INFORMATION_MARKER);
    content.push('\n');
    content.push_str(&format!("Work ID: {}\n", clean_serial_text(work_id)));
    content.push_str(&format!("Generated at: {generated_at}\n"));
    content.push_str("Source: DLsite download plan\n\n");
    content.push_str("Treat these serial values as sensitive.\n\n");
    content.push_str("Serial numbers:\n");

    for (index, serial) in serial_numbers.iter().enumerate() {
        content.push_str(&format!(
            "{}. {}\n",
            index + 1,
            clean_serial_text(&serial.label)
        ));
        content.push_str(&format!("   Value: {}\n", clean_serial_text(&serial.value)));
    }

    content
}

fn clean_serial_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

async fn remove_download_path_from_state(
    path: Option<&str>,
    allowed_roots: &[&Path],
) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };

    remove_existing_download_path(Path::new(path), allowed_roots).await
}

async fn remove_existing_download_path(path: &Path, allowed_roots: &[&Path]) -> Result<()> {
    if !path.try_exists()? {
        return Ok(());
    }

    let canonical_path = path.canonicalize()?;
    let canonical_roots = allowed_roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect::<Vec<_>>();

    if !path_is_download_child_of_any_root(&canonical_path, &canonical_roots) {
        return Err(LibraryError::DownloadPathOutsideRoots(canonical_path));
    }

    let metadata = tokio::fs::metadata(&canonical_path).await?;
    if metadata.is_dir() {
        tokio::fs::remove_dir_all(&canonical_path).await?;
    } else {
        tokio::fs::remove_file(&canonical_path).await?;
    }

    Ok(())
}

fn canonicalize_existing_directory(path: &Path) -> Result<PathBuf> {
    let canonical_path = path.canonicalize()?;
    let metadata = std::fs::metadata(&canonical_path)?;

    if !metadata.is_dir() {
        return Err(LibraryError::DownloadPathNotDirectory(canonical_path));
    }

    Ok(canonical_path)
}

fn path_is_download_child_of_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .any(|root| path != root.as_path() && path.starts_with(root))
}

pub const DL_SITE_WORK_ID_PREFIXES: &[&str] = &["RJ", "VJ", "BJ"];
const WORK_ID_MIN_DIGITS: usize = 6;
const WORK_ID_MAX_DIGITS: usize = 8;

pub fn detect_work_ids_in_text(text: &str) -> Vec<WorkId> {
    let text = text.to_ascii_uppercase();
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut seen = BTreeSet::new();
    let mut detected = Vec::new();

    while index + 2 + WORK_ID_MIN_DIGITS <= bytes.len() {
        let Some(prefix) = work_id_prefix_at(bytes, index) else {
            index += 1;
            continue;
        };

        if !is_work_id_boundary_before(bytes, index) {
            index += 1;
            continue;
        }

        let digit_start = index + prefix.len();
        let mut digit_end = digit_start;

        while digit_end < bytes.len() && bytes[digit_end].is_ascii_digit() {
            digit_end += 1;
        }

        let digit_count = digit_end.saturating_sub(digit_start);
        let is_valid = (WORK_ID_MIN_DIGITS..=WORK_ID_MAX_DIGITS).contains(&digit_count)
            && is_work_id_boundary_after(bytes, digit_end);

        if is_valid {
            let digits =
                std::str::from_utf8(&bytes[digit_start..digit_end]).expect("digits are ascii");
            let work_id = format!("{prefix}{digits}");

            if seen.insert(work_id.clone()) {
                detected.push(WorkId::from(work_id));
            }

            index = digit_end;
        } else {
            index += 1;
        }
    }

    detected
}

pub fn detect_work_ids_in_path(path: impl AsRef<Path>) -> Vec<WorkId> {
    path.as_ref()
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(detect_work_ids_in_text)
        .unwrap_or_default()
}

pub fn detect_unique_work_id_in_path(path: impl AsRef<Path>) -> Option<WorkId> {
    let mut detected = detect_work_ids_in_path(path);

    if detected.len() == 1 {
        detected.pop()
    } else {
        None
    }
}

fn work_id_prefix_at(bytes: &[u8], index: usize) -> Option<&'static str> {
    DL_SITE_WORK_ID_PREFIXES.iter().copied().find(|prefix| {
        bytes
            .get(index..index + prefix.len())
            .is_some_and(|candidate| candidate == prefix.as_bytes())
    })
}

fn is_work_id_boundary_before(bytes: &[u8], index: usize) -> bool {
    index == 0 || !bytes[index - 1].is_ascii_alphanumeric()
}

fn is_work_id_boundary_after(bytes: &[u8], index: usize) -> bool {
    index == bytes.len() || !bytes[index].is_ascii_alphanumeric()
}

fn now_string() -> String {
    datetime_to_string(Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_api::{
        AgeCategory, DownloadFile, DownloadFileKind, DownloadPlan, DownloadStreamRequest, Maker,
        PublicWorkImage, PublicWorkImageObject, WorkKind, WorkThumbnail,
    };
    use dm_credentials::InMemoryCredentialStore;
    use dm_download::{DownloadPhase, DownloadedFile};
    use dm_storage::{ProductSort, SyncRunStatus, WorkDownloadStatus};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex,
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use url::Url;

    #[test]
    fn support_details_preserve_safe_api_failure_context() {
        let error = LibraryError::Api(DmApiError::DownloadUnknownRedirect {
            work_id: WorkId::from("RJ123456"),
            location: Url::parse(
                "https://download.dlsite.com/get/=/file/RJ123456.zip?token=secret",
            )
            .unwrap(),
        });

        let details = error.support_details();

        assert_eq!(error.failure_code(), "api");
        assert!(error
            .support_message()
            .contains("unknown download redirect for RJ123456"));
        assert_eq!(details["failureKind"], "api");
        assert_eq!(details["apiErrorKind"], "download_unknown_redirect");
        assert_eq!(details["workId"], "RJ123456");
        assert_eq!(
            details["location"],
            "https://download.dlsite.com/<download-url-redacted>"
        );
        assert!(!details.to_string().contains("secret"));
    }

    #[test]
    fn support_details_preserve_download_api_failure_context() {
        let error =
            LibraryError::Download(dm_download::DownloadError::Api(DmApiError::NotAuthorized));

        let details = error.support_details();

        assert_eq!(error.failure_code(), "download");
        assert!(error
            .support_message()
            .contains("DLsite session is not authorized"));
        assert_eq!(details["failureKind"], "api");
        assert_eq!(details["downloadErrorKind"], "api");
        assert_eq!(details["apiErrorKind"], "not_authorized");
    }

    #[test]
    fn support_body_snippets_redact_urls_and_serial_material() {
        let html = r#"<a href="https://download.dlsite.com/get/secret?token=value">download</a>"#;

        let snippet = safe_body_snippet_for_log(html).unwrap();

        assert!(snippet.contains("[redacted-url]"));
        assert!(!snippet.contains("download.dlsite.com"));
        assert!(!snippet.contains("token=value"));

        let serial_snippet = safe_body_snippet_for_log("serial number: ABCD-1234").unwrap();
        assert_eq!(
            serial_snippet,
            "[redacted because response may contain serial information]"
        );
    }

    #[derive(Debug, Clone)]
    struct FakeSyncSource {
        content_count: ContentCount,
        purchases: Vec<Purchase>,
        works: Vec<Work>,
        fail_at: Option<FakeFailurePoint>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeFailurePoint {
        Login,
        Works,
    }

    fn assert_fake_credentials(credentials: &Credentials) {
        assert_eq!(credentials.username, "user@example.test");
        assert_eq!(credentials.password, "secret");
    }

    fn fake_session() -> SessionSnapshot {
        SessionSnapshot {
            cookies_json: "{}".to_owned(),
        }
    }

    /// Test sources authenticate with a password in one step; `validate_session` always says
    /// no so every operation exercises the login path rather than a stored session.
    macro_rules! impl_fake_auth_source {
        ($source:ty) => {
            #[async_trait]
            impl DlsiteAuthSource for $source {
                async fn restore_session(&self, _session: &SessionSnapshot) -> Result<()> {
                    Ok(())
                }

                async fn validate_session(&self) -> Result<bool> {
                    Ok(false)
                }

                async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome> {
                    assert_fake_credentials(credentials);
                    Ok(LoginOutcome::Authorized(fake_session()))
                }

                async fn complete_two_factor(
                    &self,
                    _challenge: &TwoFactorChallenge,
                    _code: &str,
                ) -> Result<SessionSnapshot> {
                    unreachable!("fake source never issues a two-factor challenge")
                }
            }
        };
    }

    #[async_trait]
    impl DlsiteAuthSource for FakeSyncSource {
        async fn restore_session(&self, _session: &SessionSnapshot) -> Result<()> {
            Ok(())
        }

        async fn validate_session(&self) -> Result<bool> {
            Ok(false)
        }

        async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome> {
            assert_fake_credentials(credentials);

            if self.fail_at == Some(FakeFailurePoint::Login) {
                return Err(LibraryError::SyncSource("login failed".to_owned()));
            }

            Ok(LoginOutcome::Authorized(fake_session()))
        }

        async fn complete_two_factor(
            &self,
            _challenge: &TwoFactorChallenge,
            _code: &str,
        ) -> Result<SessionSnapshot> {
            unreachable!("fake sync source never issues a two-factor challenge")
        }
    }

    #[async_trait]
    impl AccountSyncSource for FakeSyncSource {
        async fn content_count(&self) -> Result<ContentCount> {
            Ok(self.content_count.clone())
        }

        async fn purchases(&self) -> Result<Vec<Purchase>> {
            Ok(self.purchases.clone())
        }

        async fn works(&self, ids: &[WorkId]) -> Result<Vec<Work>> {
            if self.fail_at == Some(FakeFailurePoint::Works) {
                return Err(LibraryError::SyncSource("works failed".to_owned()));
            }

            let requested = ids.iter().map(|id| id.as_ref()).collect::<Vec<_>>();
            let works = self
                .works
                .iter()
                .filter(|work| requested.contains(&work.id.as_ref()))
                .cloned()
                .collect();

            Ok(works)
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeLocalWorkMetadataSource {
        works: Vec<PublicWork>,
        fail: bool,
    }

    #[async_trait]
    impl LocalWorkMetadataSource for FakeLocalWorkMetadataSource {
        async fn works(&self, ids: &[WorkId]) -> Result<Vec<PublicWork>> {
            if self.fail {
                return Err(LibraryError::SyncSource(
                    "local metadata lookup failed".to_owned(),
                ));
            }

            let requested = ids.iter().map(|id| id.as_ref()).collect::<Vec<_>>();
            Ok(self
                .works
                .iter()
                .filter(|work| requested.contains(&work.id.as_ref()))
                .cloned()
                .collect())
        }
    }

    #[derive(Debug, Clone, Default)]
    struct FakeDownloadSource;

    impl_fake_auth_source!(FakeDownloadSource);

    #[async_trait]
    impl WorkDownloadSource for FakeDownloadSource {
        async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
            Ok(DownloadPlan {
                work_id: work_id.clone(),
                files: vec![DownloadFile {
                    kind: DownloadFileKind::Direct,
                    stream_request: DownloadStreamRequest {
                        url: Url::parse(
                            "https://download.example.test/get/=/file/RJ000001.zip/_/1",
                        )
                        .unwrap(),
                    },
                }],
                serial_numbers: Vec::new(),
            })
        }

        async fn download_file_metadata(
            &self,
            file_index: usize,
            file: &DownloadFile,
        ) -> Result<DownloadFileMetadata> {
            Ok(DownloadFileMetadata {
                file_index,
                file_kind: file.kind.clone(),
                file_name: format!("file-{file_index}.zip"),
                expected_size: Some(10),
                final_url: file.stream_request.url.clone(),
            })
        }

        async fn download_files(
            &self,
            job: &DownloadJobRequest,
            _plan: &DownloadPlan,
            _cancellation: &dm_download::CancellationToken,
            progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
        ) -> Result<DownloadedWork> {
            let target_dir = job.target_root.join(job.work_id.as_ref());
            let path = target_dir.join("RJ000001.txt");
            tokio::fs::create_dir_all(&target_dir).await?;
            tokio::fs::write(&path, b"downloaded").await?;
            progress_sink(DownloadProgress {
                phase: DownloadPhase::Downloading,
                file_index: Some(0),
                file_kind: Some(DownloadFileKind::Direct),
                bytes_received: 10,
                bytes_total: Some(10),
            });

            Ok(DownloadedWork {
                work_id: job.work_id.clone(),
                target_dir,
                files: vec![DownloadedFile {
                    file_name: "RJ000001.txt".to_owned(),
                    path,
                    bytes_written: 10,
                    resumed_from: 0,
                }],
                archive_extraction: None,
            })
        }
    }

    #[derive(Debug, Clone, Default)]
    struct SerialDownloadSource;

    impl_fake_auth_source!(SerialDownloadSource);

    #[async_trait]
    impl WorkDownloadSource for SerialDownloadSource {
        async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
            let mut plan = FakeDownloadSource.download_plan(work_id).await?;
            plan.serial_numbers = vec![SerialNumber {
                label: "Serial number".to_owned(),
                value: "ABC-123-SECRET".to_owned(),
            }];

            Ok(plan)
        }

        async fn download_file_metadata(
            &self,
            file_index: usize,
            file: &DownloadFile,
        ) -> Result<DownloadFileMetadata> {
            FakeDownloadSource
                .download_file_metadata(file_index, file)
                .await
        }

        async fn download_files(
            &self,
            job: &DownloadJobRequest,
            plan: &DownloadPlan,
            cancellation: &dm_download::CancellationToken,
            progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
        ) -> Result<DownloadedWork> {
            FakeDownloadSource
                .download_files(job, plan, cancellation, progress_sink)
                .await
        }
    }

    #[derive(Debug, Clone)]
    struct WaitingDownloadSource {
        entered_download: Arc<AtomicBool>,
    }

    impl WaitingDownloadSource {
        fn new() -> Self {
            Self {
                entered_download: Arc::new(AtomicBool::new(false)),
            }
        }

        fn entered_download(&self) -> bool {
            self.entered_download.load(Ordering::SeqCst)
        }
    }

    impl_fake_auth_source!(WaitingDownloadSource);

    #[async_trait]
    impl WorkDownloadSource for WaitingDownloadSource {
        async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
            Ok(DownloadPlan {
                work_id: work_id.clone(),
                files: vec![DownloadFile {
                    kind: DownloadFileKind::Direct,
                    stream_request: DownloadStreamRequest {
                        url: Url::parse(
                            "https://download.example.test/get/=/file/RJ000001.zip/_/1",
                        )
                        .unwrap(),
                    },
                }],
                serial_numbers: Vec::new(),
            })
        }

        async fn download_file_metadata(
            &self,
            file_index: usize,
            file: &DownloadFile,
        ) -> Result<DownloadFileMetadata> {
            Ok(DownloadFileMetadata {
                file_index,
                file_kind: file.kind.clone(),
                file_name: format!("file-{file_index}.zip"),
                expected_size: Some(10),
                final_url: file.stream_request.url.clone(),
            })
        }

        async fn download_files(
            &self,
            _job: &DownloadJobRequest,
            _plan: &DownloadPlan,
            cancellation: &dm_download::CancellationToken,
            _progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
        ) -> Result<DownloadedWork> {
            self.entered_download.store(true, Ordering::SeqCst);

            loop {
                if cancellation.is_cancelled() {
                    return Err(dm_download::DownloadError::Cancelled.into());
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    #[derive(Debug, Clone)]
    struct FailingDownloadSource {
        fail_work_id: &'static str,
    }

    impl_fake_auth_source!(FailingDownloadSource);

    #[async_trait]
    impl WorkDownloadSource for FailingDownloadSource {
        async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
            FakeDownloadSource.download_plan(work_id).await
        }

        async fn download_file_metadata(
            &self,
            file_index: usize,
            file: &DownloadFile,
        ) -> Result<DownloadFileMetadata> {
            FakeDownloadSource
                .download_file_metadata(file_index, file)
                .await
        }

        async fn download_files(
            &self,
            job: &DownloadJobRequest,
            plan: &DownloadPlan,
            cancellation: &dm_download::CancellationToken,
            progress_sink: &mut (dyn FnMut(DownloadProgress) + Send),
        ) -> Result<DownloadedWork> {
            if job.work_id.as_ref() == self.fail_work_id {
                return Err(LibraryError::SyncSource("download failed".to_owned()));
            }

            FakeDownloadSource
                .download_files(job, plan, cancellation, progress_sink)
                .await
        }
    }

    #[derive(Debug, Default)]
    struct RecordingProgressSink {
        events: Mutex<Vec<SyncProgress>>,
    }

    impl SyncProgressSink for RecordingProgressSink {
        fn emit(&self, progress: SyncProgress) {
            self.events.lock().expect("events lock").push(progress);
        }
    }

    #[derive(Debug, Default)]
    struct RecordingDownloadProgressSink {
        events: Mutex<Vec<WorkDownloadProgress>>,
    }

    impl WorkDownloadProgressSink for RecordingDownloadProgressSink {
        fn emit(&self, progress: WorkDownloadProgress) {
            self.events.lock().expect("events lock").push(progress);
        }
    }

    #[derive(Debug, Default)]
    struct RecordingBulkDownloadProgressSink {
        events: Mutex<Vec<BulkWorkDownloadProgress>>,
    }

    impl BulkWorkDownloadProgressSink for RecordingBulkDownloadProgressSink {
        fn emit(&self, progress: BulkWorkDownloadProgress) {
            self.events.lock().expect("events lock").push(progress);
        }
    }

    #[derive(Debug, Default)]
    struct RecordingBulkDownloadPreviewProgressSink {
        events: Mutex<Vec<BulkWorkDownloadPreviewProgress>>,
    }

    impl BulkWorkDownloadPreviewProgressSink for RecordingBulkDownloadPreviewProgressSink {
        fn emit(&self, progress: BulkWorkDownloadPreviewProgress) {
            self.events.lock().expect("events lock").push(progress);
        }
    }

    async fn migrated_library() -> Result<Library> {
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;

        Ok(Library::new(
            storage,
            Arc::new(InMemoryCredentialStore::new()),
        ))
    }

    fn save_account_request(remember_password: bool) -> SaveAccountRequest {
        SaveAccountRequest {
            id: Some("account-a".to_owned()),
            label: "Account A".to_owned(),
            login_name: Some("user@example.test".to_owned()),
            password: Some("secret".to_owned()),
            remember_password,
            enabled: true,
        }
    }

    fn purchase(work_id: &str, purchased_at: &str) -> Purchase {
        Purchase {
            id: WorkId::new(work_id),
            purchased_at: DateTime::parse_from_rfc3339(purchased_at)
                .expect("date")
                .with_timezone(&Utc),
        }
    }

    fn localized(value: &str) -> LocalizedText {
        BTreeMap::from([(Language::Japanese, value.to_owned())])
    }

    fn work(work_id: &str, title: &str, maker: &str, published_at: &str) -> Work {
        let published_at = DateTime::parse_from_rfc3339(published_at)
            .expect("date")
            .with_timezone(&Utc);

        Work {
            id: WorkId::new(work_id),
            name: localized(title),
            maker: Maker {
                id: format!("maker-{maker}"),
                name: localized(maker),
            },
            work_kind: WorkKind {
                code: "SOU".to_owned(),
            },
            age_category: AgeCategory::All,
            genre_ids: vec![1, 2],
            thumbnail: WorkThumbnail {
                full: Url::parse(&format!("https://img.example.test/{work_id}/full.jpg"))
                    .expect("url"),
                small_square: Url::parse(&format!("https://img.example.test/{work_id}/small.jpg"))
                    .expect("url"),
            },
            registered_at: Some(published_at),
            published_at: Some(published_at),
            updated_at: Some(published_at),
            tags: Vec::new(),
            content_size: Some(10),
            extra: BTreeMap::new(),
        }
    }

    fn public_work(work_id: &str, title: &str, maker: &str) -> PublicWork {
        PublicWork {
            id: WorkId::new(work_id),
            name: Some(title.to_owned()),
            maker_id: Some(format!("maker-{maker}")),
            maker_name: Some(maker.to_owned()),
            work_kind: Some("SOU".to_owned()),
            work_kind_label: Some("ボイス・ASMR".to_owned()),
            age_category: Some(json!(3)),
            age_category_label: Some("adult".to_owned()),
            image_main: Some(PublicWorkImage::Object(PublicWorkImageObject {
                url: Some(format!("//img.example.test/{work_id}/main.jpg")),
                relative_url: None,
            })),
            image_thumb: Some(format!("//img.example.test/{work_id}/main_240x240.jpg")),
            registered_at: Some("2026-04-24 00:00:00".to_owned()),
            published_at: None,
            updated_at: Some("2026-04-25 09:09:14".to_owned()),
            content_size: Some(json!(2048)),
            extra: BTreeMap::from([
                (
                    "creaters".to_owned(),
                    json!({
                        "voice_by": [
                            {
                                "name": "Voice One",
                                "classification": "voice_by"
                            }
                        ]
                    }),
                ),
                (
                    "genres".to_owned(),
                    json!([
                        {
                            "name": "ASMR",
                            "id": 497
                        }
                    ]),
                ),
            ]),
        }
    }

    const VALID_TWO_FACTOR_CODE: &str = "123456";
    const TWO_FACTOR_SESSION: &str = "{\"two-factor\":true}";

    /// Sync source for a two-factor-enabled account: the password alone only ever yields a
    /// challenge, and a session becomes valid once it has been issued by a correct code.
    #[derive(Clone)]
    struct TwoFactorSyncSource {
        inner: FakeSyncSource,
        state: Arc<Mutex<TwoFactorSourceState>>,
    }

    #[derive(Default)]
    struct TwoFactorSourceState {
        issued_session: Option<String>,
        restored_session: Option<String>,
        submitted_codes: Vec<String>,
        login_count: u32,
    }

    impl TwoFactorSyncSource {
        fn new() -> Self {
            Self {
                inner: sync_source(),
                state: Arc::new(Mutex::new(TwoFactorSourceState::default())),
            }
        }

        fn state(&self) -> std::sync::MutexGuard<'_, TwoFactorSourceState> {
            self.state.lock().expect("two-factor state lock")
        }

        fn challenge() -> TwoFactorChallenge {
            TwoFactorChallenge {
                page_url: Url::parse("https://login.dlsite.com/login/two-factor").unwrap(),
                action: Url::parse("https://login.dlsite.com/login/two-factor?user=self").unwrap(),
                code_field: "otp".to_owned(),
                fields: vec![("_token".to_owned(), "token".to_owned())],
            }
        }
    }

    #[async_trait]
    impl DlsiteAuthSource for TwoFactorSyncSource {
        async fn restore_session(&self, session: &SessionSnapshot) -> Result<()> {
            self.state().restored_session = Some(session.cookies_json.clone());
            Ok(())
        }

        async fn validate_session(&self) -> Result<bool> {
            let state = self.state();
            Ok(state.restored_session.is_some() && state.restored_session == state.issued_session)
        }

        async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome> {
            assert_fake_credentials(credentials);
            self.state().login_count += 1;
            Ok(LoginOutcome::TwoFactorRequired(Self::challenge()))
        }

        async fn complete_two_factor(
            &self,
            challenge: &TwoFactorChallenge,
            code: &str,
        ) -> Result<SessionSnapshot> {
            assert_eq!(challenge.code_field, "otp");

            let mut state = self.state();
            state.submitted_codes.push(code.to_owned());

            if code != VALID_TWO_FACTOR_CODE {
                return Err(LibraryError::Api(DmApiError::InvalidTwoFactorCode));
            }

            state.issued_session = Some(TWO_FACTOR_SESSION.to_owned());

            Ok(SessionSnapshot {
                cookies_json: TWO_FACTOR_SESSION.to_owned(),
            })
        }
    }

    #[async_trait]
    impl WorkDownloadSource for TwoFactorSyncSource {
        async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
            FakeDownloadSource.download_plan(work_id).await
        }

        async fn download_file_metadata(
            &self,
            index: usize,
            file: &DownloadFile,
        ) -> Result<DownloadFileMetadata> {
            FakeDownloadSource.download_file_metadata(index, file).await
        }

        async fn download_files(
            &self,
            job: &DownloadJobRequest,
            plan: &DownloadPlan,
            cancellation: &dm_download::CancellationToken,
            sink: &mut (dyn FnMut(DownloadProgress) + Send),
        ) -> Result<DownloadedWork> {
            FakeDownloadSource
                .download_files(job, plan, cancellation, sink)
                .await
        }
    }

    #[async_trait]
    impl AccountSyncSource for TwoFactorSyncSource {
        async fn content_count(&self) -> Result<ContentCount> {
            self.inner.content_count().await
        }

        async fn purchases(&self) -> Result<Vec<Purchase>> {
            self.inner.purchases().await
        }

        async fn works(&self, ids: &[WorkId]) -> Result<Vec<Work>> {
            self.inner.works(ids).await
        }
    }

    /// Prompt that hands back a scripted sequence of answers and records what it was asked.
    #[derive(Clone, Default)]
    struct ScriptedTwoFactorPrompt {
        answers: Arc<Mutex<Vec<TwoFactorPromptResponse>>>,
        requests: Arc<Mutex<Vec<TwoFactorPromptRequest>>>,
    }

    impl ScriptedTwoFactorPrompt {
        fn new(answers: impl IntoIterator<Item = &'static str>) -> Self {
            Self::with_responses(
                answers
                    .into_iter()
                    .map(|code| TwoFactorPromptResponse::Code(code.to_owned())),
            )
        }

        fn with_responses(answers: impl IntoIterator<Item = TwoFactorPromptResponse>) -> Self {
            Self {
                answers: Arc::new(Mutex::new(answers.into_iter().collect())),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn requests(&self) -> Vec<TwoFactorPromptRequest> {
            self.requests.lock().expect("prompt lock").clone()
        }
    }

    #[async_trait]
    impl TwoFactorPrompt for ScriptedTwoFactorPrompt {
        async fn request_code(
            &self,
            request: TwoFactorPromptRequest,
        ) -> Result<TwoFactorPromptResponse> {
            self.requests.lock().expect("prompt lock").push(request);

            let mut answers = self.answers.lock().expect("prompt lock");

            assert!(
                !answers.is_empty(),
                "prompt was asked more often than scripted"
            );

            Ok(answers.remove(0))
        }
    }

    fn sync_source() -> FakeSyncSource {
        FakeSyncSource {
            content_count: ContentCount {
                user: 2,
                production: 0,
                page_limit: Some(50),
                concurrency: Some(500),
            },
            purchases: vec![
                purchase("RJ000001", "2026-01-01T00:00:00Z"),
                purchase("RJ000002", "2026-01-02T00:00:00Z"),
            ],
            works: vec![
                work(
                    "RJ000001",
                    "First Work",
                    "Maker One",
                    "2025-01-01T00:00:00Z",
                ),
                work(
                    "RJ000002",
                    "Second Work",
                    "Maker Two",
                    "2025-01-02T00:00:00Z",
                ),
            ],
            fail_at: None,
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("dm-library-{name}-{}-{unique}", std::process::id()));

        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_work_ids_in_flexible_folder_names() {
        assert_eq!(
            detect_work_ids_in_text("[RJ01005844] Soothing Voice")
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
            vec!["RJ01005844"]
        );
        assert_eq!(
            detect_work_ids_in_text("title rj123456 extra")
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
            vec!["RJ123456"]
        );
        assert_eq!(
            detect_work_ids_in_text("commercial VJ01001165 and book BJ123456")
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
            vec!["VJ01001165", "BJ123456"]
        );
    }

    #[test]
    fn work_id_detection_requires_boundaries_and_known_lengths() {
        for value in [
            "XRJ123456",
            "RJ123456A",
            "RJ12345",
            "RJ123456789",
            "AB123456",
        ] {
            assert_eq!(detect_work_ids_in_text(value), Vec::<WorkId>::new());
        }
    }

    #[test]
    fn work_id_detection_deduplicates_in_first_seen_order() {
        assert_eq!(
            detect_work_ids_in_text("RJ000001 / rj000001 / VJ000001")
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
            vec!["RJ000001", "VJ000001"]
        );
    }

    #[test]
    fn detects_unique_work_id_from_path_file_name() {
        let path = Path::new("/library/[RJ01005844] Soothing Voice");

        assert_eq!(
            detect_unique_work_id_in_path(path).map(|id| id.to_string()),
            Some("RJ01005844".to_owned())
        );
        assert_eq!(
            detect_work_ids_in_path("/library/no-id"),
            Vec::<WorkId>::new()
        );
        assert_eq!(
            detect_unique_work_id_in_path("/library/RJ000001 VJ000001"),
            None
        );
    }

    #[tokio::test]
    async fn saves_account_and_password_reference() -> Result<()> {
        let library = migrated_library().await?;

        let account = library.save_account(save_account_request(true)).await?;

        assert_eq!(account.id, "account-a");
        assert_eq!(
            account.credential_ref,
            Some("account:account-a:password".to_owned())
        );
        assert!(library.account_has_saved_password(&account)?);

        Ok(())
    }

    #[tokio::test]
    async fn save_account_preserves_enabled_state_for_existing_accounts() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library.set_account_enabled("account-a", false).await?;

        let account = library.save_account(save_account_request(true)).await?;
        assert!(
            !account.enabled,
            "saving details must not re-enable an account"
        );

        library.set_account_enabled("account-a", true).await?;
        let mut request = save_account_request(true);
        request.enabled = false;
        let account = library.save_account(request).await?;
        assert!(
            account.enabled,
            "only the dedicated toggle changes existing state"
        );
        Ok(())
    }

    #[tokio::test]
    async fn save_account_uses_initial_enabled_state_for_new_accounts() -> Result<()> {
        let library = migrated_library().await?;
        let account = library
            .save_account(SaveAccountRequest::new("New account"))
            .await?;
        assert!(account.enabled);

        let mut request = SaveAccountRequest::new("Initially disabled");
        request.enabled = false;
        assert!(!library.save_account(request).await?.enabled);
        Ok(())
    }

    #[tokio::test]
    async fn save_account_queued_after_disable_during_auth_keeps_account_disabled() -> Result<()> {
        use std::future::Future;
        use std::task::Poll;

        #[derive(Default)]
        struct GatedPrompt {
            entered: tokio::sync::Notify,
            release: tokio::sync::Notify,
        }

        #[async_trait]
        impl TwoFactorPrompt for GatedPrompt {
            async fn request_code(
                &self,
                _: TwoFactorPromptRequest,
            ) -> Result<TwoFactorPromptResponse> {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(TwoFactorPromptResponse::Code(
                    VALID_TWO_FACTOR_CODE.to_owned(),
                ))
            }
        }

        tokio::time::timeout(Duration::from_secs(2), async {
            let library = migrated_library().await?;
            let account = library.save_account(save_account_request(true)).await?;
            let source = TwoFactorSyncSource::new();
            let prompt = GatedPrompt::default();
            let mut authentication = Box::pin(library.authenticate_account(
                &account,
                None,
                Some(&prompt),
                None,
                &source,
            ));
            tokio::select! {
                result = &mut authentication => panic!("authentication finished before gate: {result:?}"),
                () = prompt.entered.notified() => {}
            }

            // Poll each mutation once while auth holds the lock, establishing FIFO order
            // without sleeps or assumptions about task scheduling.
            let mut disable = Box::pin(library.set_account_enabled(&account.id, false));
            std::future::poll_fn(|cx| {
                assert!(disable.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            let mut request = save_account_request(true);
            request.password = None;
            request.label = "Updated label".to_owned();
            assert!(request.enabled); // The adapter's creation default, not current state.
            let mut save = Box::pin(library.save_account(request));
            std::future::poll_fn(|cx| {
                assert!(save.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;

            prompt.release.notify_one();
            authentication.await?;
            let (disabled, saved) = tokio::join!(disable, save);
            disabled?;
            let saved = saved?;
            assert_eq!(saved.label, "Updated label");
            assert!(!saved.enabled, "queued save undid the completed disable");
            assert!(!library.find_account(&account.id).await?.enabled);
            Ok(())
        })
        .await
        .expect("gated account mutations must finish")
    }

    #[tokio::test]
    async fn removes_account_and_saved_credential() -> Result<()> {
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let library = Library::new(storage, credentials.clone());

        let account = library.save_account(save_account_request(true)).await?;
        let credential_ref = CredentialRef::account_password(&account.id)?;

        assert!(credentials.load_password(&credential_ref)?.is_some());

        let report = library.remove_account(&account.id).await?;

        assert_eq!(report.account_id, "account-a");
        assert_eq!(report.label, "Account A");
        assert!(report.credential_deleted);
        assert!(credentials.load_password(&credential_ref)?.is_none());
        assert_eq!(library.accounts().await?, Vec::new());

        Ok(())
    }

    #[tokio::test]
    async fn syncs_account_into_unified_product_cache() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        let sink = RecordingProgressSink::default();

        let report = library
            .sync_account_with_source(
                AccountSyncRequest {
                    account_id: "account-a",
                    password: None,
                    cancellation_token: None,
                    progress_sink: Some(&sink),
                    two_factor_prompt: None,
                },
                &sync_source(),
            )
            .await?;
        let page = library
            .list_products(&ProductListQuery {
                sort: ProductSort::TitleAsc,
                ..ProductListQuery::default()
            })
            .await?;
        let sync_runs = library.storage().sync_runs_for_account("account-a").await?;
        let events = sink.events.lock().expect("events lock");

        assert_eq!(report.purchased_count, 2);
        assert_eq!(report.cached_work_count, 2);
        assert_eq!(report.page_limit, Some(50));
        assert_eq!(page.total_count, 2);
        assert_eq!(page.products[0].work_id, "RJ000001");
        assert_eq!(
            page.products[0].thumbnail_url,
            Some("https://img.example.test/RJ000001/full.jpg".to_owned())
        );
        assert_eq!(page.products[0].owners[0].account_id, "account-a");
        assert_eq!(sync_runs.len(), 1);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Completed);
        assert!(matches!(events.first(), Some(SyncProgress::LoggingIn)));
        assert!(matches!(
            events.last(),
            Some(SyncProgress::Completed {
                cached_work_count: 2,
                ..
            })
        ));

        Ok(())
    }

    #[tokio::test]
    async fn downloads_owned_work_and_records_local_path() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("download-owned-work");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let sink = RecordingDownloadProgressSink::default();

        let report = library
            .download_work_with_source(
                WorkDownloadRequest {
                    progress_sink: Some(&sink),
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &download_root)
                },
                &FakeDownloadSource,
            )
            .await?;
        let page = library.list_products(&ProductListQuery::default()).await?;
        let product = page
            .products
            .iter()
            .find(|product| product.work_id == "RJ000001")
            .expect("downloaded work in product list");
        let events = sink.events.lock().expect("download events lock");

        assert_eq!(report.work_id, "RJ000001");
        assert_eq!(report.account_id, "account-a");
        assert_eq!(report.file_count, 1);
        assert_eq!(report.download_state.status, WorkDownloadStatus::Downloaded);
        assert!(library_root.join("RJ000001/RJ000001.txt").exists());
        assert!(!library_root
            .join(format!("RJ000001/{SERIAL_INFORMATION_FILE_NAME}"))
            .exists());
        assert!(!download_root.join("RJ000001").exists());
        assert_eq!(product.download.status, WorkDownloadStatus::Downloaded);
        assert_eq!(
            product.download.local_path,
            Some(
                library_root
                    .join("RJ000001")
                    .canonicalize()?
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(matches!(
            events.first(),
            Some(WorkDownloadProgress::LoggingIn)
        ));
        assert!(matches!(
            events.last(),
            Some(WorkDownloadProgress::Completed)
        ));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn downloads_serial_work_and_writes_serial_information_file() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("download-serial-work");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        let serial_path = library_root
            .join("RJ000001")
            .join(SERIAL_INFORMATION_FILE_NAME);
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let report = library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &SerialDownloadSource,
            )
            .await?;
        let serial_content = tokio::fs::read_to_string(&serial_path).await?;

        assert_eq!(report.download_state.status, WorkDownloadStatus::Downloaded);
        assert!(serial_content.starts_with(SERIAL_INFORMATION_MARKER));
        assert!(serial_content.contains("Work ID: RJ000001"));
        assert!(serial_content.contains("Serial number"));
        assert!(serial_content.contains("ABC-123-SECRET"));
        assert!(!download_root.join("RJ000001").exists());

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn download_cancellation_reaches_download_source_without_progress() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("download-cancellation-forwarding");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        let source = WaitingDownloadSource::new();
        let cancellation = CancellationToken::new();
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let result = tokio::time::timeout(Duration::from_secs(2), async {
            let download = library.download_work_with_source(
                WorkDownloadRequest {
                    cancellation_token: Some(&cancellation),
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &download_root)
                },
                &source,
            );
            let cancel = async {
                while !source.entered_download() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }

                cancellation.cancel();
            };
            let (result, ()) = tokio::join!(download, cancel);
            result
        })
        .await
        .expect("download cancellation should complete promptly");
        let err = match result {
            Ok(_) => panic!("download should have been cancelled"),
            Err(error) => error,
        };

        assert!(matches!(
            err,
            LibraryError::Cancelled | LibraryError::Download(dm_download::DownloadError::Cancelled)
        ));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn removes_downloaded_work_and_clears_state() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("remove-downloaded-work");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &FakeDownloadSource,
            )
            .await?;

        let state = library
            .remove_work_download(WorkDownloadRemovalRequest::new(
                "RJ000001",
                &library_root,
                &download_root,
            ))
            .await?;

        assert_eq!(state.status, WorkDownloadStatus::NotDownloaded);
        assert!(!library_root.join("RJ000001").exists());
        assert!(!download_root.join("RJ000001").exists());

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn marks_existing_library_folder_as_downloaded() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("mark-downloaded-work");
        let library_root = root.join("library");
        let local_path = library_root.join("manual").join("RJ000001");
        std::fs::create_dir_all(&local_path).unwrap();
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let state = library
            .mark_work_downloaded(WorkDownloadMarkRequest::new(
                "RJ000001",
                &library_root,
                &local_path,
            ))
            .await?;

        assert_eq!(state.status, WorkDownloadStatus::Downloaded);
        assert_eq!(
            state.local_path,
            Some(
                local_path
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert_eq!(state.staging_path, None);
        assert_eq!(state.unpack_policy, Some("manual".to_owned()));

        let page = library.list_products(&ProductListQuery::default()).await?;
        let product = page
            .products
            .iter()
            .find(|product| product.work_id == "RJ000001")
            .expect("marked work in product list");
        assert_eq!(product.download.status, WorkDownloadStatus::Downloaded);

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn rejects_manual_download_folder_outside_library_root() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("mark-downloaded-outside-root");
        let library_root = root.join("library");
        let outside_path = root.join("outside").join("RJ000001");
        std::fs::create_dir_all(&library_root).unwrap();
        std::fs::create_dir_all(&outside_path).unwrap();
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let err = library
            .mark_work_downloaded(WorkDownloadMarkRequest::new(
                "RJ000001",
                &library_root,
                &outside_path,
            ))
            .await
            .unwrap_err();

        assert!(matches!(err, LibraryError::DownloadPathOutsideRoots(_)));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn imports_local_work_folders_as_downloaded_products() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("import-local-work-folders");
        let library_root = root.join("library");
        let local_path = library_root.join("[RJ123456] Local Work");
        std::fs::create_dir_all(&local_path).unwrap();
        std::fs::create_dir_all(library_root.join("No Work Id")).unwrap();
        std::fs::create_dir_all(library_root.join("RJ000001 VJ000001 ambiguous")).unwrap();

        let report = library
            .import_local_work_downloads(LocalWorkImportRequest::new(&library_root))
            .await?;
        let page = library.list_products(&ProductListQuery::default()).await?;

        assert_eq!(report.scanned_directories, 3);
        assert_eq!(report.imported_count, 1);
        assert_eq!(report.skipped_no_id, 1);
        assert_eq!(report.skipped_ambiguous, 1);
        assert_eq!(report.skipped_existing, 0);
        assert_eq!(report.imported_works[0].work_id, "RJ123456");
        assert_eq!(
            report.imported_works[0].local_path,
            local_path.canonicalize().unwrap()
        );

        assert_eq!(page.total_count, 1);
        assert_eq!(page.products[0].work_id, "RJ123456");
        assert_eq!(page.products[0].title, "[RJ123456] Local Work");
        assert_eq!(page.products[0].owners[0].label, "Local");
        assert_eq!(
            page.products[0].download.status,
            WorkDownloadStatus::Downloaded
        );
        assert_eq!(
            page.products[0].download.local_path,
            Some(
                local_path
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn imports_local_work_folders_with_public_metadata() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("import-local-work-folders-metadata");
        let library_root = root.join("library");
        let local_path = library_root.join("[RJ123456] Local Work");
        let source = FakeLocalWorkMetadataSource {
            works: vec![public_work(
                "RJ123456",
                "Public Metadata Work",
                "Public Maker",
            )],
            fail: false,
        };
        std::fs::create_dir_all(&local_path).unwrap();

        let report = library
            .import_local_work_downloads_with_metadata_source(
                LocalWorkImportRequest::new(&library_root),
                &source,
            )
            .await?;
        let page = library.list_products(&ProductListQuery::default()).await?;

        assert_eq!(report.imported_count, 1);
        assert_eq!(report.metadata_candidate_count, 1);
        assert_eq!(report.metadata_updated_count, 1);
        assert_eq!(report.metadata_missing_count, 0);
        assert_eq!(report.metadata_error, None);
        assert_eq!(page.products[0].work_id, "RJ123456");
        assert_eq!(page.products[0].title, "Public Metadata Work");
        assert_eq!(page.products[0].maker_name.as_deref(), Some("Public Maker"));
        assert_eq!(page.products[0].work_type.as_deref(), Some("SOU"));
        assert_eq!(page.products[0].age_category.as_deref(), Some("r18"));
        assert_eq!(
            page.products[0].thumbnail_url.as_deref(),
            Some("https://img.example.test/RJ123456/main.jpg")
        );
        assert_eq!(page.products[0].content_size_bytes, Some(2048));
        assert_eq!(page.products[0].credit_groups[0].names, vec!["Voice One"]);
        assert_eq!(
            page.products[0].download.local_path,
            Some(
                local_path
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn local_work_import_hydrates_existing_placeholder_metadata() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("import-local-work-existing-metadata");
        let library_root = root.join("library");
        let local_path = library_root.join("[RJ123456] Local Work");
        std::fs::create_dir_all(&local_path).unwrap();

        library
            .import_local_work_downloads(LocalWorkImportRequest::new(&library_root))
            .await?;

        let source = FakeLocalWorkMetadataSource {
            works: vec![public_work(
                "RJ123456",
                "Public Metadata Work",
                "Public Maker",
            )],
            fail: false,
        };
        let report = library
            .import_local_work_downloads_with_metadata_source(
                LocalWorkImportRequest::new(&library_root),
                &source,
            )
            .await?;
        let page = library.list_products(&ProductListQuery::default()).await?;

        assert_eq!(report.imported_count, 0);
        assert_eq!(report.skipped_existing, 1);
        assert_eq!(report.metadata_candidate_count, 1);
        assert_eq!(report.metadata_updated_count, 1);
        assert_eq!(page.products[0].title, "Public Metadata Work");
        assert_eq!(page.products[0].maker_name.as_deref(), Some("Public Maker"));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn local_work_import_skips_existing_download_records() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("import-local-skips-existing");
        let library_root = root.join("library");
        let local_path = library_root.join("[RJ000001] Local Duplicate");
        std::fs::create_dir_all(&local_path).unwrap();
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .mark_work_downloaded(WorkDownloadMarkRequest::new(
                "RJ000001",
                &library_root,
                &local_path,
            ))
            .await?;

        let report = library
            .import_local_work_downloads(LocalWorkImportRequest::new(&library_root))
            .await?;

        assert_eq!(report.scanned_directories, 1);
        assert_eq!(report.imported_count, 0);
        assert_eq!(report.skipped_existing, 1);
        assert!(report.imported_works.is_empty());

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn redownload_replaces_existing_local_work_after_staging() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("redownload-replaces-work");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        let local_file = library_root.join("RJ000001/RJ000001.txt");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &FakeDownloadSource,
            )
            .await?;
        tokio::fs::write(&local_file, b"user edit").await?;

        let report = library
            .download_work_with_source(
                WorkDownloadRequest {
                    replace_existing: true,
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &download_root)
                },
                &FakeDownloadSource,
            )
            .await?;

        assert_eq!(report.download_state.status, WorkDownloadStatus::Downloaded);
        assert_eq!(tokio::fs::read(&local_file).await?, b"downloaded");
        assert!(!download_root.join("RJ000001").exists());

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn bulk_download_skips_downloaded_works() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("bulk-download-skips");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &FakeDownloadSource,
            )
            .await?;
        let sink = RecordingBulkDownloadProgressSink::default();

        let report = library
            .download_products_with_source(
                BulkWorkDownloadRequest {
                    progress_sink: Some(&sink),
                    ..BulkWorkDownloadRequest::new(
                        ProductListQuery::default(),
                        &library_root,
                        &download_root,
                    )
                },
                &FakeDownloadSource,
            )
            .await?;
        let page = library.list_products(&ProductListQuery::default()).await?;
        let events = sink.events.lock().expect("bulk events lock");

        assert_eq!(report.total_count, 2);
        assert_eq!(report.requested_count, 1);
        assert_eq!(report.skipped_downloaded_count, 1);
        assert_eq!(report.succeeded_count, 1);
        assert_eq!(report.failed_count, 0);
        assert!(library_root.join("RJ000002/RJ000001.txt").exists());
        assert!(page
            .products
            .iter()
            .all(|product| product.download.status == WorkDownloadStatus::Downloaded));
        assert!(matches!(
            events.first(),
            Some(BulkWorkDownloadProgress::Selecting)
        ));
        assert!(matches!(
            events.last(),
            Some(BulkWorkDownloadProgress::Completed { .. })
        ));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn bulk_download_preview_reports_expected_bytes() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("bulk-preview");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &FakeDownloadSource,
            )
            .await?;

        let preview = library
            .preview_download_products_with_source(
                BulkWorkDownloadPreviewRequest {
                    query: ProductListQuery::default(),
                    work_ids: None,
                    skip_downloaded: true,
                    cancellation_token: None,
                    progress_sink: None,
                },
                &FakeDownloadSource,
            )
            .await?;

        assert_eq!(preview.total_count, 2);
        assert_eq!(preview.requested_count, 1);
        assert_eq!(preview.skipped_downloaded_count, 1);
        assert_eq!(preview.planned_count, 1);
        assert_eq!(preview.failed_count, 0);
        assert_eq!(preview.known_expected_bytes, 10);
        assert_eq!(preview.total_expected_bytes, Some(10));
        assert_eq!(preview.unknown_size_count, 0);
        assert_eq!(preview.works[0].work_id, "RJ000002");
        assert_eq!(preview.works[0].file_count, 1);

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn bulk_download_preview_reports_progress() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("bulk-preview-progress");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &download_root),
                &FakeDownloadSource,
            )
            .await?;
        let sink = RecordingBulkDownloadPreviewProgressSink::default();

        let preview = library
            .preview_download_products_with_source(
                BulkWorkDownloadPreviewRequest {
                    query: ProductListQuery::default(),
                    work_ids: None,
                    skip_downloaded: true,
                    cancellation_token: None,
                    progress_sink: Some(&sink),
                },
                &FakeDownloadSource,
            )
            .await?;
        let events = sink.events.lock().expect("preview events lock");

        assert_eq!(preview.requested_count, 1);
        assert!(matches!(
            events.first(),
            Some(BulkWorkDownloadPreviewProgress::Selecting)
        ));
        assert!(matches!(
            events.get(1),
            Some(BulkWorkDownloadPreviewProgress::Selected {
                requested_count: 1,
                ..
            })
        ));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                BulkWorkDownloadPreviewProgress::WorkStarted {
                    work_id,
                    current: 1,
                    total: 1
                } if work_id == "RJ000002"
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                BulkWorkDownloadPreviewProgress::WorkPlanned {
                    work_id,
                    current: 1,
                    total: 1,
                    known_expected_bytes: 10,
                    total_expected_bytes: Some(10),
                    unknown_size_count: 0
                } if work_id == "RJ000002"
            )
        }));
        assert!(matches!(
            events.last(),
            Some(BulkWorkDownloadPreviewProgress::Completed {
                planned_count: 1,
                failed_count: 0,
                ..
            })
        ));

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn bulk_download_limits_to_supplied_work_ids() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("bulk-download-limited");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let report = library
            .download_products_with_source(
                BulkWorkDownloadRequest {
                    work_ids: Some(vec!["RJ000002".to_owned()]),
                    ..BulkWorkDownloadRequest::new(
                        ProductListQuery::default(),
                        &library_root,
                        &download_root,
                    )
                },
                &FakeDownloadSource,
            )
            .await?;
        let first = library.storage().work_download_state("RJ000001").await?;
        let second = library.storage().work_download_state("RJ000002").await?;

        assert_eq!(report.total_count, 2);
        assert_eq!(report.requested_count, 1);
        assert_eq!(report.succeeded_count, 1);
        assert_eq!(first.status, WorkDownloadStatus::NotDownloaded);
        assert_eq!(second.status, WorkDownloadStatus::Downloaded);

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn bulk_download_continues_after_work_failure() -> Result<()> {
        let library = migrated_library().await?;
        let root = test_dir("bulk-download-failure");
        let library_root = root.join("library");
        let download_root = root.join("downloads");
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;

        let report = library
            .download_products_with_source(
                BulkWorkDownloadRequest::new(
                    ProductListQuery::default(),
                    &library_root,
                    &download_root,
                ),
                &FailingDownloadSource {
                    fail_work_id: "RJ000002",
                },
            )
            .await?;
        let failed_state = library.storage().work_download_state("RJ000002").await?;

        assert_eq!(report.requested_count, 2);
        assert_eq!(report.succeeded_count, 1);
        assert_eq!(report.failed_count, 1);
        assert_eq!(report.failed_works[0].work_id, "RJ000002");
        assert_eq!(failed_state.status, WorkDownloadStatus::Failed);
        assert!(library_root.join("RJ000001/RJ000001.txt").exists());

        std::fs::remove_dir_all(root).unwrap();

        Ok(())
    }

    #[test]
    fn download_path_root_check_rejects_root_and_sibling_prefixes() {
        let root = PathBuf::from("/tmp/dlsite/library");

        assert!(path_is_download_child_of_any_root(
            &root.join("RJ000001"),
            std::slice::from_ref(&root)
        ));
        assert!(!path_is_download_child_of_any_root(
            &root,
            std::slice::from_ref(&root)
        ));
        assert!(!path_is_download_child_of_any_root(
            &PathBuf::from("/tmp/dlsite/library-other/RJ000001"),
            &[root]
        ));
    }

    #[tokio::test]
    async fn sync_can_use_one_shot_password_without_remembering() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(false)).await?;

        let report = library
            .sync_account_with_source(
                AccountSyncRequest {
                    account_id: "account-a",
                    password: Some("secret"),
                    cancellation_token: None,
                    progress_sink: None,
                    two_factor_prompt: None,
                },
                &sync_source(),
            )
            .await?;

        assert_eq!(report.cached_work_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn cancelled_two_factor_download_records_cancellation_and_stops_bulk() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let root = test_dir("two-factor-cancel-download");
        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::with_responses([TwoFactorPromptResponse::Cancelled]);
        let error = library
            .download_products_with_source(
                BulkWorkDownloadRequest {
                    two_factor_prompt: Some(&prompt),
                    ..BulkWorkDownloadRequest::new(
                        ProductListQuery::default(),
                        &root.join("library"),
                        &root.join("staging"),
                    )
                },
                &source,
            )
            .await
            .expect_err("cancelled bulk must stop");
        assert_eq!(error.failure_code(), "cancelled");
        assert_eq!(prompt.requests().len(), 1);
        let page = library.list_products(&ProductListQuery::default()).await?;
        assert_eq!(
            page.products
                .iter()
                .filter(|p| p.download.status == WorkDownloadStatus::Cancelled)
                .count(),
            1
        );
        assert_eq!(
            page.products
                .iter()
                .filter(|p| p.download.status == WorkDownloadStatus::NotDownloaded)
                .count(),
            1
        );
        assert_eq!(std::fs::read_dir(root.join("library"))?.count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn authentication_http_waits_cancel_and_release_account_lock() -> Result<()> {
        struct PendingSource {
            phase: &'static str,
            entered: tokio::sync::Notify,
        }
        impl PendingSource {
            async fn wait(&self, phase: &str) {
                if self.phase == phase {
                    self.entered.notify_one();
                    std::future::pending::<()>().await;
                }
            }
        }
        #[async_trait]
        impl DlsiteAuthSource for PendingSource {
            async fn restore_session(&self, _: &SessionSnapshot) -> Result<()> {
                self.wait("restore").await;
                Ok(())
            }
            async fn validate_session(&self) -> Result<bool> {
                self.wait("validate").await;
                Ok(false)
            }
            async fn begin_login(&self, _: &Credentials) -> Result<LoginOutcome> {
                self.wait("login").await;
                Ok(LoginOutcome::TwoFactorRequired(
                    TwoFactorSyncSource::challenge(),
                ))
            }
            async fn complete_two_factor(
                &self,
                _: &TwoFactorChallenge,
                _: &str,
            ) -> Result<SessionSnapshot> {
                self.wait("submit").await;
                panic!("unexpected code submission")
            }
        }
        for phase in ["restore", "validate", "login", "submit"] {
            let library = migrated_library().await?;
            let account = library.save_account(save_account_request(true)).await?;
            let session_ref = CredentialRef::account_session(&account.id)?;
            library
                .credentials
                .save_password(&session_ref, "old-session")?;
            let token = CancellationToken::new();
            let source = PendingSource {
                phase,
                entered: tokio::sync::Notify::new(),
            };
            let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);
            let result = tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(
                    library.authenticate_account(
                        &account,
                        None,
                        Some(&prompt),
                        Some(&token),
                        &source
                    ),
                    async {
                        source.entered.notified().await;
                        token.cancel();
                    }
                )
                .0
            })
            .await
            .expect("authentication must observe cancellation");
            assert!(result
                .expect_err("pending request must cancel")
                .is_cancelled());
            assert!(library.login_lock(&account.id).try_lock().is_ok());
            assert!(prompt.requests().len() <= 1);
            if matches!(phase, "restore" | "validate") {
                assert_eq!(
                    library.credentials.load_password(&session_ref)?.as_deref(),
                    Some("old-session")
                );
            } else {
                assert!(library.credentials.load_password(&session_ref)?.is_none());
            }
            library
                .authenticate_account(&account, None, None, None, &sync_source())
                .await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn bulk_token_cancellation_keeps_completed_report() -> Result<()> {
        struct CancelAfterFirst(CancellationToken);
        impl BulkWorkDownloadProgressSink for CancelAfterFirst {
            fn emit(&self, progress: BulkWorkDownloadProgress) {
                if matches!(progress, BulkWorkDownloadProgress::WorkCompleted { .. }) {
                    self.0.cancel();
                }
            }
        }
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let root = test_dir("bulk-partial-cancel");
        let token = CancellationToken::new();
        let sink = CancelAfterFirst(token.clone());
        let result = library
            .download_products_with_source(
                BulkWorkDownloadRequest {
                    cancellation_token: Some(&token),
                    progress_sink: Some(&sink),
                    ..BulkWorkDownloadRequest::new(
                        ProductListQuery::default(),
                        &root.join("library"),
                        &root.join("staging"),
                    )
                },
                &FakeDownloadSource,
            )
            .await;
        let LibraryError::BulkInterrupted {
            report,
            interrupted_work,
            ..
        } = result.unwrap_err()
        else {
            panic!("partial report required")
        };
        assert_eq!(report.succeeded_count, 1);
        assert_eq!(report.failed_count, 0);
        assert!(interrupted_work.is_none());
        assert!(report.succeeded_works[0].local_path.is_dir());
        Ok(())
    }

    #[tokio::test]
    async fn replacement_db_failure_restores_old_bytes_and_complete_record() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let root = test_dir("replacement-db-rollback");
        let library_root = root.join("library");
        let staging_root = root.join("staging");
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &staging_root),
                &FakeDownloadSource,
            )
            .await?;
        tokio::fs::write(library_root.join("RJ000001/RJ000001.txt"), b"old bytes!").await?;
        let previous = library.storage.work_download_state("RJ000001").await?;
        let mut tx = library.storage.begin_write().await?;
        tx.execute("CREATE TRIGGER reject_download BEFORE UPDATE ON work_downloads BEGIN SELECT RAISE(ABORT, 'injected DB failure'); END").await?;
        tx.commit().await?;
        let error = library
            .download_work_with_source(
                WorkDownloadRequest {
                    replace_existing: true,
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &staging_root)
                },
                &FakeDownloadSource,
            )
            .await
            .unwrap_err();
        assert_eq!(error.failure_code(), "storage");
        assert_eq!(
            library.storage.work_download_state("RJ000001").await?,
            previous
        );
        assert_eq!(
            tokio::fs::read(library_root.join("RJ000001/RJ000001.txt")).await?,
            b"old bytes!"
        );
        assert!(staging_root.join("RJ000001/RJ000001.txt").is_file());
        assert!(library
            .storage
            .download_finalization("RJ000001")
            .await?
            .is_none());
        Ok(())
    }

    #[tokio::test]
    async fn replacement_prompt_cancel_preserves_download_and_concurrent_consumers_share_login(
    ) -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let root = test_dir("replacement-auth-cancel");
        let library_root = root.join("library");
        let staging = root.join("staging");
        library
            .download_work_with_source(
                WorkDownloadRequest::new("RJ000001", &library_root, &staging),
                &FakeDownloadSource,
            )
            .await?;
        let previous = library.storage.work_download_state("RJ000001").await?;
        let source = TwoFactorSyncSource::new();
        let cancel = ScriptedTwoFactorPrompt::with_responses([TwoFactorPromptResponse::Cancelled]);
        let result = library
            .download_work_with_source(
                WorkDownloadRequest {
                    replace_existing: true,
                    two_factor_prompt: Some(&cancel),
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &staging)
                },
                &source,
            )
            .await;
        assert!(result.unwrap_err().is_cancelled());
        assert_eq!(
            library.storage.work_download_state("RJ000001").await?,
            previous
        );
        assert_eq!(
            std::fs::read(library_root.join("RJ000001/RJ000001.txt"))?,
            b"downloaded"
        );

        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);
        let source = TwoFactorSyncSource::new();
        let (sync, download) = tokio::join!(
            library.sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source
            ),
            library.download_work_with_source(
                WorkDownloadRequest {
                    two_factor_prompt: Some(&prompt),
                    ..WorkDownloadRequest::new("RJ000002", &library_root, &staging)
                },
                &source
            )
        );
        sync?;
        download?;
        assert_eq!(source.state().login_count, 1);
        assert_eq!(prompt.requests().len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn exhausted_two_factor_retries_remain_failed() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new(["000000", "000000", "000000"]);
        let error = library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await
            .unwrap_err();
        assert!(!error.is_cancelled());
        assert_eq!(error.failure_code(), "two_factor_rejected");
        assert_eq!(
            library.storage.sync_runs_for_account("account-a").await?[0].status,
            dm_storage::SyncRunStatus::Failed
        );
        assert_eq!(prompt.requests().len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn replacement_rejects_staging_overlap_with_manual_content_before_deletion() -> Result<()>
    {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let root = test_dir("manual-staging-overlap");
        let library_root = root.join("library");
        let staging = library_root.join("staging");
        let old = staging.join("RJ000001");
        std::fs::create_dir_all(&old)?;
        std::fs::write(old.join("content"), b"keep")?;
        library
            .mark_work_downloaded(WorkDownloadMarkRequest::new(
                "RJ000001",
                &library_root,
                &old,
            ))
            .await?;
        let previous = library.storage.work_download_state("RJ000001").await?;
        let result = library
            .download_work_with_source(
                WorkDownloadRequest {
                    replace_existing: true,
                    ..WorkDownloadRequest::new("RJ000001", &library_root, &staging)
                },
                &FakeDownloadSource,
            )
            .await;
        assert!(matches!(
            result,
            Err(LibraryError::DownloadPathOutsideRoots(_))
        ));
        assert_eq!(std::fs::read(old.join("content"))?, b"keep");
        assert_eq!(
            library.storage.work_download_state("RJ000001").await?,
            previous
        );
        Ok(())
    }

    #[tokio::test]
    async fn manual_replacement_interrupted_before_intent_tracks_staging_for_later_delete(
    ) -> Result<()> {
        struct InterruptedTransfer {
            cancel: bool,
        }
        impl_fake_auth_source!(InterruptedTransfer);
        #[async_trait]
        impl WorkDownloadSource for InterruptedTransfer {
            async fn download_plan(&self, id: &WorkId) -> Result<DownloadPlan> {
                FakeDownloadSource.download_plan(id).await
            }
            async fn download_file_metadata(
                &self,
                index: usize,
                file: &DownloadFile,
            ) -> Result<DownloadFileMetadata> {
                FakeDownloadSource.download_file_metadata(index, file).await
            }
            async fn download_files(
                &self,
                job: &DownloadJobRequest,
                plan: &DownloadPlan,
                cancellation: &dm_download::CancellationToken,
                sink: &mut (dyn FnMut(DownloadProgress) + Send),
            ) -> Result<DownloadedWork> {
                FakeDownloadSource
                    .download_files(job, plan, cancellation, sink)
                    .await?;
                if self.cancel {
                    Err(LibraryError::Download(
                        dm_download::DownloadError::Cancelled,
                    ))
                } else {
                    Err(LibraryError::SyncSource(
                        "injected transfer failure".to_owned(),
                    ))
                }
            }
        }
        for cancel in [false, true] {
            let root = test_dir("manual-transfer-staging");
            std::fs::create_dir_all(&root)?;
            let storage_path = root.join("library.sqlite");
            let storage = Storage::open(&storage_path).await?;
            storage.run_migrations().await?;
            let library = Library::new(storage, Arc::new(InMemoryCredentialStore::new()));
            library.save_account(save_account_request(true)).await?;
            library
                .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
                .await?;
            let library_root = root.join("library");
            let staging_root = root.join("staging");
            let old = library_root.join("RJ000001");
            std::fs::create_dir_all(&old)?;
            std::fs::write(old.join("content"), b"manual old")?;
            let mut previous = library
                .mark_work_downloaded(WorkDownloadMarkRequest::new(
                    "RJ000001",
                    &library_root,
                    &old,
                ))
                .await?;
            assert!(previous.staging_path.is_none());
            let error = library
                .download_work_with_source(
                    WorkDownloadRequest {
                        replace_existing: true,
                        ..WorkDownloadRequest::new("RJ000001", &library_root, &staging_root)
                    },
                    &InterruptedTransfer { cancel },
                )
                .await
                .unwrap_err();
            assert_eq!(error.is_cancelled(), cancel);
            assert!(library
                .storage
                .download_finalization("RJ000001")
                .await?
                .is_none());
            assert_eq!(std::fs::read(old.join("content"))?, b"manual old");
            let staging = staging_root.join("RJ000001").canonicalize()?;
            previous.staging_path = Some(staging.to_string_lossy().into_owned());
            assert_eq!(
                library.storage.work_download_state("RJ000001").await?,
                previous
            );
            drop(library);
            let reopened = Library::new(
                Storage::open(&storage_path).await?,
                Arc::new(InMemoryCredentialStore::new()),
            );
            reopened
                .remove_work_download(WorkDownloadRemovalRequest::new(
                    "RJ000001",
                    &library_root,
                    &staging_root,
                ))
                .await?;
            assert!(!staging.exists());
            assert!(!old.exists());
        }
        Ok(())
    }

    #[tokio::test]
    async fn cancelling_replacement_during_transfer_or_preparation_keeps_old_state() -> Result<()> {
        struct CancelProgress {
            token: CancellationToken,
            at_finalization: bool,
        }
        impl WorkDownloadProgressSink for CancelProgress {
            fn emit(&self, progress: WorkDownloadProgress) {
                if (self.at_finalization && matches!(progress, WorkDownloadProgress::Finalizing))
                    || (!self.at_finalization
                        && matches!(progress, WorkDownloadProgress::Download(_)))
                {
                    self.token.cancel();
                }
            }
        }
        for at_finalization in [false, true] {
            let library = migrated_library().await?;
            library.save_account(save_account_request(true)).await?;
            library
                .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
                .await?;
            let root = test_dir("replacement-cancel-boundary");
            let library_root = root.join("library");
            let staging = root.join("staging");
            library
                .download_work_with_source(
                    WorkDownloadRequest::new("RJ000001", &library_root, &staging),
                    &FakeDownloadSource,
                )
                .await?;
            std::fs::write(library_root.join("RJ000001/RJ000001.txt"), b"old content")?;
            let old = library.storage.work_download_state("RJ000001").await?;
            let token = CancellationToken::new();
            let sink = CancelProgress {
                token: token.clone(),
                at_finalization,
            };
            let error = library
                .download_work_with_source(
                    WorkDownloadRequest {
                        replace_existing: true,
                        cancellation_token: Some(&token),
                        progress_sink: Some(&sink),
                        ..WorkDownloadRequest::new("RJ000001", &library_root, &staging)
                    },
                    &FakeDownloadSource,
                )
                .await
                .unwrap_err();
            assert!(error.is_cancelled());
            assert_eq!(library.storage.work_download_state("RJ000001").await?, old);
            assert_eq!(
                std::fs::read(library_root.join("RJ000001/RJ000001.txt"))?,
                b"old content"
            );
            assert!(staging.join("RJ000001/RJ000001.txt").is_file());
        }
        Ok(())
    }

    #[tokio::test]
    async fn session_restore_distinguishes_invalid_data_from_validation_network_errors(
    ) -> Result<()> {
        struct Probe {
            outcome: &'static str,
            logins: std::sync::atomic::AtomicUsize,
        }
        #[async_trait]
        impl DlsiteAuthSource for Probe {
            async fn restore_session(&self, _: &SessionSnapshot) -> Result<()> {
                if self.outcome == "malformed" {
                    return Err(LibraryError::Json(
                        serde_json::from_str::<Value>("invalid").unwrap_err(),
                    ));
                }
                Ok(())
            }
            async fn validate_session(&self) -> Result<bool> {
                if self.outcome == "network" {
                    return Err(LibraryError::SyncSource("offline".to_owned()));
                }
                Ok(self.outcome == "valid")
            }
            async fn begin_login(&self, _: &Credentials) -> Result<LoginOutcome> {
                self.logins
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(LoginOutcome::Authorized(SessionSnapshot {
                    cookies_json: "fresh".to_owned(),
                }))
            }
            async fn complete_two_factor(
                &self,
                _: &TwoFactorChallenge,
                _: &str,
            ) -> Result<SessionSnapshot> {
                unreachable!()
            }
        }
        for outcome in ["valid", "expired", "malformed", "network"] {
            let library = migrated_library().await?;
            let account = library.save_account(save_account_request(true)).await?;
            let session_ref = CredentialRef::account_session(&account.id)?;
            library.credentials.save_password(&session_ref, "stored")?;
            // New Library instance, same durable stores; no process-local auth state reused.
            let library = Library::new(library.storage.clone(), library.credentials.clone());
            let source = Probe {
                outcome,
                logins: std::sync::atomic::AtomicUsize::new(0),
            };
            let result = library
                .authenticate_account(&account, None, None, None, &source)
                .await;
            let should_login = matches!(outcome, "expired" | "malformed");
            assert_eq!(result.is_err(), outcome == "network");
            assert_eq!(
                source.logins.load(std::sync::atomic::Ordering::SeqCst),
                usize::from(should_login)
            );
            assert_eq!(
                library.credentials.load_password(&session_ref)?.as_deref(),
                Some(if should_login { "fresh" } else { "stored" })
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn two_factor_sync_completes_with_a_prompted_code() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);
        let report = library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await?;

        assert_eq!(report.cached_work_count, 2);
        assert_eq!(source.state().submitted_codes, vec![VALID_TWO_FACTOR_CODE]);

        let requests = prompt.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].account_id, "account-a");
        assert_eq!(requests[0].attempt, 1);
        assert!(!requests[0].previous_code_rejected);

        Ok(())
    }

    #[tokio::test]
    async fn rejected_two_factor_code_is_prompted_again() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new(["000000", VALID_TWO_FACTOR_CODE]);

        library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await?;

        assert_eq!(
            source.state().submitted_codes,
            vec!["000000", VALID_TWO_FACTOR_CODE]
        );

        let requests = prompt.requests();
        assert_eq!(requests.len(), 2);
        assert!(!requests[0].previous_code_rejected);
        assert_eq!(requests[1].attempt, 2);
        assert!(requests[1].previous_code_rejected);

        Ok(())
    }

    #[tokio::test]
    async fn cancelled_two_factor_prompt_reports_a_cancelled_sync() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::with_responses([TwoFactorPromptResponse::Cancelled]);
        let result = library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await;

        let error = result.expect_err("cancelling the prompt must not sync");

        assert!(matches!(
            &error,
            LibraryError::TwoFactorCancelled(account_id) if account_id == "account-a"
        ));
        // `dm-jobs` recognizes only this exact code as a cancellation, so the job it belongs
        // to finishes as Cancelled rather than Failed.
        assert_eq!(error.failure_code(), "cancelled");
        assert_eq!(
            error.support_details()["failureKind"],
            "two_factor_cancelled"
        );
        assert!(source.state().submitted_codes.is_empty());

        let runs = library.storage().sync_runs_for_account("account-a").await?;
        assert_eq!(runs[0].status, dm_storage::SyncRunStatus::Cancelled);

        Ok(())
    }

    #[tokio::test]
    async fn timed_out_two_factor_prompt_fails_sync() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::with_responses([TwoFactorPromptResponse::TimedOut]);
        let result = library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await;

        let error = result.expect_err("an unanswered prompt must not sync");

        assert!(matches!(
            &error,
            LibraryError::TwoFactorTimedOut(account_id) if account_id == "account-a"
        ));
        // Unlike cancellation, nobody chose this outcome, so it stays a visible failure.
        assert_eq!(error.failure_code(), "two_factor_timed_out");
        assert!(source.state().submitted_codes.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn cancelling_a_job_waiting_on_the_login_lock_does_not_wait_for_the_dialog() -> Result<()>
    {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        // Hold the account's login lock the way a job sitting on an open dialog would.
        let login_lock = library.login_lock("account-a");
        let held = login_lock.lock().await;

        let cancellation = CancellationToken::new();
        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);
        let queued = library.sync_account_with_source(
            AccountSyncRequest {
                two_factor_prompt: Some(&prompt),
                cancellation_token: Some(&cancellation),
                ..AccountSyncRequest::new("account-a")
            },
            &source,
        );

        let result = tokio::join!(queued, async {
            // Let the queued sync reach the lock before cancelling it.
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancellation.cancel();
        })
        .0;

        assert!(matches!(result, Err(LibraryError::Cancelled)));
        // The lock was never released, so the queued job gave up on its own.
        assert!(prompt.requests().is_empty());
        drop(held);

        Ok(())
    }

    #[tokio::test]
    async fn two_factor_challenge_without_a_prompt_is_reported() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let result = library
            .sync_account_with_source(
                AccountSyncRequest::new("account-a"),
                &TwoFactorSyncSource::new(),
            )
            .await;

        assert!(matches!(
            result,
            Err(LibraryError::TwoFactorPromptUnavailable(account_id)) if account_id == "account-a"
        ));

        Ok(())
    }

    #[tokio::test]
    async fn stored_session_skips_the_two_factor_prompt() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);
        let request = || AccountSyncRequest {
            two_factor_prompt: Some(&prompt),
            ..AccountSyncRequest::new("account-a")
        };

        library.sync_account_with_source(request(), &source).await?;
        library.sync_account_with_source(request(), &source).await?;

        // The second sync reused the stored session instead of logging in again.
        assert_eq!(prompt.requests().len(), 1);
        assert_eq!(source.state().login_count, 1);
        assert_eq!(
            source.state().restored_session.as_deref(),
            Some(TWO_FACTOR_SESSION)
        );

        Ok(())
    }

    #[tokio::test]
    async fn session_is_not_stored_when_the_account_does_not_remember_its_password() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(false)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE, VALID_TWO_FACTOR_CODE]);
        let request = || AccountSyncRequest {
            password: Some("secret"),
            two_factor_prompt: Some(&prompt),
            ..AccountSyncRequest::new("account-a")
        };

        library.sync_account_with_source(request(), &source).await?;

        // The session cookie is as good as the password, so an account that keeps neither
        // leaves nothing behind for the next job to reuse.
        let session_ref = CredentialRef::account_session("account-a")?;

        assert!(library.credentials.load_password(&session_ref)?.is_none());

        library.sync_account_with_source(request(), &source).await?;

        assert_eq!(prompt.requests().len(), 2);
        assert_eq!(source.state().login_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn session_is_stored_when_the_account_remembers_its_password() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE]);

        library
            .sync_account_with_source(
                AccountSyncRequest {
                    two_factor_prompt: Some(&prompt),
                    ..AccountSyncRequest::new("account-a")
                },
                &source,
            )
            .await?;

        let session_ref = CredentialRef::account_session("account-a")?;

        assert_eq!(
            library.credentials.load_password(&session_ref)?.as_deref(),
            Some(TWO_FACTOR_SESSION)
        );

        Ok(())
    }

    #[tokio::test]
    async fn changing_the_password_forgets_the_stored_session() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;

        let source = TwoFactorSyncSource::new();
        let prompt = ScriptedTwoFactorPrompt::new([VALID_TWO_FACTOR_CODE, VALID_TWO_FACTOR_CODE]);
        let request = || AccountSyncRequest {
            two_factor_prompt: Some(&prompt),
            ..AccountSyncRequest::new("account-a")
        };

        library.sync_account_with_source(request(), &source).await?;
        library.save_account(save_account_request(true)).await?;
        library.sync_account_with_source(request(), &source).await?;

        assert_eq!(prompt.requests().len(), 2);
        assert_eq!(source.state().login_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn missing_password_blocks_sync() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(false)).await?;

        assert!(matches!(
            library
                .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
                .await,
            Err(LibraryError::MissingPassword(account_id)) if account_id == "account-a"
        ));

        Ok(())
    }

    #[tokio::test]
    async fn failed_sync_is_recorded_without_staling_cache() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &sync_source())
            .await?;
        let mut failing_source = sync_source();
        failing_source.fail_at = Some(FakeFailurePoint::Works);

        assert!(matches!(
            library
                .sync_account_with_source(AccountSyncRequest::new("account-a"), &failing_source)
                .await,
            Err(LibraryError::SyncSource(_))
        ));

        let page = library.list_products(&ProductListQuery::default()).await?;
        let sync_runs = library.storage().sync_runs_for_account("account-a").await?;

        assert_eq!(page.total_count, 2);
        assert_eq!(sync_runs.len(), 2);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Failed);
        assert_eq!(sync_runs[0].error_code, Some("sync_source".to_owned()));

        Ok(())
    }

    #[tokio::test]
    async fn missing_work_details_are_cached_but_hidden_from_product_list() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        let mut source = sync_source();
        source.works.pop();

        let report = library
            .sync_account_with_source(AccountSyncRequest::new("account-a"), &source)
            .await?;

        let page = library.list_products(&ProductListQuery::default()).await?;
        let sync_runs = library.storage().sync_runs_for_account("account-a").await?;

        assert_eq!(report.purchased_count, 2);
        assert_eq!(report.cached_work_count, 2);
        assert_eq!(report.missing_detail_count, 1);
        assert_eq!(page.total_count, 1);
        assert_eq!(page.products.len(), 1);
        assert_eq!(page.products[0].work_id, "RJ000001");
        assert_eq!(sync_runs.len(), 1);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Completed);

        Ok(())
    }

    #[tokio::test]
    async fn cancellation_records_failed_sync() -> Result<()> {
        let library = migrated_library().await?;
        library.save_account(save_account_request(true)).await?;
        let token = CancellationToken::new();
        token.cancel();

        assert!(matches!(
            library
                .sync_account_with_source(
                    AccountSyncRequest {
                        account_id: "account-a",
                        password: None,
                        cancellation_token: Some(&token),
                        progress_sink: None,
                        two_factor_prompt: None,
                    },
                    &sync_source(),
                )
                .await,
            Err(LibraryError::Cancelled)
        ));

        let sync_runs = library.storage().sync_runs_for_account("account-a").await?;

        assert_eq!(sync_runs.len(), 1);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Cancelled);
        assert_eq!(sync_runs[0].error_code, Some("cancelled".to_owned()));

        Ok(())
    }
}
