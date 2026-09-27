use serde_json::{json, Map, Value};
use std::{collections::HashMap, path::Path};
use uuid::Uuid;

/// Private values never leave this run's bounded alias table.
#[derive(Default)]
pub(crate) struct Privacy {
    aliases: HashMap<String, String>,
    bytes: usize,
}

impl Privacy {
    pub(crate) fn alias(&mut self, value: &str) -> String {
        if let Some(alias) = self.aliases.get(value) {
            return alias.clone();
        }
        if self.aliases.len() >= 4096 || self.bytes + value.len() > 256 * 1024 {
            return "reference-unavailable".into();
        }
        let alias = format!("ref-{}", Uuid::new_v4());
        self.bytes += value.len();
        self.aliases.insert(value.into(), alias.clone());
        alias
    }

    pub(crate) fn path(&mut self, value: &str, appdir: Option<&str>) -> Value {
        let p = Path::new(value);
        let origin = if value.is_empty() {
            "empty"
        } else if appdir.is_some_and(|root| !root.is_empty() && p.starts_with(root)) {
            "appImage"
        } else if p.is_absolute() {
            "host"
        } else {
            "relative"
        };
        // Only literal well-known OS paths are public. Never expose arbitrary basenames.
        let public = matches!(
            value,
            "/usr/bin"
                | "/bin"
                | "/usr/local/bin"
                | "/usr/lib"
                | "/lib"
                | "/usr/share"
                | "/usr/local/share"
        );
        json!({"reference": if public { value.to_owned() } else { self.alias(value) },
            "origin":origin,"absolute":p.is_absolute(),"hasSpaces":value.contains(' '),"nonAscii":!value.is_ascii()})
    }

    pub(crate) fn fields(&mut self, value: &Value, depth: usize) -> Value {
        if depth > 8 {
            return json!({"omitted":true});
        }
        let Some(map) = value.as_object() else {
            return json!({});
        };
        let mut result = Map::new();
        for (key, value) in map.iter().take(64) {
            let lower = key.to_ascii_lowercase();
            if [
                "password", "cookie", "token", "secret", "serial", "session", "auth", "snippet",
                "message", "url", "endpoint", "title", "label",
            ]
            .iter()
            .any(|s| lower.contains(s))
            {
                continue;
            }
            if matches!(
                key.as_str(),
                "workId"
                    | "accountId"
                    | "requestId"
                    | "interruptedWorkId"
                    | "path"
                    | "localPath"
                    | "retainedPath"
                    | "logDir"
                    | "dataDir"
            ) {
                if let Some(s) = value.as_str() {
                    result.insert(key.clone(), json!(self.alias(s)));
                }
            } else if matches!(key.as_str(), "jobId" | "operationId" | "clientId") {
                if let Some(s) = value.as_str().filter(|s| valid_id(s)) {
                    result.insert(key.clone(), json!(s));
                }
            } else if matches!(
                key.as_str(),
                "errorDetails" | "details" | "bulkDownload" | "output"
            ) {
                result.insert(key.clone(), self.fields(value, depth + 1));
            } else if numeric_field(key)
                && (value.is_number() || value.is_boolean() || value.is_null())
            {
                result.insert(key.clone(), value.clone());
            } else if matches!(
                key.as_str(),
                "failureKind"
                    | "archiveErrorKind"
                    | "apiErrorKind"
                    | "downloadErrorKind"
                    | "phase"
                    | "status"
                    | "kind"
                    | "errorCode"
            ) {
                if let Some(s) = value.as_str() {
                    result.insert(key.clone(), json!(known_code(s)));
                }
            }
        }
        Value::Object(result)
    }
}

fn numeric_field(key: &str) -> bool {
    matches!(
        key,
        "bytes"
            | "bytesWritten"
            | "fileCount"
            | "cachedWorkCount"
            | "succeededCount"
            | "plannedCount"
            | "knownExpectedBytes"
            | "totalExpectedBytes"
            | "unknownSizeCount"
            | "httpStatus"
            | "osCode"
            | "exitCode"
            | "signal"
            | "durationMs"
            | "attempt"
            | "attempts"
            | "isTimeout"
            | "isConnect"
            | "isBody"
            | "isDecode"
            | "limit"
            | "expectedBytes"
            | "actualBytes"
            | "downloadedBytes"
            | "totalBytes"
            | "current"
            | "total"
            | "count"
            | "totalCount"
            | "requestedCount"
            | "downloadedCount"
            | "failedCount"
            | "skippedDownloadedCount"
            | "skippedQueuedCount"
            | "scannedDirectories"
            | "importedCount"
            | "skippedNoId"
            | "skippedAmbiguous"
            | "skipped"
            | "repeatCount"
            | "cancelled"
            | "expected"
            | "actual"
    )
}

