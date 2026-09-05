use super::*;

const MAX_TWO_FACTOR_ATTEMPTS: u32 = 3;

impl Library {
    pub(super) fn save_account_credential(
        &self,
        account_id: &str,
        password: Option<&str>,
        remember_password: bool,
        existing_credential_ref: Option<CredentialRef>,
    ) -> Result<Option<CredentialRef>> {
        let credential_ref =
            existing_credential_ref.or(Some(CredentialRef::account_password(account_id)?));

        if remember_password {
            let credential_ref = credential_ref.ok_or_else(|| {
                LibraryError::Credentials(CredentialsError::InvalidCredentialRef(
                    "invalid account id",
                ))
            })?;

            if let Some(password) = password {
                self.credentials.save_password(&credential_ref, password)?;
            } else if self.credentials.load_password(&credential_ref)?.is_none() {
                return Err(LibraryError::MissingPassword(account_id.to_owned()));
            }

            Ok(Some(credential_ref))
        } else {
            if let Some(credential_ref) = credential_ref {
                self.credentials.delete_password(&credential_ref)?;
            }

            Ok(None)
        }
    }

    fn password_for_account(&self, account: &Account, password: Option<&str>) -> Result<String> {
        if let Some(password) = password {
            return Ok(password.to_owned());
        }

        let credential_ref = account
            .credential_ref
            .as_deref()
            .ok_or_else(|| LibraryError::MissingPassword(account.id.clone()))
            .and_then(|value| CredentialRef::new(value.to_owned()).map_err(Into::into))?;

        self.credentials
            .load_password(&credential_ref)?
            .ok_or_else(|| LibraryError::MissingPassword(account.id.clone()))
    }

    /// Brings `source` to an authorized state for `account`.
    ///
    /// A stored session is tried first so that two-factor accounts are not asked for a code
    /// on every job — TOTP codes are single-use, and login happens once per sync and once
    /// per download. Only when no usable session exists does this fall back to a password
    /// login, and only a two-factor-enabled account reaches `prompt`.
    pub(super) async fn authenticate_account<S>(
        &self,
        account: &Account,
        password: Option<&str>,
        two_factor_prompt: Option<&dyn TwoFactorPrompt>,
        cancellation_token: Option<&CancellationToken>,
        source: &S,
    ) -> Result<()>
    where
        S: DlsiteAuthSource + Sync,
    {
        if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
            return Err(LibraryError::Cancelled);
        }

        let login_lock = self.login_lock(&account.id);
        let _login_guard = acquire_login_lock(&login_lock, cancellation_token).await?;
        let account = &self.find_account(&account.id).await?;
        if !account.enabled {
            return Err(LibraryError::AccountDisabled(account.id.clone()));
        }
        let session_ref = CredentialRef::account_session(&account.id)?;

        if account.credential_ref.is_none() {
            self.credentials.delete_password(&session_ref)?;
        }
        if self
            .restore_stored_session(&session_ref, cancellation_token, source)
            .await?
        {
            check_cancelled(cancellation_token)?;
            self.storage
                .record_account_login(&account.id, &now_string())
                .await?;
            return Ok(());
        }

        let login_name = account
            .login_name
            .as_deref()
            .ok_or_else(|| LibraryError::MissingLoginName(account.id.clone()))?;
        let password = self.password_for_account(account, password)?;
        let credentials = Credentials::new(login_name, password);

        let session = match cancellable(source.begin_login(&credentials), cancellation_token)
            .await?
        {
            LoginOutcome::Authorized(session) => session,
            LoginOutcome::TwoFactorRequired(challenge) => {
                let prompt = two_factor_prompt
                    .ok_or_else(|| LibraryError::TwoFactorPromptUnavailable(account.id.clone()))?;

                self.complete_two_factor_login(
                    account,
                    &challenge,
                    prompt,
                    cancellation_token,
                    source,
                )
                .await?
            }
        };

        // A session cookie grants what the password grants, so it follows the account's
        // remember-password setting. An account that does not keep its password -- which is
        // what an absent credential ref means -- does not keep a session either, and any
        // session stored before that setting changed is dropped here.
        check_cancelled(cancellation_token)?;
        if account.credential_ref.is_some() {
            self.credentials
                .save_password(&session_ref, &session.cookies_json)?;
        } else {
            self.credentials.delete_password(&session_ref)?;
        }

        self.storage
            .record_account_login(&account.id, &now_string())
            .await?;

