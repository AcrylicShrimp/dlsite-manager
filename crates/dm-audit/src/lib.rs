use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

pub type Result<T> = std::result::Result<T, AuditError>;

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit log I/O error")]
    Io(#[from] std::io::Error),
    #[error("audit log JSON error")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditOutcome {
    Queued,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub at: String,
    pub level: AuditLevel,
    pub operation: String,
    pub outcome: AuditOutcome,
    pub message: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub details: Value,
}

impl AuditEvent {
    pub fn new(
        level: AuditLevel,
        operation: impl Into<String>,
        outcome: AuditOutcome,
        message: impl Into<String>,
    ) -> Self {
        Self {
            at: now_string(),
            level,
            operation: operation.into(),
            outcome,
            message: message.into(),
            error_code: None,
            error_message: None,
            details: Value::Object(Map::new()),
        }
    }

    pub fn queued(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AuditLevel::Info, operation, AuditOutcome::Queued, message)
    }

    pub fn succeeded(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            AuditLevel::Info,
            operation,
            AuditOutcome::Succeeded,
            message,
        )
    }

    pub fn failed(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AuditLevel::Error, operation, AuditOutcome::Failed, message)
    }

    pub fn cancelled(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            AuditLevel::Warn,
            operation,
            AuditOutcome::Cancelled,
            message,
        )
    }

    pub fn with_error(
        mut self,
        code: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        self.error_code = code.map(Into::into);
        self.error_message = Some(message.into());
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = sanitize_value(details);
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuditLogger {
    log_dir: Arc<PathBuf>,
}

impl AuditLogger {
    pub fn new(log_dir: impl Into<PathBuf>) -> Result<Self> {
        let log_dir = log_dir.into();
        std::fs::create_dir_all(&log_dir)?;
        Ok(Self {
            log_dir: Arc::new(log_dir),
        })
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub async fn record(&self, event: AuditEvent) -> Result<()> {
        fs::create_dir_all(self.log_dir()).await?;
        let path = self.file_path_for_event(&event);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        let mut line = serde_json::to_string(&event)?;

        line.push('\n');
        file.write_all(line.as_bytes()).await?;

        tracing::event!(
            target: "dm_audit",
            tracing::Level::INFO,
            operation = event.operation.as_str(),
            outcome = ?event.outcome,
            audit_level = ?event.level,
            error_code = event.error_code.as_deref(),
            message = event.message.as_str(),
            "audit event recorded",
        );

        Ok(())
    }

    pub async fn recent_events(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        let limit = limit.clamp(1, 500);
        let files = self.audit_files_newest_first().await?;
        let mut events = Vec::new();

        for file in files {
            let content = match fs::read_to_string(&file).await {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };

            for line in content.lines().rev() {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if let Ok(event) = serde_json::from_str::<AuditEvent>(line) {
                    events.push(event);
                }

                if events.len() >= limit {
                    return Ok(events);
                }
            }
        }

        Ok(events)
    }

    async fn audit_files_newest_first(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut entries = match fs::read_dir(self.log_dir()).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(files),
            Err(error) => return Err(error.into()),
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension() != Some(OsStr::new("jsonl")) {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };

            if file_name.starts_with("audit-") {
                files.push(path);
            }
        }

        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        Ok(files)
    }

    fn file_path_for_event(&self, event: &AuditEvent) -> PathBuf {
        let date = event.at.get(0..10).unwrap_or("unknown");

        self.log_dir.join(format!("audit-{date}.jsonl"))
    }
}

pub fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[redacted]".to_owned()))
                    } else {
                        (key, sanitize_value(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(sanitize_value).collect()),
        value => value,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();

    normalized.contains("password")
        || normalized.contains("cookie")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("serial")
        || normalized.contains("session")
        || normalized.contains("auth")
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn records_and_reads_recent_events() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let logger = AuditLogger::new(temp.path())?;

        logger
            .record(AuditEvent::succeeded("settings.save", "Saved settings"))
            .await?;
        logger
            .record(
                AuditEvent::failed("download.open", "Open failed")
                    .with_error(Some("not_downloaded"), "RJ000001 is not downloaded"),
            )
            .await?;

        let events = logger.recent_events(10).await?;

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].operation, "download.open");
        assert_eq!(events[1].operation, "settings.save");

        Ok(())
    }

    #[test]
    fn sanitizes_sensitive_details_recursively() {
        let value = sanitize_value(json!({
            "accountId": "account-a",
            "password": "secret",
            "nested": {
                "sessionCookie": "cookie",
                "items": [{ "serialNumber": "1234" }]
            }
        }));

        assert_eq!(value["accountId"], "account-a");
        assert_eq!(value["password"], "[redacted]");
        assert_eq!(value["nested"]["sessionCookie"], "[redacted]");
        assert_eq!(value["nested"]["items"][0]["serialNumber"], "[redacted]");
    }
}
