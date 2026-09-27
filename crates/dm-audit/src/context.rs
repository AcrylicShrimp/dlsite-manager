use crate::{AuditEvent, AuditLogger, AuditOutcome};
use std::{future::Future, time::Instant};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct OperationContext {
    pub id: String,
    pub parent: Option<String>,
    pub job_id: Option<String>,
}
tokio::task_local! {static CURRENT:(OperationContext,AuditLogger);}
pub(crate) fn current() -> Option<OperationContext> {
    CURRENT.try_with(|(context, _)| context.clone()).ok()
}

pub struct Operation {
    logger: AuditLogger,
    name: &'static str,
    context: OperationContext,
    started: Instant,
    finished: bool,
}
impl Operation {
    pub(crate) fn new(logger: AuditLogger, name: &'static str, job_id: Option<String>) -> Self {
        let parent = current();
        let context = OperationContext {
            id: format!("op-{}", Uuid::new_v4()),
            parent: parent.as_ref().map(|p| p.id.clone()),
            job_id: job_id
                .or_else(|| parent.and_then(|p| p.job_id))
                .filter(|id| crate::privacy::valid_id(id)),
        };
        let op = Self {
            logger,
            name,
            context,
            started: Instant::now(),
            finished: false,
        };
        let mut event = AuditEvent::queued(name, "");
        event.kind = "started".into();
        event.audience = "support".into();
        op.emit(event);
        op
    }
    pub fn id(&self) -> &str {
        &self.context.id
    }
    pub fn context(&self) -> OperationContext {
        self.context.clone()
    }
    pub async fn scope<F: Future>(&self, future: F) -> F::Output {
        CURRENT
            .scope((self.context.clone(), self.logger.clone()), future)
            .await
    }
    pub fn finish(&mut self, outcome: AuditOutcome, details: serde_json::Value) {
        if self.finished {
            return;
        }
        self.finished = true;
        let mut event = AuditEvent::new(
            if outcome == AuditOutcome::Failed {
                crate::AuditLevel::Error
            } else {
                crate::AuditLevel::Info
            },
            self.name,
            outcome,
            "",
        );
        event.kind = "finished".into();
        event.duration_ms = Some(self.started.elapsed().as_millis().min(u64::MAX as u128) as u64);
        event.details = details;
        if self.name.starts_with("get_") || self.name.starts_with("list_") {
            event.audience = "support".into();
        }
        self.emit(event);
    }
    pub async fn command<T, F>(mut self, future: F) -> std::result::Result<T, String>
    where
        F: Future<Output = std::result::Result<T, String>>,
    {
        let result = self.scope(future).await;
        self.finish(
            if result.is_ok() {
                AuditOutcome::Succeeded
            } else {
                AuditOutcome::Failed
            },
            serde_json::json!({}),
        );
        result.map_err(|message| {
            format!(
                "{message} [diagnostic:{}:{}]",
                self.logger.run_id(),
                self.id()
            )
        })
    }
    fn emit(&self, mut event: AuditEvent) {
        event.operation_id = Some(self.context.id.clone());
        event.parent_operation_id = self.context.parent.clone();
        event.job_id = self.context.job_id.clone();
        self.logger.record_now(event);
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if !self.finished {
            self.finish(AuditOutcome::Unknown, serde_json::json!({}));
        }
    }
}

/// Record a typed cause while its command/job context is still available.
pub fn record_current_failure(code: &str, details: serde_json::Value) {
    let _ = CURRENT.try_with(|(_, logger)| {
        let mut event = AuditEvent::failed("command", "")
            .with_error(Some(code), "")
            .with_details(details);
        event.audience = "support".into();
        event.stage = "cause".into();
        logger.record_now(event);
    });
}
