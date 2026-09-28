//! Always-on, local diagnostics. Unsafe strings are converted before queueing.
mod context;
mod environment;
mod layer;
mod privacy;
mod store;

use chrono::{SecondsFormat, Utc};
pub use context::{record_current_failure, Operation, OperationContext};
pub use environment::{EnvironmentSnapshot, ENVIRONMENT_KEYS};
pub use layer::DiagnosticLayer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
pub use store::{ExportScope, RunSummary};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, AuditError>;
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("diagnostics I/O failure ({0:?})")]
    Io(#[from] std::io::Error),
    #[error("invalid diagnostic data")]
    Json(#[from] serde_json::Error),
    #[error("diagnostic archive failure")]
    Zip(#[from] zip::result::ZipError),
    #[error("diagnostics busy or unavailable")]
    Unavailable,
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
    Unknown,
}

/// Activity projection of the same persisted event, not a second log stream.
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
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub sequence: u64,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub parent_operation_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default = "schema_version")]
    pub schema_version: u8,
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub audience: String,
}
fn schema_version() -> u8 {
    1
}
impl AuditEvent {
    pub fn new(
        level: AuditLevel,
        operation: impl Into<String>,
        outcome: AuditOutcome,
        _message: impl Into<String>,
    ) -> Self {
        Self {
            at: now_string(),
            level,
            operation: operation.into(),
            outcome,
            message: String::new(),
            error_code: None,
            error_message: None,
            details: json!({}),
            run_id: String::new(),
            sequence: 0,
            operation_id: None,
            parent_operation_id: None,
            job_id: None,
            duration_ms: None,
            schema_version: 1,
            component: "app".into(),
            stage: "result".into(),
            kind: "diagnostic".into(),
            audience: "activity".into(),
        }
    }
    pub fn queued(op: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(AuditLevel::Info, op, AuditOutcome::Queued, msg)
    }
    pub fn succeeded(op: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(AuditLevel::Info, op, AuditOutcome::Succeeded, msg)
    }
    pub fn failed(op: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(AuditLevel::Error, op, AuditOutcome::Failed, msg)
    }
    pub fn cancelled(op: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(AuditLevel::Warn, op, AuditOutcome::Cancelled, msg)
    }
    pub fn with_error(
        mut self,
        code: Option<impl Into<String>>,
        _message: impl Into<String>,
    ) -> Self {
        self.error_code = code.map(Into::into);
        self
    }
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub state: &'static str,
    pub last_io_code: Option<i32>,
    pub dropped_routine: u64,
    pub dropped_critical: u64,
    pub ring_evicted: u64,
    pub oversized: u64,
    pub last_written_sequence: u64,
    pub last_sequence: u64,
    pub flush_complete: bool,
}
impl Default for Health {
    fn default() -> Self {
        Self {
            state: "unavailable",
            last_io_code: None,
            dropped_routine: 0,
            dropped_critical: 0,
            ring_evicted: 0,
            oversized: 0,
            last_written_sequence: 0,
            last_sequence: 0,
            flush_complete: false,
        }
    }
}
#[derive(Clone)]
pub struct AuditLogger {
    owner: Arc<Owner>,
}
impl std::fmt::Debug for AuditLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLogger")
            .field("run_id", &self.owner.shared.run_id)
            .finish_non_exhaustive()
    }
}
struct Owner {
    shared: Arc<Shared>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        let mut s = self.shared.lock();
        s.stop = true;
        self.shared.wake.notify_all();
    }
}
struct Shared {
    root: PathBuf,
    run_id: String,
    inner: Mutex<Inner>,
    wake: Condvar,
    export: Mutex<()>,
}
struct Inner {
    privacy: privacy::Privacy,
    queue: VecDeque<(u64, Vec<u8>)>,
    queue_bytes: usize,
    ring: VecDeque<AuditEvent>,
    ring_bytes: usize,
    metadata: Value,
    health: Health,
    stop: bool,
    flush_requested: u64,
    flush_ack: u64,
    flush_results: VecDeque<(u64, u64, bool)>,
}
impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}
const RING_BYTES: usize = 1024 * 1024;
const QUEUE_BYTES: usize = 1024 * 1024;
const EVENT_BYTES: usize = 16 * 1024;

