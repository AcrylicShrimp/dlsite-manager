use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
};
use tokio::sync::broadcast;
use uuid::Uuid;

pub type JobMetadata = BTreeMap<String, serde_json::Value>;
pub type JobRunResult = Result<JobMetadata, JobFailure>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    pub fn new() -> Self {
        Self(format!("job-{}", Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobKind(String);

impl JobKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JobKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for JobKind {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for JobKind {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_active(self) -> bool {
        !self.is_terminal()
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobEventKind {
    Created,
    Started,
    Updated,
    Log,
    CancellationRequested,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub current: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>,
}

impl JobProgress {
    pub fn new(current: Option<u64>, total: Option<u64>, unit: Option<impl Into<String>>) -> Self {
        Self {
            current,
            total,
            unit: unit.map(Into::into),
        }
    }

    pub fn items(current: Option<u64>, total: Option<u64>) -> Self {
        Self::new(current, total, Some("items"))
    }

    pub fn bytes(current: Option<u64>, total: Option<u64>) -> Self {
        Self::new(current, total, Some("bytes"))
    }

    pub fn files(current: Option<u64>, total: Option<u64>) -> Self {
        Self::new(current, total, Some("files"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobFailure {
    pub code: Option<String>,
    pub message: String,
    pub details: JobMetadata,
}

impl JobFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            details: JobMetadata::new(),
        }
    }

    pub fn with_code(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.into()),
            message: message.into(),
            details: JobMetadata::new(),
        }
    }

    pub fn cancelled() -> Self {
        Self::with_code("cancelled", "job was cancelled")
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.code.as_deref() == Some("cancelled")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: JobId,
    pub kind: JobKind,
    pub title: String,
    pub status: JobStatus,
    pub phase: Option<String>,
    pub progress: Option<JobProgress>,
    pub metadata: JobMetadata,
    pub output: Option<JobMetadata>,
    pub error: Option<JobFailure>,
    pub cancellable: bool,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogEntry {
    pub sequence: u64,
    pub at: String,
    pub level: JobLogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub sequence: u64,
    pub event_kind: JobEventKind,
    pub job_id: JobId,
    pub kind: JobKind,
    pub status: JobStatus,
    pub phase: Option<String>,
    pub progress: Option<JobProgress>,
    pub message: Option<String>,
    pub log: Option<JobLogEntry>,
    pub snapshot: JobSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogPage {
    pub job_id: JobId,
    pub entries: Vec<JobLogEntry>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobManagerConfig {
    pub max_finished_jobs: usize,
    pub max_logs_per_job: usize,
    pub event_channel_capacity: usize,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        Self {
            max_finished_jobs: 100,
            max_logs_per_job: 200,
            event_channel_capacity: 512,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JobManagerError {
    #[error("job not found: {0}")]
    JobNotFound(JobId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelJobResult {
    pub outcome: CancelJobOutcome,
    pub snapshot: JobSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CancelJobOutcome {
    Requested,
    AlreadyFinished,
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Mutex<Inner>>,
    events: broadcast::Sender<JobEvent>,
}

impl JobManager {
    pub fn new(config: JobManagerConfig) -> Self {
        let event_channel_capacity = config.event_channel_capacity.max(1);
        let (events, _) = broadcast::channel(event_channel_capacity);

        Self {
            inner: Arc::new(Mutex::new(Inner::new(config))),
            events,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.events.subscribe()
    }

    pub fn spawn<K, T, F, Fut>(&self, kind: K, title: T, metadata: JobMetadata, job_fn: F) -> JobId
    where
        K: Into<JobKind>,
        T: Into<String>,
        F: FnOnce(JobContext) -> Fut + Send + 'static,
        Fut: Future<Output = JobRunResult> + Send + 'static,
    {
        let id = JobId::new();
        let kind = kind.into();
        let cancellation_token = CancellationToken::new();
        let snapshot = JobSnapshot {
            id: id.clone(),
            kind,
            title: title.into(),
            status: JobStatus::Queued,
            phase: None,
            progress: None,
            metadata,
            output: None,
            error: None,
            cancellable: true,
            created_at: now_string(),
            started_at: None,
            finished_at: None,
        };
        let event = {
            let mut inner = self.lock_inner();
            let record = JobRecord {
                snapshot: snapshot.clone(),
                logs: VecDeque::new(),
                cancellation_token: cancellation_token.clone(),
            };

            inner.created_order.push(id.clone());
            inner.jobs.insert(id.clone(), record);
            inner.event_from_snapshot(
                JobEventKind::Created,
                snapshot,
                Some("job queued".to_owned()),
                None,
            )
        };
        self.emit(event);

        let manager = self.clone();
        let task_id = id.clone();
        let context = JobContext {
            manager: self.clone(),
            job_id: id.clone(),
            cancellation_token,
        };

        tokio::spawn(async move {
            manager.mark_running(&task_id);
            let result = job_fn(context).await;
            manager.finish_job(&task_id, result);
        });

        id
    }

    pub fn list_jobs(&self) -> Vec<JobSnapshot> {
        let inner = self.lock_inner();

        inner
            .created_order
            .iter()
            .filter_map(|id| inner.jobs.get(id))
            .map(|record| record.snapshot.clone())
            .collect()
    }

    pub fn get_job(&self, id: &JobId) -> Option<JobSnapshot> {
        self.lock_inner()
            .jobs
            .get(id)
            .map(|record| record.snapshot.clone())
    }

    pub fn cancel_job(&self, id: &JobId) -> Result<CancelJobResult, JobManagerError> {
        let event_and_result = {
            let mut inner = self.lock_inner();
            let Some(record) = inner.jobs.get_mut(id) else {
                return Err(JobManagerError::JobNotFound(id.clone()));
            };

            if record.snapshot.status.is_terminal() {
                let snapshot = record.snapshot.clone();
                return Ok(CancelJobResult {
                    outcome: CancelJobOutcome::AlreadyFinished,
                    snapshot,
                });
            }

            record.cancellation_token.cancel();
            record.snapshot.status = JobStatus::Cancelling;
            record.snapshot.cancellable = true;
            let snapshot = record.snapshot.clone();
            let event = inner.event_from_snapshot(
                JobEventKind::CancellationRequested,
                snapshot.clone(),
                Some("cancellation requested".to_owned()),
                None,
            );

            (
                event,
                CancelJobResult {
                    outcome: CancelJobOutcome::Requested,
                    snapshot,
                },
            )
        };
        self.emit(event_and_result.0);
        Ok(event_and_result.1)
    }

    pub fn job_logs(
        &self,
        id: &JobId,
        after_sequence: Option<u64>,
        limit: Option<usize>,
    ) -> Result<JobLogPage, JobManagerError> {
        let inner = self.lock_inner();
        let Some(record) = inner.jobs.get(id) else {
            return Err(JobManagerError::JobNotFound(id.clone()));
        };
        let after_sequence = after_sequence.unwrap_or(0);
        let limit = limit.unwrap_or(100).min(500);
        let mut entries = record
            .logs
            .iter()
            .filter(|entry| entry.sequence > after_sequence)
            .take(limit.saturating_add(1))
            .cloned()
            .collect::<Vec<_>>();
        let has_more = entries.len() > limit;

        if has_more {
            entries.truncate(limit);
        }

        Ok(JobLogPage {
            job_id: id.clone(),
            entries,
            has_more,
        })
    }

    pub fn clear_finished(&self) -> usize {
        let mut inner = self.lock_inner();
        let finished_ids = inner
            .created_order
            .iter()
            .filter(|id| {
                inner
                    .jobs
                    .get(*id)
                    .is_some_and(|record| record.snapshot.status.is_terminal())
            })
            .cloned()
            .collect::<Vec<_>>();
        let removed_count = finished_ids.len();

        for id in &finished_ids {
            inner.jobs.remove(id);
        }

        inner
            .created_order
            .retain(|id| !finished_ids.iter().any(|removed| removed == id));
        inner
            .finished_order
            .retain(|id| !finished_ids.iter().any(|removed| removed == id));

        removed_count
    }

    fn mark_running(&self, id: &JobId) {
        let event = {
            let mut inner = self.lock_inner();
            let Some(record) = inner.jobs.get_mut(id) else {
                return;
            };

            if record.snapshot.status.is_terminal() || record.snapshot.started_at.is_some() {
                return;
            }

            if record.snapshot.status == JobStatus::Queued {
                record.snapshot.status = JobStatus::Running;
            }

            record.snapshot.started_at = Some(now_string());
            let snapshot = record.snapshot.clone();
            inner.event_from_snapshot(
                JobEventKind::Started,
                snapshot,
                Some("job started".to_owned()),
                None,
            )
        };
        self.emit(event);
    }

    fn finish_job(&self, id: &JobId, result: JobRunResult) {
        let event = {
            let mut inner = self.lock_inner();
            let Some(record) = inner.jobs.get_mut(id) else {
                return;
            };

            if record.snapshot.status.is_terminal() {
                return;
            }

            record.snapshot.finished_at = Some(now_string());
            record.snapshot.cancellable = false;

            let message = match result {
                Ok(output) => {
                    record.snapshot.status = JobStatus::Succeeded;
                    record.snapshot.output = Some(output);
                    Some("job succeeded".to_owned())
                }
                Err(error) if error.is_cancelled() => {
                    record.snapshot.status = JobStatus::Cancelled;
                    record.snapshot.error = Some(error);
                    Some("job cancelled".to_owned())
                }
                Err(error) => {
                    let message = error.message.clone();
                    record.snapshot.status = JobStatus::Failed;
                    record.snapshot.error = Some(error);
                    Some(message)
                }
            };

            let snapshot = record.snapshot.clone();
            inner.finished_order.push_back(id.clone());
            inner.trim_finished();
            inner.event_from_snapshot(JobEventKind::Finished, snapshot, message, None)
        };
        self.emit(event);
    }

    fn set_phase(&self, id: &JobId, phase: Option<String>) {
        let event = {
            let mut inner = self.lock_inner();
            let Some(record) = inner.jobs.get_mut(id) else {
                return;
            };

            if record.snapshot.status.is_terminal() {
                return;
            }

            record.snapshot.phase = phase;
            let snapshot = record.snapshot.clone();
            inner.event_from_snapshot(JobEventKind::Updated, snapshot, None, None)
        };
        self.emit(event);
    }

    fn set_progress(&self, id: &JobId, progress: Option<JobProgress>) {
        let event = {
            let mut inner = self.lock_inner();
            let Some(record) = inner.jobs.get_mut(id) else {
                return;
            };

            if record.snapshot.status.is_terminal() {
                return;
            }

            record.snapshot.progress = progress;
            let snapshot = record.snapshot.clone();
            inner.event_from_snapshot(JobEventKind::Updated, snapshot, None, None)
        };
        self.emit(event);
    }

    fn add_log(&self, id: &JobId, level: JobLogLevel, message: String) {
        let event = {
            let mut inner = self.lock_inner();
            let max_logs_per_job = inner.config.max_logs_per_job;
            let log = JobLogEntry {
                sequence: inner.next_log_sequence(),
                at: now_string(),
                level,
                message,
            };
            let Some(record) = inner.jobs.get_mut(id) else {
                return;
            };

            record.logs.push_back(log.clone());

            while record.logs.len() > max_logs_per_job {
                record.logs.pop_front();
            }

            let snapshot = record.snapshot.clone();
            inner.event_from_snapshot(
                JobEventKind::Log,
                snapshot,
                Some(log.message.clone()),
                Some(log),
            )
        };
        self.emit(event);
    }

    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("job manager lock poisoned")
    }

    fn emit(&self, event: JobEvent) {
        let _ = self.events.send(event);
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new(JobManagerConfig::default())
    }
}

#[derive(Clone)]
pub struct JobContext {
    manager: JobManager,
    job_id: JobId,
    cancellation_token: CancellationToken,
}

impl JobContext {
    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub fn check_cancelled(&self) -> Result<(), JobFailure> {
        if self.is_cancelled() {
            Err(JobFailure::cancelled())
        } else {
            Ok(())
        }
    }

    pub fn cancelled_failure(&self) -> JobFailure {
        JobFailure::cancelled()
    }

    pub fn set_phase(&self, phase: impl Into<String>) {
        self.manager.set_phase(&self.job_id, Some(phase.into()));
    }

    pub fn clear_phase(&self) {
        self.manager.set_phase(&self.job_id, None);
    }

    pub fn set_progress(&self, progress: JobProgress) {
        self.manager.set_progress(&self.job_id, Some(progress));
    }

    pub fn clear_progress(&self) {
        self.manager.set_progress(&self.job_id, None);
    }

    pub fn log(&self, level: JobLogLevel, message: impl Into<String>) {
        self.manager.add_log(&self.job_id, level, message.into());
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.log(JobLogLevel::Debug, message);
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(JobLogLevel::Info, message);
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.log(JobLogLevel::Warn, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(JobLogLevel::Error, message);
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

struct JobRecord {
    snapshot: JobSnapshot,
    logs: VecDeque<JobLogEntry>,
    cancellation_token: CancellationToken,
}

struct Inner {
    config: JobManagerConfig,
    jobs: BTreeMap<JobId, JobRecord>,
    created_order: Vec<JobId>,
    finished_order: VecDeque<JobId>,
    next_event_sequence: u64,
    next_log_sequence: u64,
}

impl Inner {
    fn new(config: JobManagerConfig) -> Self {
        Self {
            config,
            jobs: BTreeMap::new(),
            created_order: Vec::new(),
            finished_order: VecDeque::new(),
            next_event_sequence: 0,
            next_log_sequence: 0,
        }
    }

    fn next_event_sequence(&mut self) -> u64 {
        self.next_event_sequence += 1;
        self.next_event_sequence
    }

    fn next_log_sequence(&mut self) -> u64 {
        self.next_log_sequence += 1;
        self.next_log_sequence
    }

    fn event_from_snapshot(
        &mut self,
        event_kind: JobEventKind,
        snapshot: JobSnapshot,
        message: Option<String>,
        log: Option<JobLogEntry>,
    ) -> JobEvent {
        JobEvent {
            sequence: self.next_event_sequence(),
            event_kind,
            job_id: snapshot.id.clone(),
            kind: snapshot.kind.clone(),
            status: snapshot.status,
            phase: snapshot.phase.clone(),
            progress: snapshot.progress.clone(),
            message,
            log,
            snapshot,
        }
    }

    fn trim_finished(&mut self) {
        while self.finished_order.len() > self.config.max_finished_jobs {
            let Some(id) = self.finished_order.pop_front() else {
                break;
            };

            self.jobs.remove(&id);
            self.created_order.retain(|created_id| created_id != &id);
        }
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn successful_job_transitions_to_succeeded() {
        let manager = JobManager::default();
        let mut receiver = manager.subscribe();
        let id = manager.spawn(
            "test",
            "Successful job",
            JobMetadata::new(),
            |context| async move {
                context.info("starting work");
                context.set_phase("working");
                context.set_progress(JobProgress::items(Some(1), Some(2)));

                let mut output = JobMetadata::new();
                output.insert("done".to_owned(), json!(true));

                Ok(output)
            },
        );

        let snapshot = wait_for_terminal(&manager, &id).await;

        assert_eq!(snapshot.status, JobStatus::Succeeded);
        assert_eq!(
            snapshot.output.as_ref().and_then(|value| value.get("done")),
            Some(&json!(true))
        );
        assert_eq!(snapshot.phase.as_deref(), Some("working"));
        assert_eq!(
            manager.job_logs(&id, None, None).unwrap().entries[0].message,
            "starting work"
        );

        let events = drain_until_terminal(&mut receiver).await;
        assert!(events
            .iter()
            .any(|event| event.event_kind == JobEventKind::Created));
        assert!(events
            .iter()
            .any(|event| event.event_kind == JobEventKind::Started));
        assert_eq!(events.last().unwrap().snapshot.status, JobStatus::Succeeded);
    }

    #[tokio::test]
    async fn failing_job_transitions_to_failed() {
        let manager = JobManager::default();
        let id = manager.spawn(
            "test",
            "Failing job",
            JobMetadata::new(),
            |_context| async { Err(JobFailure::with_code("fixture_failed", "fixture failure")) },
        );

        let snapshot = wait_for_terminal(&manager, &id).await;

        assert_eq!(snapshot.status, JobStatus::Failed);
        assert_eq!(
            snapshot
                .error
                .as_ref()
                .and_then(|error| error.code.as_deref()),
            Some("fixture_failed")
        );
        assert_eq!(
            snapshot.error.as_ref().map(|error| error.message.as_str()),
            Some("fixture failure")
        );
    }

    #[tokio::test]
    async fn cancellation_request_finishes_as_cancelled_when_observed() {
        let manager = JobManager::default();
        let id = manager.spawn(
            "test",
            "Cancellable job",
            JobMetadata::new(),
            |context| async move {
                context.set_phase("waiting");

                loop {
                    if context.is_cancelled() {
                        return Err(context.cancelled_failure());
                    }

                    sleep(Duration::from_millis(5)).await;
                }
            },
        );

        wait_for_status(&manager, &id, JobStatus::Running).await;
        let cancel_result = manager.cancel_job(&id).unwrap();

        assert_eq!(cancel_result.outcome, CancelJobOutcome::Requested);
        assert_eq!(cancel_result.snapshot.status, JobStatus::Cancelling);

        let snapshot = wait_for_terminal(&manager, &id).await;

        assert_eq!(snapshot.status, JobStatus::Cancelled);
        assert_eq!(
            snapshot
                .error
                .as_ref()
                .and_then(|error| error.code.as_deref()),
            Some("cancelled")
        );
    }

    #[tokio::test]
    async fn logs_are_ordered_and_bounded() {
        let manager = JobManager::new(JobManagerConfig {
            max_finished_jobs: 100,
            max_logs_per_job: 2,
            event_channel_capacity: 32,
        });
        let id = manager.spawn(
            "test",
            "Logging job",
            JobMetadata::new(),
            |context| async move {
                context.info("one");
                context.warn("two");
                context.error("three");
                Ok(JobMetadata::new())
            },
        );

        wait_for_terminal(&manager, &id).await;
        let logs = manager.job_logs(&id, None, Some(10)).unwrap().entries;

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].message, "two");
        assert_eq!(logs[1].message, "three");
        assert!(logs[0].sequence < logs[1].sequence);
    }

    #[tokio::test]
    async fn events_are_ordered_and_include_final_snapshot() {
        let manager = JobManager::default();
        let mut receiver = manager.subscribe();
        let id = manager.spawn(
            "test",
            "Evented job",
            JobMetadata::new(),
            |context| async move {
                context.set_phase("first");
                context.set_progress(JobProgress::files(Some(1), Some(1)));
                Ok(JobMetadata::new())
            },
        );

        let snapshot = wait_for_terminal(&manager, &id).await;
        let events = drain_until_terminal(&mut receiver).await;

        assert_eq!(snapshot.status, JobStatus::Succeeded);
        assert!(!events.is_empty());
        assert!(events
            .windows(2)
            .all(|events| events[0].sequence < events[1].sequence));
        assert_eq!(events.last().unwrap().status, JobStatus::Succeeded);
        assert_eq!(events.last().unwrap().snapshot.id, id);
    }

    #[tokio::test]
    async fn clearing_finished_jobs_preserves_running_jobs() {
        let manager = JobManager::default();
        let finished_id = manager.spawn(
            "test",
            "Finished job",
            JobMetadata::new(),
            |_context| async { Ok(JobMetadata::new()) },
        );
        let running_id = manager.spawn(
            "test",
            "Running job",
            JobMetadata::new(),
            |context| async move {
                loop {
                    if context.is_cancelled() {
                        return Err(context.cancelled_failure());
                    }

                    sleep(Duration::from_millis(5)).await;
                }
            },
        );

        wait_for_terminal(&manager, &finished_id).await;
        wait_for_status(&manager, &running_id, JobStatus::Running).await;

        assert_eq!(manager.clear_finished(), 1);
        assert!(manager.get_job(&finished_id).is_none());
        assert_eq!(
            manager.get_job(&running_id).map(|snapshot| snapshot.status),
            Some(JobStatus::Running)
        );

        manager.cancel_job(&running_id).unwrap();
        assert_eq!(
            wait_for_terminal(&manager, &running_id).await.status,
            JobStatus::Cancelled
        );
    }

    async fn wait_for_terminal(manager: &JobManager, id: &JobId) -> JobSnapshot {
        timeout(Duration::from_secs(2), async {
            loop {
                if let Some(snapshot) = manager.get_job(id) {
                    if snapshot.status.is_terminal() {
                        return snapshot;
                    }
                }

                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("job reached terminal status")
    }

    async fn wait_for_status(manager: &JobManager, id: &JobId, status: JobStatus) -> JobSnapshot {
        timeout(Duration::from_secs(2), async {
            loop {
                if let Some(snapshot) = manager.get_job(id) {
                    if snapshot.status == status {
                        return snapshot;
                    }
                }

                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("job reached expected status")
    }

    async fn drain_until_terminal(receiver: &mut broadcast::Receiver<JobEvent>) -> Vec<JobEvent> {
        timeout(Duration::from_secs(2), async {
            let mut events = Vec::new();

            loop {
                let event = receiver.recv().await.expect("job event");
                let terminal = event.status.is_terminal();
                events.push(event);

                if terminal {
                    return events;
                }
            }
        })
        .await
        .expect("terminal event received")
    }
}