        Ok(())
    }

    pub(super) fn login_lock(&self, account_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.login_locks
            .lock()
            .expect("login lock registry is poisoned")
            .entry(account_id.to_owned())
            .or_default()
            .clone()
    }

    /// Returns whether a stored session was restored and is still authorized. A session that
    /// no longer works is discarded so the next attempt does not retry it.
    async fn restore_stored_session<S>(
        &self,
        session_ref: &CredentialRef,
        cancellation_token: Option<&CancellationToken>,
        source: &S,
    ) -> Result<bool>
    where
        S: DlsiteAuthSource + Sync,
    {
        let Some(cookies_json) = self.credentials.load_password(session_ref)? else {
            return Ok(false);
        };

        let session = SessionSnapshot { cookies_json };

        let restored = match cancellable(source.restore_session(&session), cancellation_token).await
        {
            Ok(()) => true,
            Err(error) if error.is_cancelled() => return Err(error),
            Err(_) => false,
        };
        if restored && cancellable(source.validate_session(), cancellation_token).await? {
            return Ok(true);
        }

        self.credentials.delete_password(session_ref)?;

        Ok(false)
    }

    async fn complete_two_factor_login<S>(
        &self,
        account: &Account,
        challenge: &TwoFactorChallenge,
        prompt: &dyn TwoFactorPrompt,
        cancellation_token: Option<&CancellationToken>,
        source: &S,
    ) -> Result<SessionSnapshot>
    where
        S: DlsiteAuthSource + Sync,
    {
        let mut previous_code_rejected = false;

        for attempt in 1..=MAX_TWO_FACTOR_ATTEMPTS {
            let response = cancellable(
                prompt.request_code(TwoFactorPromptRequest {
                    account_id: account.id.clone(),
                    account_label: account.label.clone(),
                    attempt,
                    previous_code_rejected,
                }),
                cancellation_token,
            )
            .await?;

            let code = match response {
                TwoFactorPromptResponse::Code(code) => code,
                TwoFactorPromptResponse::Cancelled => {
                    return Err(LibraryError::TwoFactorCancelled(account.id.clone()))
                }
                TwoFactorPromptResponse::TimedOut => {
                    return Err(LibraryError::TwoFactorTimedOut(account.id.clone()))
                }
            };

            match cancellable(
                source.complete_two_factor(challenge, &code),
                cancellation_token,
            )
            .await
            {
                Ok(session) => return Ok(session),
                Err(LibraryError::Api(DmApiError::InvalidTwoFactorCode)) => {
                    previous_code_rejected = true;
                }
                Err(error) => return Err(error),
            }
        }

        Err(LibraryError::TwoFactorRejected {
            account_id: account.id.clone(),
            attempts: MAX_TWO_FACTOR_ATTEMPTS,
        })
    }

    /// Drops any stored session for an account, so a removed or re-credentialed account does
    /// not leave a usable session behind.
    pub(super) fn forget_account_session(&self, account_id: &str) -> Result<()> {
        let session_ref = CredentialRef::account_session(account_id)?;

        self.credentials.delete_password(&session_ref)?;

        Ok(())
    }
}

/// Authentication surface shared by every source that talks to DLsite on behalf of an
/// account.
///
/// It is split from the feature traits because logging in is identical for sync and
/// download, and because two-factor login is a multi-step conversation: the caller has to be
/// able to restore a stored session, notice it is stale, and resume a challenge with a code
/// obtained from the user.
#[async_trait]
pub trait DlsiteAuthSource {
    async fn restore_session(&self, session: &SessionSnapshot) -> Result<()>;
    async fn validate_session(&self) -> Result<bool>;
    async fn begin_login(&self, credentials: &Credentials) -> Result<LoginOutcome>;
    async fn complete_two_factor(
        &self,
        challenge: &TwoFactorChallenge,
        code: &str,
    ) -> Result<SessionSnapshot>;
}

/// Context handed to the UI when a two-factor code is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoFactorPromptRequest {
    pub account_id: String,
    pub account_label: String,
    /// 1 for the first prompt of this login.
    pub attempt: u32,
    pub previous_code_rejected: bool,
}

/// How often a task waiting on the per-account login lock re-checks its cancellation token.
/// `CancellationToken` is a flag rather than a future, so waiting on it means polling.
const LOGIN_LOCK_CANCELLATION_POLL: Duration = Duration::from_millis(200);

/// Waits for the per-account login lock while still observing cancellation.
///
/// The holder may be sitting on an open two-factor dialog, so a queued job that waits here
/// unconditionally would not react to Cancel until that dialog's own timeout expired.
pub(super) async fn acquire_login_lock<'a>(
    lock: &'a tokio::sync::Mutex<()>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<tokio::sync::MutexGuard<'a, ()>> {
    cancellable(async { Ok(lock.lock().await) }, cancellation_token).await
}

fn check_cancelled(token: Option<&CancellationToken>) -> Result<()> {
    if token.is_some_and(CancellationToken::is_cancelled) {
        Err(LibraryError::Cancelled)
    } else {
        Ok(())
    }
}

async fn cancellable<T>(
    future: impl std::future::Future<Output = Result<T>>,
    token: Option<&CancellationToken>,
) -> Result<T> {
    check_cancelled(token)?;
    let Some(token) = token else {
        return future.await;
    };
    tokio::select! {
        biased;
        () = wait_for_cancellation(token) => Err(LibraryError::Cancelled),
        result = future => { check_cancelled(Some(token))?; result }
    }
}

async fn wait_for_cancellation(cancellation_token: &CancellationToken) {
    while !cancellation_token.is_cancelled() {
        tokio::time::sleep(LOGIN_LOCK_CANCELLATION_POLL).await;
    }
}

/// How a two-factor prompt ended.
///
/// Cancelling and timing out are kept apart because they mean different things to the job
/// that asked: a cancelled job finishes as Cancelled, while a prompt nobody answered is a
/// failure the user still needs to see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwoFactorPromptResponse {
    Code(String),
    /// The user dismissed the prompt, or the job was cancelled underneath it.
    Cancelled,
    /// The prompt expired before anyone answered it.
    TimedOut,
}

#[async_trait]
pub trait TwoFactorPrompt: Send + Sync {
    async fn request_code(
        &self,
        request: TwoFactorPromptRequest,
    ) -> Result<TwoFactorPromptResponse>;
}