impl AuditLogger {
    /// Failure to attach disk storage leaves a usable bounded in-memory logger.
    pub fn new(log_dir: impl Into<PathBuf>) -> Result<Self> {
        let shared = Arc::new(Shared {
            root: log_dir.into(),
            run_id: format!("run-{}", Uuid::new_v4()),
            inner: Mutex::new(Inner {
                privacy: privacy::Privacy::default(),
                queue: VecDeque::new(),
                queue_bytes: 0,
                ring: VecDeque::new(),
                ring_bytes: 0,
                metadata: json!({"schemaVersion":1,"metadataUnavailable":true}),
                health: Health::default(),
                stop: false,
                flush_requested: 0,
                flush_ack: 0,
                flush_results: VecDeque::new(),
            }),
            wake: Condvar::new(),
            export: Mutex::new(()),
        });
        let worker = shared.clone();
        if std::thread::Builder::new()
            .name("diagnostics-writer".into())
            .spawn(move || store::writer(worker))
            .is_err()
        {
            shared.lock().health.state = "unavailable";
        }
        Ok(Self {
            owner: Arc::new(Owner { shared }),
        })
    }
    pub fn log_dir(&self) -> &Path {
        &self.owner.shared.root
    }
    pub fn run_id(&self) -> &str {
        &self.owner.shared.run_id
    }
    pub fn health(&self) -> Health {
        self.owner.shared.lock().health.clone()
    }
    pub fn summary(&self) -> Value {
        json!({"runId":self.run_id(),"health":self.health(),"metadata":self.owner.shared.lock().metadata.clone()})
    }
    pub fn operation(&self, name: &'static str) -> Operation {
        Operation::new(self.clone(), name, None)
    }
    pub fn job_operation(&self, name: &'static str, job_id: &str) -> Operation {
        Operation::new(self.clone(), name, Some(job_id.to_owned()))
    }
    pub async fn record(&self, event: AuditEvent) -> Result<()> {
        self.record_now(event);
        Ok(())
    }
    pub fn record_now(&self, mut event: AuditEvent) {
        let mut inner = self.owner.shared.lock();
        event.details = inner.privacy.fields(&event.details, 0);
        self.submit(&mut inner, event);
    }
    pub(crate) fn trusted(&self, event: AuditEvent) {
        let mut inner = self.owner.shared.lock();
        self.submit(&mut inner, event);
    }
    fn submit(&self, inner: &mut Inner, mut event: AuditEvent) {
        event.operation = operation_name(&event.operation).into();
        event.at = now_string();
        event.schema_version = 1;
        event.component = match event.operation.as_str() {
            "desktop.launch" => "desktop",
            "job" | "job.phase" | "job.events" => "jobs",
            "app.startup" => "startup",
            "runtime" => "core",
            op if op.starts_with("native.") || op.starts_with("frontend.") => "frontend",
            _ => "app",
        }
        .into();
        event.stage = match event.stage.as_str() {
            "result" | "validated" | "cause" | "configuration" | "directories" | "database"
            | "credentials" => event.stage,
            _ => "result".into(),
        };
        event.kind = match event.kind.as_str() {
            "started" | "finished" => event.kind,
            _ => "diagnostic".into(),
        };
        event.audience = if event.audience == "support" {
            "support"
        } else {
            "activity"
        }
        .into();
        event.operation_id = event.operation_id.filter(|id| privacy::valid_id(id));
        event.parent_operation_id = event.parent_operation_id.filter(|id| privacy::valid_id(id));
        event.job_id = event.job_id.filter(|id| privacy::valid_id(id));
        event.message = match event.operation.as_str() {
            "open_work_download" | "open_audit_log_dir" | "open_external_url" | "work.open"
            | "audit.openLogDir" | "external.open"
                if event.outcome == AuditOutcome::Succeeded =>
            {
                "Open request sent".into()
            }
            "desktop.launch" if event.kind == "finished" => match event.outcome {
                AuditOutcome::Failed => "Open request failed",
                AuditOutcome::Unknown => "Open result not confirmed",
                _ => "Open request sent",
            }
            .into(),
            _ => format!("{}: {:?}", event.operation, event.outcome),
        };
        event.error_message = None;
        event.error_code = event.error_code.map(|c| privacy::known_code(&c).to_owned());
        if let Some(context) = context::current() {
            event.operation_id.get_or_insert(context.id);
            event.parent_operation_id = event.parent_operation_id.or(context.parent);
            event.job_id = event.job_id.or(context.job_id);
        }
        event.run_id = self.run_id().into();
        inner.health.last_sequence += 1;
        event.sequence = inner.health.last_sequence;
        let mut bytes = serde_json::to_vec(&event).expect("serializable event");
        if bytes.len() > EVENT_BYTES {
            inner.health.oversized += 1;
            event.details = json!({"omitted":true,"reason":"event_size"});
            bytes = serde_json::to_vec(&event).expect("serializable event");
        }
        bytes.push(b'\n');
        while inner.ring_bytes + bytes.len() > RING_BYTES {
            if let Some(old) = inner.ring.pop_front() {
                inner.ring_bytes = inner
                    .ring_bytes
                    .saturating_sub(serde_json::to_vec(&old).map_or(0, |b| b.len() + 1));
                inner.health.ring_evicted += 1;
            } else {
                break;
            }
        }
        inner.ring_bytes += bytes.len();
        inner.ring.push_back(event.clone());
        let critical = event.level == AuditLevel::Error || event.kind == "finished";
        let cap = if critical {
            QUEUE_BYTES
        } else {
            QUEUE_BYTES - 256 * 1024
        };
        if inner.queue_bytes + bytes.len() > cap {
            if critical {
                inner.health.dropped_critical += 1;
            } else {
                inner.health.dropped_routine += 1;
            }
            inner.health.state = "degraded";
        } else {
            inner.queue_bytes += bytes.len();
            inner.queue.push_back((event.sequence, bytes));
            self.owner.shared.wake.notify_one();
        }
    }
    pub async fn recent_events(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        Ok(self
            .owner
            .shared
            .lock()
            .ring
            .iter()
            .rev()
            .filter(|e| e.audience == "activity")
            .take(limit.clamp(1, 500))
            .cloned()
            .collect())
    }
    pub fn operation_events(&self, id: &str) -> Vec<AuditEvent> {
        self.owner
            .shared
            .lock()
            .ring
            .iter()
            .filter(|e| {
                e.operation_id.as_deref() == Some(id)
                    || e.parent_operation_id.as_deref() == Some(id)
            })
            .cloned()
            .collect()
    }
    pub fn flush(&self, timeout: Duration) -> bool {
        let until = Instant::now() + timeout;
        let shared = &self.owner.shared;
        let mut inner = shared.lock();
        inner.flush_requested += 1;
        let ticket = inner.flush_requested;
        shared.wake.notify_one();
        while inner.flush_ack < ticket {
            let Some(left) = until.checked_duration_since(Instant::now()) else {
                break;
            };
            let (next, result) = shared
                .wake
                .wait_timeout(inner, left)
                .unwrap_or_else(|e| e.into_inner());
            inner = next;
            if result.timed_out() {
                break;
            }
        }
        inner.health.flush_complete = inner
            .flush_results
            .iter()
            .find(|(first, last, _)| *first <= ticket && ticket <= *last)
            .is_some_and(|(_, _, succeeded)| *succeeded);
        inner.health.flush_complete
    }
    pub fn path_reference(&self, path: &Path) -> Value {
        self.owner
            .shared
            .lock()
            .privacy
            .path(&path.to_string_lossy(), None)
    }
    pub fn desktop_path(&self, path: &Path) {
        let mut event = AuditEvent::succeeded("desktop.launch", "");
        event.stage = "validated".into();
        event.audience = "support".into();
        event.details = json!({"targetKind":"path","target":self.path_reference(path)});
        self.trusted(event);
    }
}

