use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use dm_api::{
    ContentCount, ContentQuery, Credentials, DlsiteClient, DmApiError, DownloadFile, DownloadPlan,
    Language, LocalizedText, Purchase, Work, WorkId,
};
use dm_credentials::{CredentialRef, CredentialStore, CredentialsError};
use dm_download::{
    DownloadFileMetadata, DownloadJobRequest, DownloadProgress, DownloadedWork, UnpackPolicy,
};
pub use dm_jobs::CancellationToken;
use dm_storage::{
    Account, AccountSyncCommit, AccountUpsert, AccountWork, CachedWork, ProductListPage,
    ProductListQuery, Storage, StorageError, SyncCancellation, SyncFailure, WorkDownloadState,
    WorkDownloadStatus, WorkDownloadUpdate,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, LibraryError>;

const BULK_DOWNLOAD_PAGE_LIMIT: u32 = 500;
const DOWNLOAD_CANCELLATION_POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(50);

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
    #[error("sync was cancelled")]
    Cancelled,
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
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("json error")]
    Json(#[from] serde_json::Error),
}

impl LibraryError {
    fn failure_code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Credentials(_) => "credentials",
            Self::Api(_) => "api",
            Self::SyncSource(_) => "sync_source",
            Self::AccountNotFound(_) => "account_not_found",
            Self::AccountDisabled(_) => "account_disabled",
            Self::MissingLoginName(_) => "missing_login_name",
            Self::MissingPassword(_) => "missing_password",
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
}

#[derive(Clone)]
pub struct Library {
    storage: Storage,
    credentials: Arc<dyn CredentialStore>,
}

impl Library {
    pub fn new(storage: Storage, credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            storage,
            credentials,
        }
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub async fn accounts(&self) -> Result<Vec<Account>> {
        Ok(self.storage.accounts().await?)
    }

    pub async fn save_account(&self, request: SaveAccountRequest) -> Result<Account> {
        let existing_account = match request.id.as_deref() {
            Some(account_id) => match self.find_account(account_id).await {
                Ok(account) => Some(account),
                Err(LibraryError::AccountNotFound(_)) => None,
                Err(error) => return Err(error),
            },
            None => None,
        };
        let account_id = request
            .id
            .unwrap_or_else(|| format!("account-{}", Uuid::new_v4()));
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
            enabled: request.enabled,
        };