pub(crate) fn valid_id(s: &str) -> bool {
    let value = s
        .strip_prefix("job-")
        .or_else(|| s.strip_prefix("op-"))
        .or_else(|| s.strip_prefix("run-"))
        .unwrap_or(s);
    Uuid::parse_str(value).is_ok()
}

pub(crate) fn known_code(s: &str) -> &str {
    match s {
        "zip_other"
        | "zip_io"
        | "zip_invalid"
        | "zip_unsupported"
        | "zip_missing_entry"
        | "zip_password"
        | "zip_compression"
        | "rar_corrupt"
        | "rar_format"
        | "rar_open"
        | "rar_create"
        | "rar_close"
        | "rar_read"
        | "rar_write"
        | "rar_memory"
        | "rar_password"
        | "rar_dictionary"
        | "rar_reference"
        | "rar_other"
        | "unsafe_entry"
        | "target_exists"
        | "unsupported_plan"
        | "finalize_prepare"
        | "finalize_install"
        | "rollback"
        | "finalize_cleanup"
        | "shared_library_load"
        | "undefined_symbol"
        | "symbol_lookup"
        | "no_desktop_handler"
        | "display_unavailable"
        | "connection_failed"
        | "directories"
        | "database"
        | "login"
        | "network"
        | "metadata"
        | "purchases"
        | "works"
        | "other"
        | "validation"
        | "storage"
        | "path"
        | "path_outside_roots"
        | "not_downloaded"
        | "opener"
        | "api"
        | "download"
        | "archive"
        | "credentials"
        | "io"
        | "unexpected_status"
        | "unexpected_response"
        | "transport"
        | "timeout"
        | "cancelled"
        | "partial_failure"
        | "failed"
        | "succeeded"
        | "queued"
        | "running"
        | "cancelling"
        | "completed"
        | "unknown"
        | "batch_limit_exceeded"
        | "download_recovery_required"
        | "permission_denied"
        | "not_found"
        | "disk_full"
        | "connection"
        | "spawn_failed"
        | "helper_failed"
        | "two_factor_timeout"
        | "two_factor_rejected"
        | "account_disabled"
        | "account_not_found"
        | "accountSync"
        | "workDownload"
        | "bulkWorkDownload"
        | "bulkWorkDownloadPreview"
        | "authenticating"
        | "loadingWorks"
        | "loadingProducts"
        | "bulkPlanning"
        | "downloading"
        | "extracting"
        | "finalizing"
        | "loadingMetadata"
        | "scanning"
        | "recovering"
        | "connecting"
        | "bulkDownloading"
        | "committing"
        | "cookie_store"
        | "download_account_not_found"
        | "download_page_link_not_found"
        | "download_path_not_directory"
        | "download_path_outside_roots"
        | "download_target_exists"
        | "download_unavailable"
        | "download_unknown_redirect"
        | "file_name_unknown"
        | "frontend"
        | "incomplete_download"
        | "invalid_credentials"
        | "invalid_file_name"
        | "invalid_location_header"
        | "invalid_response"
        | "invalid_two_factor_code"
        | "json"
        | "loadingCount"
        | "loadingPurchases"
        | "location_header_missing"
        | "loggingIn"
        | "missing_login_name"
        | "missing_password"
        | "not_authorized"
        | "panic"
        | "plan_work_mismatch"
        | "redirect_limit_exceeded"
        | "resolvingDownload"
        | "scanningLocalDownloads"
        | "size_exceeded"
        | "stream"
        | "sync_source"
        | "target_already_exists"
        | "two_factor_cancelled"
        | "two_factor_form_not_recognized"
        | "two_factor_prompt_unavailable"
        | "two_factor_required"
        | "two_factor_timed_out"
        | "unexpected_json"
        | "unpacking"
        | "url"
        | "xsrf_token_not_found" => s,
        _ => "unclassified",
    }
}
