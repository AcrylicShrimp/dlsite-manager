use super::*;

const TWO_FACTOR_REQUEST_EVENT: &str = "dm-two-factor-request";
const TWO_FACTOR_CLOSED_EVENT: &str = "dm-two-factor-closed";
/// How long a two-factor dialog stays open before the job gives up waiting.
const TWO_FACTOR_PROMPT_TIMEOUT: Duration = Duration::from_secs(300);

/// Open two-factor dialogs, keyed by request ID.
///
/// A job parks on the receiver while the UI shows the dialog; `submit_two_factor_code` and
/// `cancel_two_factor` complete it from the command side.
#[derive(Clone, Default)]
pub(super) struct TwoFactorPrompts {
    inner: Arc<Mutex<TwoFactorPromptsInner>>,
}

#[derive(Default)]
struct TwoFactorPromptsInner {
    next_id: u64,
    pending: BTreeMap<String, tokio::sync::oneshot::Sender<Option<String>>>,
}

impl TwoFactorPrompts {
    fn open(&self) -> (String, tokio::sync::oneshot::Receiver<Option<String>>) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mut inner = self.inner.lock().expect("two-factor prompt lock");

        inner.next_id += 1;

        let request_id = format!("two-factor-{}", inner.next_id);

        inner.pending.insert(request_id.clone(), sender);

        (request_id, receiver)
    }

    /// Answers an open dialog. Returns whether the request was still open.
    pub(super) fn answer(&self, request_id: &str, code: Option<String>) -> bool {
        let sender = self
            .inner
            .lock()
            .expect("two-factor prompt lock")
            .pending
            .remove(request_id);

        match sender {
            Some(sender) => sender.send(code).is_ok(),
            None => false,
        }
    }

    fn close(&self, request_id: &str) {
        self.inner
            .lock()
            .expect("two-factor prompt lock")
            .pending
            .remove(request_id);
    }
}

/// Two-factor prompt that runs a dialog in the app window on behalf of a background job.
pub(super) struct JobTwoFactorPrompt {
    app: AppHandle,
    prompts: TwoFactorPrompts,
    context: JobContext,
}

/// Cleanup also runs when the core cancels and drops the prompt future.
struct PromptCleanup {
    prompts: TwoFactorPrompts,
    request_id: String,
    on_close: Box<dyn Fn(&str) + Send + Sync>,
}

impl Drop for PromptCleanup {
    fn drop(&mut self) {
        self.prompts.close(&self.request_id);
        (self.on_close)(&self.request_id);
    }
}

impl JobTwoFactorPrompt {
    pub(super) fn new(app: AppHandle, prompts: TwoFactorPrompts, context: JobContext) -> Self {
        Self {
            app,
            prompts,
            context,
        }
    }
}

#[async_trait::async_trait]
impl dm_library::TwoFactorPrompt for JobTwoFactorPrompt {
    async fn request_code(
        &self,
        request: dm_library::TwoFactorPromptRequest,
    ) -> Result<dm_library::TwoFactorPromptResponse, dm_library::LibraryError> {
        let (request_id, receiver) = self.prompts.open();
        let app = self.app.clone();
        let _cleanup = PromptCleanup {
            prompts: self.prompts.clone(),
            request_id: request_id.clone(),
            on_close: Box::new(move |request_id| {
                let _ = app.emit(TWO_FACTOR_CLOSED_EVENT, json!({ "requestId": request_id }));
            }),
        };

        self.context.set_phase("waitingForTwoFactor");
        self.context.clear_progress();
        self.context.info(format!(
            "Waiting for a two-factor code for account {}",
            request.account_id
        ));

        if let Err(error) = self.app.emit(
            TWO_FACTOR_REQUEST_EVENT,
            json!({
                "requestId": request_id,
                "accountId": request.account_id,
                "accountLabel": request.account_label,
                "attempt": request.attempt,
                "previousCodeRejected": request.previous_code_rejected,
                "jobId": self.context.job_id().as_str(),
            }),
        ) {
            self.prompts.close(&request_id);
            return Err(dm_library::LibraryError::SyncSource(format!(
                "could not show the two-factor dialog: {error}"
            )));
        }

        // Cancellation and timeout stay distinct: a cancelled job finishes as Cancelled,
        // while a prompt nobody answered is a failure the user still needs to see.
        let response = wait_for_response(
            receiver,
            self.context.cancellation_token(),
            TWO_FACTOR_PROMPT_TIMEOUT,
        )
        .await;
        if response == dm_library::TwoFactorPromptResponse::TimedOut {
            self.context.warn("Two-factor code was not entered in time");
        }

        Ok(response)
    }
}