        self.storage.save_account(&account).await?;
        self.find_account(&account_id).await
    }

    pub async fn set_account_enabled(&self, account_id: &str, enabled: bool) -> Result<()> {
        Ok(self
            .storage
            .set_account_enabled(account_id, enabled)
            .await?)
    }

    pub async fn list_products(&self, query: &ProductListQuery) -> Result<ProductListPage> {
        Ok(self.storage.list_products(query).await?)
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
        let staging_dir = request.download_root.join(request.work_id);
        let final_dir = request.library_root.join(request.work_id);
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
            let completed_at = now_string();
            let status = if matches!(error, LibraryError::Cancelled)
                || matches!(
                    error,
                    LibraryError::Download(dm_download::DownloadError::Cancelled)
                ) {
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
                    error_message: Some(error.to_string()),
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
            request.check_cancelled()?;
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
                    });
                    request.emit(BulkWorkDownloadProgress::WorkCompleted {
                        work_id,
                        current,
                        total: requested_count,
                    });
                }
                Err(error)
                    if matches!(error, LibraryError::Cancelled)
                        || matches!(
                            error,
                            LibraryError::Download(dm_download::DownloadError::Cancelled)
                        ) =>
                {
                    return Err(error);
                }
                Err(error) => {
                    let error_code = error.failure_code().to_owned();
                    let error_message = error.to_string();

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
                preview.total_expected_bytes = match preview.total_expected_bytes {
                    Some(total) => Some(total.saturating_add(content_size_bytes)),
                    None => None,
                };
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
            if matches!(error, LibraryError::Cancelled) {
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
                        error_message: Some(error.to_string()),
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

        let login_name = account
            .login_name
            .as_deref()
            .ok_or_else(|| LibraryError::MissingLoginName(account.id.clone()))?;
        let password = self.password_for_account(account, request.password)?;
        let credentials = Credentials::new(login_name, password);

        source.login(&credentials).await?;
        self.storage
            .record_account_login(&account.id, &now_string())
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

        let login_name = account
            .login_name
            .as_deref()
            .ok_or_else(|| LibraryError::MissingLoginName(account.id.clone()))?;
        let password = self.password_for_account(account, request.password)?;
        let credentials = Credentials::new(login_name, password);

        source.login(&credentials).await?;
        self.storage
            .record_account_login(&account.id, &now_string())
            .await?;

        request.check_cancelled()?;
        request.emit(WorkDownloadProgress::ResolvingPlan);
        let plan = source.download_plan(work_id).await?;

        request.check_cancelled()?;
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
        if request.replace_existing {
            remove_existing_download_path(final_dir, &[request.library_root]).await?;
        }
        move_downloaded_work_dir(staging_dir, final_dir).await?;

        let completed_at = now_string();
        let bytes_received = downloaded
            .files
            .iter()
            .map(|file| file.bytes_written)
            .sum::<u64>();

        self.storage
            .save_work_download(&WorkDownloadUpdate {
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
            })
            .await?;

        request.emit(WorkDownloadProgress::Completed);

        Ok(WorkDownloadReport {
            work_id: request.work_id.to_owned(),
            account_id: account.id.clone(),
            local_path: final_dir.to_path_buf(),
            file_count: downloaded.files.len(),
            archive_extracted: downloaded.archive_extraction.is_some(),
            download_state: self.storage.work_download_state(request.work_id).await?,
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

    fn save_account_credential(
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAccountRequest {
    pub id: Option<String>,
    pub label: String,
    pub login_name: Option<String>,
    pub password: Option<String>,
    pub remember_password: bool,
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
}

impl<'a> AccountSyncRequest<'a> {
    pub fn new(account_id: &'a str) -> Self {
        Self {
            account_id,
            password: None,
            cancellation_token: None,
            progress_sink: None,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkDownloadReport {
    pub work_id: String,
    pub account_id: String,
    pub local_path: PathBuf,
    pub file_count: usize,
    pub archive_extracted: bool,
    pub download_state: WorkDownloadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkDownloadProgress {
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
pub trait WorkDownloadSource {
    async fn login(&self, credentials: &Credentials) -> Result<()>;
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
impl WorkDownloadSource for DlsiteWorkDownloadSource {
    async fn login(&self, credentials: &Credentials) -> Result<()> {
        self.client.login(credentials).await?;
        Ok(())
    }

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
pub trait AccountSyncSource {
    async fn login(&self, credentials: &Credentials) -> Result<()>;
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
impl AccountSyncSource for DlsiteSyncSource {
    async fn login(&self, credentials: &Credentials) -> Result<()> {
        self.client.login(credentials).await?;
        Ok(())
    }

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

fn preferred_localized_text(text: &LocalizedText) -> Option<&String> {
    text.get(&Language::English)
        .or_else(|| text.get(&Language::Japanese))
        .or_else(|| text.get(&Language::Korean))
        .or_else(|| text.get(&Language::Taiwanese))
        .or_else(|| text.get(&Language::Chinese))
        .or_else(|| text.values().next())
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

async fn move_downloaded_work_dir(source: &Path, destination: &Path) -> Result<()> {
    if destination.try_exists()? {
        return Err(LibraryError::DownloadTargetExists(
            destination.to_path_buf(),
        ));
    }

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    match tokio::fs::rename(source, destination).await {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            let source = source.to_path_buf();
            let destination = destination.to_path_buf();
            tokio::task::spawn_blocking(move || {
                copy_dir_recursively(&source, &destination)?;
                std::fs::remove_dir_all(&source)?;
                Ok::<(), std::io::Error>(())
            })
            .await
            .map_err(|err| LibraryError::Io(std::io::Error::other(err)))??;
            drop(rename_error);
            Ok(())
        }
    }
}

fn copy_dir_recursively(source: &Path, destination: &Path) -> std::io::Result<()> {
    if destination.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("destination already exists: {}", destination.display()),
        ));
    }

    std::fs::create_dir_all(destination)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let entry_source = entry.path();
        let entry_destination = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursively(&entry_source, &entry_destination)?;
        } else if file_type.is_file() {
            std::fs::copy(&entry_source, &entry_destination)?;
        }
    }

    Ok(())
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
        WorkKind, WorkThumbnail,
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

    #[async_trait]
    impl AccountSyncSource for FakeSyncSource {
        async fn login(&self, credentials: &Credentials) -> Result<()> {
            assert_eq!(credentials.username, "user@example.test");
            assert_eq!(credentials.password, "secret");

            if self.fail_at == Some(FakeFailurePoint::Login) {
                return Err(LibraryError::SyncSource("login failed".to_owned()));
            }

            Ok(())
        }

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
    struct FakeDownloadSource;

    #[async_trait]
    impl WorkDownloadSource for FakeDownloadSource {
        async fn login(&self, credentials: &Credentials) -> Result<()> {
            assert_eq!(credentials.username, "user@example.test");
            assert_eq!(credentials.password, "secret");
            Ok(())
        }

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

    #[async_trait]
    impl WorkDownloadSource for WaitingDownloadSource {
        async fn login(&self, credentials: &Credentials) -> Result<()> {
            assert_eq!(credentials.username, "user@example.test");
            assert_eq!(credentials.password, "secret");
            Ok(())
        }

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

    #[async_trait]
    impl WorkDownloadSource for FailingDownloadSource {
        async fn login(&self, credentials: &Credentials) -> Result<()> {
            FakeDownloadSource.login(credentials).await
        }

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
        let events = sink.events.lock().expect("download events lock");

        assert_eq!(report.work_id, "RJ000001");
        assert_eq!(report.account_id, "account-a");
        assert_eq!(report.file_count, 1);
        assert_eq!(report.download_state.status, WorkDownloadStatus::Downloaded);
        assert!(library_root.join("RJ000001/RJ000001.txt").exists());
        assert!(!download_root.join("RJ000001").exists());
        assert_eq!(
            page.products[0].download.status,
            WorkDownloadStatus::Downloaded
        );
        assert_eq!(
            page.products[0].download.local_path,
            Some(library_root.join("RJ000001").to_string_lossy().into_owned())
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
        assert_eq!(
            page.products[0].download.status,
            WorkDownloadStatus::Downloaded
        );

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
            &[root.clone()]
        ));
        assert!(!path_is_download_child_of_any_root(&root, &[root.clone()]));
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
                },
                &sync_source(),
            )
            .await?;

        assert_eq!(report.cached_work_count, 2);

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