pub fn sanitize_value(value: Value) -> Value {
    privacy::Privacy::default().fields(&value, 0)
}
fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn operation_name(value: &str) -> &str {
    match value {
        // Static operation vocabulary; arbitrary frontend/tracing strings never reach disk.
        "native.dialog" | "native.updater" | "native.relaunch" | "native.appInfo"
        | "native.path" | "native.listener" | "frontend.rejection" | "open_external_url"
        | "app.startup" | "command" | "job" | "job.phase" | "job.events" | "desktop.launch"
        | "external.open" | "frontend.error" | "diagnostics.export" | "runtime" => value,
        _ => legacy_operation(value),
    }
}

fn legacy_operation(value: &str) -> &str {
    match value {
        "account.login.twoFactor"
        | "account.remove"
        | "account.save"
        | "account.setEnabled"
        | "account.sync"
        | "account.sync.queue"
        | "app.startup"
        | "audit.openLogDir"
        | "cancel_job"
        | "cancel_two_factor"
        | "clear_finished_jobs"
        | "delete_work_download"
        | "get_audit_log_dir"
        | "get_job"
        | "get_job_logs"
        | "get_product_detail"
        | "get_settings"
        | "job.unknown"
        | "list_accounts"
        | "list_audit_events"
        | "list_jobs"
        | "list_product_filter_facets"
        | "list_products"
        | "mark_work_downloaded"
        | "open_audit_log_dir"
        | "open_work_download"
        | "preview_bulk_work_download"
        | "product.tags.update"
        | "remove_account"
        | "save_account"
        | "save_settings"
        | "save_product_cover"
        | "scan_local_work_downloads"
        | "set_account_enabled"
        | "set_product_custom_tags"
        | "settings.save"
        | "start_account_sync"
        | "start_bulk_work_download"
        | "start_work_download"
        | "submit_two_factor_code"
        | "work.bulkDownload"
        | "work.bulkDownload.preview"
        | "work.bulkDownloadPreview"
        | "work.download"
        | "work.download.delete"
        | "work.download.mark"
        | "work.download.queue"
        | "work.local.scan"
        | "work.open" => value,
        _ => "unclassified",
    }
}

#[cfg(test)]
mod tests;