async fn wait_for_response(
    receiver: tokio::sync::oneshot::Receiver<Option<String>>,
    cancellation: &dm_library::CancellationToken,
    timeout: Duration,
) -> dm_library::TwoFactorPromptResponse {
    tokio::select! {
        biased;
        () = wait_for_cancellation(cancellation) => dm_library::TwoFactorPromptResponse::Cancelled,
        answer = receiver => match answer {
            Ok(Some(code)) => dm_library::TwoFactorPromptResponse::Code(code),
            // `None` is the dialog's Cancel button; a dropped sender means the request
            // was closed out from under the dialog, which the job cannot act on either.
            Ok(None) | Err(_) => dm_library::TwoFactorPromptResponse::Cancelled,
        },
        () = tokio::time::sleep(timeout) => {
            dm_library::TwoFactorPromptResponse::TimedOut
        }
    }
}

async fn wait_for_cancellation(cancellation: &dm_library::CancellationToken) {
    while !cancellation.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_library::{CancellationToken, TwoFactorPromptResponse as Response};

    #[tokio::test]
    async fn prompt_answers_cancellation_and_timeout_stay_distinct() {
        let prompts = TwoFactorPrompts::default();
        let token = CancellationToken::new();
        let (id, rx) = prompts.open();
        assert!(prompts.answer(&id, Some("123456".to_owned())));
        assert!(!prompts.answer(&id, Some("654321".to_owned())));
        assert_eq!(
            wait_for_response(rx, &token, Duration::from_secs(1)).await,
            Response::Code("123456".to_owned())
        );
        let (id, rx) = prompts.open();
        prompts.answer(&id, None);
        assert_eq!(
            wait_for_response(rx, &token, Duration::from_secs(1)).await,
            Response::Cancelled
        );
        let (id, rx) = prompts.open();
        assert_eq!(
            wait_for_response(rx, &token, Duration::from_millis(1)).await,
            Response::TimedOut
        );
        prompts.close(&id);
        let (id, rx) = prompts.open();
        token.cancel();
        assert_eq!(
            wait_for_response(rx, &token, Duration::from_secs(1)).await,
            Response::Cancelled
        );
        prompts.close(&id);
    }

    #[tokio::test]
    async fn dropping_prompt_wait_closes_only_its_registry_entry_and_emits_closure() {
        let prompts = TwoFactorPrompts::default();
        let (id, rx) = prompts.open();
        let (other, other_rx) = prompts.open();
        let closed = Arc::new(Mutex::new(Vec::new()));
        let observed = closed.clone();
        let guard = PromptCleanup {
            prompts: prompts.clone(),
            request_id: id.clone(),
            on_close: Box::new(move |id| observed.lock().unwrap().push(id.to_owned())),
        };
        let mut pending = Box::pin(async move {
            let _guard = guard;
            let _ = rx.await;
        });
        // Poll once, then drop the owning future, just like core auth cancellation.
        assert!(tokio::time::timeout(Duration::from_millis(1), &mut pending)
            .await
            .is_err());
        assert_eq!(prompts.inner.lock().unwrap().pending.len(), 2);
        drop(pending);
        assert!(!prompts.answer(&id, None));
        assert_eq!(*closed.lock().unwrap(), vec![id]);
        assert!(prompts.answer(&other, Some("654321".to_owned())));
        assert_eq!(other_rx.await.unwrap().as_deref(), Some("654321"));
    }
}
