use dm_audit::{AuditEvent, AuditLogger};
use dm_credentials::{CredentialStore, LocalCredentialStore};
use dm_jobs::{
    JobContext, JobEventKind, JobFailure, JobId, JobLogPage, JobManager, JobMetadata, JobProgress,
    JobStatus,
};
use dm_library::{
    AccountRemovalReport, AccountSyncRequest, BulkWorkDownloadPreview,
    BulkWorkDownloadPreviewProgress, BulkWorkDownloadPreviewProgressSink,
    BulkWorkDownloadPreviewRequest, BulkWorkDownloadProgress, BulkWorkDownloadProgressSink,
    BulkWorkDownloadReport, BulkWorkDownloadRequest, DlsitePublicMetadataSource, DlsiteSyncSource,
    DlsiteWorkDownloadSource, Library, LocalWorkImportReport, LocalWorkImportRequest,
    SaveAccountRequest, SyncProgress, SyncProgressSink, WorkDownloadMarkRequest,
    WorkDownloadProgress, WorkDownloadProgressSink, WorkDownloadRemovalRequest,
    WorkDownloadRequest,
};
use dm_storage::{
    Account, AppSettings, ProductAgeCategory, ProductCreditGroup, ProductCustomTag,
    ProductCustomTagFacet, ProductDetail, ProductFilterFacets, ProductListItem, ProductListPage,
    ProductListQuery, ProductMakerFacet, ProductOwner, ProductSort, ProductSourceGroup, ProductTag,
    ProductTextValue, ProductTypeGroup, Storage, WorkDownloadState, WorkDownloadStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::broadcast::error::RecvError;

struct AppState {
    storage: Storage,
    library: Library,
    jobs: JobManager,
    audit: AuditLogger,
    download_reservations: DownloadReservations,
    _tracing_guard: tracing_appender::non_blocking::WorkerGuard,
}

const WORK_DOWNLOAD_PROGRESS_EVENT_INTERVAL: Duration = Duration::from_secs(1);
const BULK_DOWNLOAD_PAGE_LIMIT: u32 = 500;
const DOWNLOAD_RESERVATION_METADATA_KEY: &str = "downloadReservationId";

#[derive(Clone, Default)]
struct DownloadReservations {
    inner: Arc<Mutex<DownloadReservationsInner>>,
}

#[derive(Default)]
struct DownloadReservationsInner {
    next_owner: u64,
    by_work_id: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadReservationSplit {
    available: Vec<String>,
    reserved: Vec<String>,
}

impl DownloadReservations {
    fn next_owner_id(&self) -> String {
        let mut inner = self.inner.lock().expect("download reservations lock");

        inner.next_owner = inner.next_owner.saturating_add(1);
        format!("download-reservation-{}", inner.next_owner)
    }

    fn split_available(&self, work_ids: &[String]) -> DownloadReservationSplit {
        let inner = self.inner.lock().expect("download reservations lock");
        let mut seen = BTreeSet::new();
        let mut available = Vec::new();
        let mut reserved = Vec::new();

        for work_id in work_ids {
            if !seen.insert(work_id.as_str()) {
                continue;
            }

            if inner.by_work_id.contains_key(work_id) {
                reserved.push(work_id.clone());
            } else {
                available.push(work_id.clone());
            }
        }

        DownloadReservationSplit {
            available,
            reserved,
        }
    }

    fn claim_available(&self, work_ids: &[String], owner_id: &str) -> DownloadReservationSplit {
        let mut inner = self.inner.lock().expect("download reservations lock");
        let mut seen = BTreeSet::new();
        let mut available = Vec::new();
        let mut reserved = Vec::new();

        for work_id in work_ids {
            if !seen.insert(work_id.as_str()) {
                continue;
            }

            if inner.by_work_id.contains_key(work_id) {
                reserved.push(work_id.clone());
            } else {
                inner
                    .by_work_id
                    .insert(work_id.clone(), owner_id.to_owned());
                available.push(work_id.clone());
            }
        }

        DownloadReservationSplit {
            available,
            reserved,
        }
    }

    fn release_owner(&self, owner_id: &str) -> usize {
        let mut inner = self.inner.lock().expect("download reservations lock");
        let before = inner.by_work_id.len();

        inner
            .by_work_id
            .retain(|_, reservation_owner| reservation_owner != owner_id);

        before.saturating_sub(inner.by_work_id.len())
    }

    #[cfg(test)]
    fn is_reserved(&self, work_id: &str) -> bool {
        self.inner
            .lock()
            .expect("download reservations lock")
            .by_work_id
            .contains_key(work_id)
    }
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, String> {
    state
        .storage
        .app_settings()
        .await
        .map(AppSettingsDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    settings: SaveSettingsRequest,
) -> Result<AppSettingsDto, String> {
    let settings = match settings.into_app_settings() {
        Ok(settings) => settings,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("settings.save", "Failed to validate settings")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let result = state.storage.save_app_settings(&settings).await;

    match result {
        Ok(()) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("settings.save", "Saved settings").with_details(json!({
                    "libraryRootSet": settings.library_root.is_some(),
                    "downloadRootSet": settings.download_root.is_some(),
                })),
            )
            .await;
            Ok(AppSettingsDto::from(settings))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("settings.save", "Failed to save settings")
                    .with_error(Some("storage"), message.clone()),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountDto>, String> {
    let accounts = state.library.accounts().await.map_err(command_error)?;
    let mut dtos = Vec::with_capacity(accounts.len());

    for account in accounts {
        let has_credential = state
            .library
            .account_has_saved_password(&account)
            .map_err(command_error)?;

        dtos.push(AccountDto::from_account(account, has_credential));
    }

    Ok(dtos)
}

#[tauri::command]
async fn save_account(
    state: State<'_, AppState>,
    request: SaveAccountCommandRequest,
) -> Result<AccountDto, String> {
    let mut request = match request.into_library_request() {
        Ok(request) => request,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("account.save", "Failed to validate account")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    if let Some(account_id) = request.id.as_deref() {
        let accounts = match state.library.accounts().await {
            Ok(accounts) => accounts,
            Err(error) => {
                let message = command_error(error);
                record_audit(
                    &state.audit,
                    AuditEvent::failed("account.save", "Failed to load existing account")
                        .with_error(Some("library"), message.clone())
                        .with_details(json!({ "accountId": account_id })),
                )
                .await;
                return Err(message);
            }
        };

        if let Some(account) = accounts
            .into_iter()
            .find(|account| account.id == account_id)
        {
            request.enabled = account.enabled;
        }
    }
    let details = json!({
        "accountId": request.id.clone(),
        "hasLoginName": request.login_name.is_some(),
        "hasPassword": request.password.is_some(),
        "enabled": request.enabled,
    });
    let result = state.library.save_account(request).await;

    match result {
        Ok(account) => {
            let has_credential = state
                .library
                .account_has_saved_password(&account)
                .map_err(command_error)?;

            record_audit(
                &state.audit,
                AuditEvent::succeeded("account.save", "Saved account").with_details(json!({
                    "accountId": account.id.clone(),
                    "label": account.label.clone(),
                    "hasCredential": has_credential,
                    "enabled": account.enabled,
                })),
            )
            .await;
            Ok(AccountDto::from_account(account, has_credential))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("account.save", "Failed to save account")
                    .with_error(Some("library"), message.clone())
                    .with_details(details),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn set_account_enabled(
    state: State<'_, AppState>,
    request: SetAccountEnabledRequest,
) -> Result<(), String> {
    let account_id = match normalize_required_id(request.account_id) {
        Ok(account_id) => account_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("account.setEnabled", "Failed to validate account toggle")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let result = state
        .library
        .set_account_enabled(&account_id, request.enabled)
        .await;

    match result {
        Ok(()) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("account.setEnabled", "Updated account enabled state")
                    .with_details(json!({
                        "accountId": account_id,
                        "enabled": request.enabled,
                    })),
            )
            .await;
            Ok(())
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "account.setEnabled",
                    "Failed to update account enabled state",
                )
                .with_error(Some("library"), message.clone())
                .with_details(json!({
                    "accountId": account_id,
                    "enabled": request.enabled,
                })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn remove_account(
    state: State<'_, AppState>,
    request: RemoveAccountRequest,
) -> Result<AccountRemovalReportDto, String> {
    let account_id = match normalize_required_id(request.account_id) {
        Ok(account_id) => account_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("account.remove", "Failed to validate account removal")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };

    match state.library.remove_account(&account_id).await {
        Ok(report) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("account.remove", "Removed account").with_details(json!({
                    "accountId": report.account_id.clone(),
                    "label": report.label.clone(),
                    "credentialDeleted": report.credential_deleted,
                })),
            )
            .await;
            Ok(AccountRemovalReportDto::from(report))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("account.remove", "Failed to remove account")
                    .with_error(Some("library"), message.clone())
                    .with_details(json!({ "accountId": account_id })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn list_products(
    state: State<'_, AppState>,
    request: ListProductsRequest,
) -> Result<ProductListPageDto, String> {
    state
        .library
        .list_products(&request.into_query()?)
        .await
        .map(ProductListPageDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn list_product_filter_facets(
    state: State<'_, AppState>,
    request: ListProductsRequest,
) -> Result<ProductFilterFacetsDto, String> {
    state
        .library
        .product_filter_facets(&request.into_query()?)
        .await
        .map(ProductFilterFacetsDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn get_product_detail(
    state: State<'_, AppState>,
    request: GetProductDetailRequest,
) -> Result<ProductDetailDto, String> {
    let work_id = normalize_required_id(request.work_id)?;

    state
        .library
        .product_detail(&work_id)
        .await
        .map(ProductDetailDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn set_product_custom_tags(
    state: State<'_, AppState>,
    request: SetProductCustomTagsRequest,
) -> Result<Vec<ProductCustomTagDto>, String> {
    let work_id = match normalize_required_id(request.work_id) {
        Ok(work_id) => work_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("product.tags.update", "Failed to validate custom tags")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let tags = match normalize_optional_strings(Some(request.tags)) {
        Ok(tags) => tags,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("product.tags.update", "Failed to validate custom tags")
                    .with_error(Some("validation"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let result = state.library.set_product_custom_tags(&work_id, &tags).await;

    match result {
        Ok(tags) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("product.tags.update", "Updated product custom tags")
                    .with_details(json!({
                        "workId": work_id,
                        "tagCount": tags.len(),
                    })),
            )
            .await;
            Ok(tags.into_iter().map(ProductCustomTagDto::from).collect())
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "product.tags.update",
                    "Failed to update product custom tags",
                )
                .with_error(Some("library"), message.clone())
                .with_details(json!({
                    "workId": work_id,
                    "tagCount": tags.len(),
                })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn start_account_sync(
    state: State<'_, AppState>,
    request: StartAccountSyncRequest,
) -> Result<StartJobResponse, String> {
    let account_id = match normalize_required_id(request.account_id) {
        Ok(account_id) => account_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("account.sync.queue", "Failed to validate account sync")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let password = match normalize_secret(request.password) {
        Ok(password) => password,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "account.sync.queue",
                    "Failed to validate account sync secret",
                )
                .with_error(Some("validation"), error.clone())
                .with_details(json!({ "accountId": account_id })),
            )
            .await;
            return Err(error);
        }
    };
    let library = state.library.clone();
    let mut metadata = JobMetadata::new();

    metadata.insert("accountId".to_owned(), json!(account_id.clone()));

    let job_account_id = account_id.clone();
    let job_id = state.jobs.spawn(
        "accountSync",
        format!("Sync {job_account_id}"),
        metadata,
        move |context| async move {
            context.info("Preparing account sync");
            let client = dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default())
                .map_err(|error| JobFailure::with_code("api_client", error.to_string()))?;
            let source = DlsiteSyncSource::new(client);
            let progress_sink = JobSyncProgressSink {
                context: context.clone(),
            };
            let report = library
                .sync_account_with_source(
                    AccountSyncRequest {
                        account_id: &job_account_id,
                        password: password.as_deref(),
                        cancellation_token: Some(context.cancellation_token()),
                        progress_sink: Some(&progress_sink),
                    },
                    &source,
                )
                .await
                .map_err(account_sync_failure)?;
            let mut output = JobMetadata::new();

            output.insert("accountId".to_owned(), json!(report.account_id));
            output.insert("syncRunId".to_owned(), json!(report.sync_run_id));
            output.insert("purchasedCount".to_owned(), json!(report.purchased_count));
            output.insert(
                "cachedWorkCount".to_owned(),
                json!(report.cached_work_count),
            );
            output.insert(
                "missingDetailCount".to_owned(),
                json!(report.missing_detail_count),
            );
            output.insert("pageLimit".to_owned(), json!(report.page_limit));
            output.insert("concurrency".to_owned(), json!(report.concurrency));
            if report.missing_detail_count > 0 {
                context.warn(format!(
                    "{} purchased works were missing details from content/works",
                    report.missing_detail_count
                ));
            }
            context.info(format!("Synced {} works", report.cached_work_count));

            Ok(output)
        },
    );

    record_audit(
        &state.audit,
        AuditEvent::queued("account.sync", "Queued account sync").with_details(json!({
            "accountId": account_id,
            "jobId": job_id.to_string(),
        })),
    )
    .await;

    Ok(StartJobResponse {
        job_id: job_id.to_string(),
    })
}

#[tauri::command]
async fn start_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartWorkDownloadRequest,
) -> Result<StartJobResponse, String> {
    let work_id = match normalize_required_id(request.work_id) {
        Ok(work_id) => work_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.queue", "Failed to validate download")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let account_id = match normalize_optional_id(request.account_id) {
        Ok(account_id) => account_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.queue", "Failed to validate download account")
                    .with_error(Some("validation"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let password = match normalize_secret(request.password) {
        Ok(password) => password,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.queue", "Failed to validate download secret")
                    .with_error(Some("validation"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.queue",
                    "Failed to load settings for download",
                )
                .with_error(Some("storage"), message.clone())
                .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(message);
        }
    };
    let library_root = match required_library_root(&settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.queue", "Failed to resolve library folder")
                    .with_error(Some("settings"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let download_root = match effective_download_root(&app, &settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.queue",
                    "Failed to resolve download staging folder",
                )
                .with_error(Some("settings"), error.clone())
                .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let unpack_policy = request.unpack_policy.unwrap_or_default().into();
    let replace_existing = request.replace_existing.unwrap_or(false);
    let library = state.library.clone();
    let mut metadata = JobMetadata::new();

    metadata.insert("workId".to_owned(), json!(work_id.clone()));
    if let Some(account_id) = &account_id {
        metadata.insert("accountId".to_owned(), json!(account_id));
    }

    let reservation_id = state.download_reservations.next_owner_id();
    let reservation = state
        .download_reservations
        .claim_available(std::slice::from_ref(&work_id), &reservation_id);

    metadata.insert(
        "skippedQueuedCount".to_owned(),
        json!(reservation.reserved.len()),
    );

    if reservation.available.is_empty() {
        metadata.insert("skippedQueued".to_owned(), json!(true));

        let job_work_id = work_id.clone();
        let job_id = state.jobs.spawn(
            "workDownload",
            format!("Download {job_work_id}"),
            metadata,
            move |context| async move {
                context.info("Download is already queued or running; skipping duplicate request");
                let mut output = JobMetadata::new();

                output.insert("workId".to_owned(), json!(job_work_id));
                output.insert("skippedQueued".to_owned(), json!(true));
                output.insert("skippedQueuedCount".to_owned(), json!(1usize));

                Ok(output)
            },
        );

        record_audit(
            &state.audit,
            AuditEvent::queued("work.download", "Queued duplicate work download no-op")
                .with_details(json!({
                    "workId": work_id,
                    "accountId": account_id,
                    "jobId": job_id.to_string(),
                    "skippedQueuedCount": 1,
                    "replaceExisting": replace_existing,
                    "unpackPolicy": unpack_policy_label(unpack_policy),
                })),
        )
        .await;

        return Ok(StartJobResponse {
            job_id: job_id.to_string(),
        });
    }

    metadata.insert(
        DOWNLOAD_RESERVATION_METADATA_KEY.to_owned(),
        json!(reservation_id),
    );
    metadata.insert("reservedWorkIds".to_owned(), json!(reservation.available));

    let job_work_id = work_id.clone();
    let audit_account_id = account_id.clone();
    let job_id = state.jobs.spawn(
        "workDownload",
        format!("Download {job_work_id}"),
        metadata,
        move |context| async move {
            context.info("Preparing download");
            let client = dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default())
                .map_err(|error| JobFailure::with_code("api_client", error.to_string()))?;
            let source = DlsiteWorkDownloadSource::new(client);
            let progress_sink = JobWorkDownloadProgressSink::new(context.clone());
            let report = library
                .download_work_with_source(
                    WorkDownloadRequest {
                        work_id: &job_work_id,
                        account_id: account_id.as_deref(),
                        password: password.as_deref(),
                        library_root: &library_root,
                        download_root: &download_root,
                        unpack_policy,
                        replace_existing,
                        cancellation_token: Some(context.cancellation_token()),
                        progress_sink: Some(&progress_sink),
                    },
                    &source,
                )
                .await
                .map_err(work_download_failure)?;
            let mut output = JobMetadata::new();

            output.insert("workId".to_owned(), json!(report.work_id));
            output.insert("accountId".to_owned(), json!(report.account_id));
            output.insert(
                "localPath".to_owned(),
                json!(report.local_path.to_string_lossy().to_string()),
            );
            output.insert("fileCount".to_owned(), json!(report.file_count));
            output.insert(
                "archiveExtracted".to_owned(),
                json!(report.archive_extracted),
            );
            context.info(format!("Downloaded {}", job_work_id));

            Ok(output)
        },
    );

    record_audit(
        &state.audit,
        AuditEvent::queued("work.download", "Queued work download").with_details(json!({
            "workId": work_id,
            "accountId": audit_account_id,
            "jobId": job_id.to_string(),
            "replaceExisting": replace_existing,
            "unpackPolicy": unpack_policy_label(unpack_policy),
        })),
    )
    .await;

    Ok(StartJobResponse {
        job_id: job_id.to_string(),
    })
}

#[tauri::command]
async fn start_bulk_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: BulkWorkDownloadCommandRequest,
) -> Result<StartJobResponse, String> {
    let query = match request.into_query() {
        Ok(query) => query,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.queue",
                    "Failed to validate bulk download",
                )
                .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.queue",
                    "Failed to load settings for bulk download",
                )
                .with_error(Some("storage"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };
    let library_root = match required_library_root(&settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.queue",
                    "Failed to resolve library folder",
                )
                .with_error(Some("settings"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let download_root = match effective_download_root(&app, &settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.queue",
                    "Failed to resolve download staging folder",
                )
                .with_error(Some("settings"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let unpack_policy = request.unpack_policy.unwrap_or_default().into();
    let skip_downloaded = request.skip_downloaded.unwrap_or(true);
    let candidates = match bulk_download_candidates(&state.library, &query, skip_downloaded).await {
        Ok(candidates) => candidates,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.queue",
                    "Failed to select bulk download products",
                )
                .with_error(Some("library"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };
    let reservation_id = state.download_reservations.next_owner_id();
    let reservation = state
        .download_reservations
        .claim_available(&candidates.work_ids, &reservation_id);
    let skipped_queued_count = reservation.reserved.len();
    let reserved_work_ids = reservation.available.clone();
    let library = state.library.clone();
    let mut metadata = JobMetadata::new();

    metadata.insert("search".to_owned(), json!(query.search.clone()));
    metadata.insert("accountId".to_owned(), json!(query.account_id.clone()));
    metadata.insert("skipDownloaded".to_owned(), json!(skip_downloaded));
    metadata.insert(
        "candidateCount".to_owned(),
        json!(candidates.work_ids.len()),
    );
    metadata.insert(
        "skippedDownloadedCount".to_owned(),
        json!(candidates.skipped_downloaded_count),
    );
    metadata.insert("skippedQueuedCount".to_owned(), json!(skipped_queued_count));
    metadata.insert("reservedCount".to_owned(), json!(reserved_work_ids.len()));
    metadata.insert(
        "unpackPolicy".to_owned(),
        json!(unpack_policy_label(unpack_policy)),
    );

    if !reserved_work_ids.is_empty() {
        metadata.insert(
            DOWNLOAD_RESERVATION_METADATA_KEY.to_owned(),
            json!(reservation_id),
        );
        metadata.insert(
            "reservedWorkIds".to_owned(),
            json!(reserved_work_ids.clone()),
        );
    }

    if reserved_work_ids.is_empty() {
        let total_count = candidates.total_count;
        let skipped_downloaded_count = candidates.skipped_downloaded_count;
        let audit_metadata = metadata.clone();
        let job_id = state.jobs.spawn(
            "bulkWorkDownload",
            "Download Library results",
            metadata,
            move |context| async move {
                context.info("No matching products are available for download");
                let output = bulk_download_noop_output(
                    total_count,
                    skipped_downloaded_count,
                    skipped_queued_count,
                );

                Ok(output)
            },
        );

        record_audit(
            &state.audit,
            AuditEvent::queued("work.bulkDownload", "Queued bulk work download no-op")
                .with_details(json!({
                    "jobId": job_id.to_string(),
                    "metadata": audit_metadata,
                })),
        )
        .await;

        return Ok(StartJobResponse {
            job_id: job_id.to_string(),
        });
    }

    let audit_metadata = metadata.clone();
    let job_id = state.jobs.spawn(
        "bulkWorkDownload",
        "Download Library results",
        metadata,
        move |context| async move {
            context.info("Preparing bulk download");
            if skipped_queued_count > 0 {
                context.info(format!(
                    "Skipping {skipped_queued_count} works already queued or downloading"
                ));
            }
            let client = dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default())
                .map_err(|error| JobFailure::with_code("api_client", error.to_string()))?;
            let source = DlsiteWorkDownloadSource::new(client);
            let progress_sink = JobBulkWorkDownloadProgressSink {
                context: context.clone(),
            };
            let report = library
                .download_products_with_source(
                    BulkWorkDownloadRequest {
                        query,
                        work_ids: Some(reserved_work_ids),
                        library_root: &library_root,
                        download_root: &download_root,
                        unpack_policy,
                        skip_downloaded,
                        cancellation_token: Some(context.cancellation_token()),
                        progress_sink: Some(&progress_sink),
                    },
                    &source,
                )
                .await
                .map_err(work_download_failure)?;
            let output = bulk_download_output(&report, skipped_queued_count);

            context.info(format!(
                "Bulk download finished: {} downloaded, {} failed, {} downloaded skips, {} queued skips",
                report.succeeded_count,
                report.failed_count,
                report.skipped_downloaded_count,
                skipped_queued_count
            ));

            if report.failed_count > 0 {
                return Err(JobFailure::with_code(
                    "partial_failure",
                    format!(
                        "Downloaded {} works, failed {} works",
                        report.succeeded_count, report.failed_count
                    ),
                )
                .with_detail("bulkDownload", json!(output.clone())));
            }

            Ok(output)
        },
    );

    record_audit(
        &state.audit,
        AuditEvent::queued("work.bulkDownload", "Queued bulk work download").with_details(json!({
            "jobId": job_id.to_string(),
            "metadata": audit_metadata,
        })),
    )
    .await;

    Ok(StartJobResponse {
        job_id: job_id.to_string(),
    })
}

#[tauri::command]
async fn preview_bulk_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: BulkWorkDownloadCommandRequest,
) -> Result<BulkWorkDownloadPreviewDto, String> {
    let query = match request.into_query() {
        Ok(query) => query,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.preview",
                    "Failed to validate bulk download preview",
                )
                .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.preview",
                    "Failed to load settings for bulk download preview",
                )
                .with_error(Some("storage"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };

    if let Err(error) = required_library_root(&settings) {
        record_audit(
            &state.audit,
            AuditEvent::failed(
                "work.bulkDownload.preview",
                "Failed to resolve library folder",
            )
            .with_error(Some("settings"), error.clone()),
        )
        .await;
        return Err(error);
    }

    if let Err(error) = effective_download_root(&app, &settings) {
        record_audit(
            &state.audit,
            AuditEvent::failed(
                "work.bulkDownload.preview",
                "Failed to resolve download staging folder",
            )
            .with_error(Some("settings"), error.clone()),
        )
        .await;
        return Err(error);
    }

    let skip_downloaded = request.skip_downloaded.unwrap_or(true);
    let candidates = match bulk_download_candidates(&state.library, &query, skip_downloaded).await {
        Ok(candidates) => candidates,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.bulkDownload.preview",
                    "Failed to select bulk download products",
                )
                .with_error(Some("library"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };
    let reservation = state
        .download_reservations
        .split_available(&candidates.work_ids);
    let skipped_queued_count = reservation.reserved.len();
    let available_work_ids = reservation.available;
    let library = state.library.clone();
    let (result_tx, result_rx) =
        tokio::sync::oneshot::channel::<Result<BulkWorkDownloadPreviewDto, String>>();
    let mut metadata = JobMetadata::new();

    metadata.insert("search".to_owned(), json!(query.search.clone()));
    metadata.insert("accountId".to_owned(), json!(query.account_id.clone()));
    metadata.insert("skipDownloaded".to_owned(), json!(skip_downloaded));
    metadata.insert(
        "candidateCount".to_owned(),
        json!(candidates.work_ids.len()),
    );
    metadata.insert(
        "skippedDownloadedCount".to_owned(),
        json!(candidates.skipped_downloaded_count),
    );
    metadata.insert("skippedQueuedCount".to_owned(), json!(skipped_queued_count));
    metadata.insert(
        "plannedCandidateCount".to_owned(),
        json!(available_work_ids.len()),
    );

    let audit_metadata = metadata.clone();
    let job_id = state.jobs.spawn(
        "bulkWorkDownloadPreview",
        "Plan Bulk Download",
        metadata,
        move |context| async move {
            context.info("Preparing bulk download plan");
            let client = match dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default()) {
                Ok(client) => client,
                Err(error) => {
                    let message = error.to_string();
                    let _ = result_tx.send(Err(message.clone()));
                    return Err(JobFailure::with_code("api_client", message));
                }
            };
            let source = DlsiteWorkDownloadSource::new(client);
            let progress_sink = JobBulkWorkDownloadPreviewProgressSink {
                context: context.clone(),
            };
            let result = library
                .preview_download_products_with_source(
                    BulkWorkDownloadPreviewRequest {
                        query,
                        work_ids: Some(available_work_ids),
                        skip_downloaded,
                        cancellation_token: Some(context.cancellation_token()),
                        progress_sink: Some(&progress_sink),
                    },
                    &source,
                )
                .await;

            match result {
                Ok(preview) => {
                    let dto =
                        BulkWorkDownloadPreviewDto::from_preview(preview, skipped_queued_count);
                    let output = bulk_download_preview_output(&dto);

                    let _ = result_tx.send(Ok(dto));

                    Ok(output)
                }
                Err(error) => {
                    let failure = work_download_failure(error);
                    let message = failure.message.clone();

                    let _ = result_tx.send(Err(message));

                    Err(failure)
                }
            }
        },
    );

    record_audit(
        &state.audit,
        AuditEvent::queued("work.bulkDownload.preview", "Queued bulk download preview")
            .with_details(json!({
                "jobId": job_id.to_string(),
                "metadata": audit_metadata,
            })),
    )
    .await;

    result_rx
        .await
        .map_err(|_| "bulk download preview job stopped before returning a result".to_owned())?
}

#[tauri::command]
async fn open_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: OpenWorkDownloadRequest,
) -> Result<(), String> {
    let work_id = match normalize_required_id(request.work_id) {
        Ok(work_id) => work_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.open", "Failed to validate open request")
                    .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let download = match state.storage.work_download_state(&work_id).await {
        Ok(download) => download,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.open", "Failed to load download state")
                    .with_error(Some("storage"), message.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(message);
        }
    };

    if download.status != WorkDownloadStatus::Downloaded {
        let message = format!("{work_id} is not downloaded");
        record_audit(
            &state.audit,
            AuditEvent::failed("work.open", "Failed to open downloaded work")
                .with_error(Some("not_downloaded"), message.clone())
                .with_details(json!({ "workId": work_id })),
        )
        .await;
        return Err(message);
    }

    let local_path = download
        .local_path
        .as_deref()
        .ok_or_else(|| format!("{work_id} does not have a local path"))?;
    let canonical_path = match canonicalize_existing_path(local_path) {
        Ok(path) => path,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.open", "Failed to resolve downloaded work path")
                    .with_error(Some("path"), error.clone())
                    .with_details(json!({ "workId": work_id, "path": local_path })),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.open", "Failed to load settings")
                    .with_error(Some("storage"), message.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(message);
        }
    };
    let allowed_roots = canonical_open_roots(&app, &settings);

    if !path_is_under_any_root(&canonical_path, &allowed_roots) {
        let message = format!(
            "download path is outside the configured library or staging folders: {}",
            canonical_path.display()
        );
        record_audit(
            &state.audit,
            AuditEvent::failed("work.open", "Refused to open path outside configured roots")
                .with_error(Some("path_outside_roots"), message.clone())
                .with_details(json!({
                    "workId": work_id,
                    "path": canonical_path.to_string_lossy().to_string(),
                })),
        )
        .await;
        return Err(message);
    }

    match app
        .opener()
        .open_path(
            canonical_path.to_string_lossy().into_owned(),
            None::<String>,
        )
        .map_err(command_error)
    {
        Ok(()) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("work.open", "Opened downloaded work").with_details(json!({
                    "workId": work_id,
                    "path": canonical_path.to_string_lossy().to_string(),
                })),
            )
            .await;
            Ok(())
        }
        Err(message) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.open", "Failed to open downloaded work")
                    .with_error(Some("opener"), message.clone())
                    .with_details(json!({
                        "workId": work_id,
                        "path": canonical_path.to_string_lossy().to_string(),
                    })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn delete_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DeleteWorkDownloadRequest,
) -> Result<WorkDownloadStateDto, String> {
    let work_id = match normalize_required_id(request.work_id) {
        Ok(work_id) => work_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.delete",
                    "Failed to validate delete download request",
                )
                .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.delete", "Failed to load settings")
                    .with_error(Some("storage"), message.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(message);
        }
    };
    let library_root = match required_library_root(&settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.delete", "Failed to resolve library folder")
                    .with_error(Some("settings"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let download_root = match effective_download_root(&app, &settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.delete",
                    "Failed to resolve download staging folder",
                )
                .with_error(Some("settings"), error.clone())
                .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };

    let result = state
        .library
        .remove_work_download(WorkDownloadRemovalRequest::new(
            &work_id,
            &library_root,
            &download_root,
        ))
        .await;

    match result {
        Ok(state_after_delete) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("work.download.delete", "Deleted work download")
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            Ok(WorkDownloadStateDto::from(state_after_delete))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.delete", "Failed to delete work download")
                    .with_error(Some("library"), message.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn mark_work_downloaded(
    state: State<'_, AppState>,
    request: MarkWorkDownloadedRequest,
) -> Result<WorkDownloadStateDto, String> {
    let work_id = match normalize_required_id(request.work_id) {
        Ok(work_id) => work_id,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.mark",
                    "Failed to validate manual download request",
                )
                .with_error(Some("validation"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let local_path = match normalize_required_path(request.local_path) {
        Ok(path) => path,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.download.mark",
                    "Failed to validate manual download path",
                )
                .with_error(Some("validation"), error.clone())
                .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.mark", "Failed to load settings")
                    .with_error(Some("storage"), message.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(message);
        }
    };
    let library_root = match required_library_root(&settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.mark", "Failed to resolve library folder")
                    .with_error(Some("settings"), error.clone())
                    .with_details(json!({ "workId": work_id })),
            )
            .await;
            return Err(error);
        }
    };

    let result = state
        .library
        .mark_work_downloaded(WorkDownloadMarkRequest::new(
            &work_id,
            &library_root,
            &local_path,
        ))
        .await;

    match result {
        Ok(state_after_mark) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("work.download.mark", "Marked work as downloaded")
                    .with_details(json!({
                        "workId": work_id,
                        "path": state_after_mark.local_path,
                    })),
            )
            .await;
            Ok(WorkDownloadStateDto::from(state_after_mark))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.download.mark", "Failed to mark work as downloaded")
                    .with_error(Some("library"), message.clone())
                    .with_details(json!({
                        "workId": work_id,
                        "path": local_path.to_string_lossy().to_string(),
                    })),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn scan_local_work_downloads(
    state: State<'_, AppState>,
) -> Result<LocalWorkImportReportDto, String> {
    let settings = match state.storage.app_settings().await {
        Ok(settings) => settings,
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.local.scan", "Failed to load settings")
                    .with_error(Some("storage"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };
    let library_root = match required_library_root(&settings) {
        Ok(root) => root,
        Err(error) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("work.local.scan", "Failed to resolve library folder")
                    .with_error(Some("settings"), error.clone()),
            )
            .await;
            return Err(error);
        }
    };
    let metadata_source = match dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default()) {
        Ok(client) => DlsitePublicMetadataSource::new(client),
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed(
                    "work.local.scan",
                    "Failed to prepare DLsite product metadata lookup",
                )
                .with_error(Some("api_client"), message.clone()),
            )
            .await;
            return Err(message);
        }
    };

    match state
        .library
        .import_local_work_downloads_with_metadata_source(
            LocalWorkImportRequest::new(&library_root),
            &metadata_source,
        )
        .await
    {
        Ok(report) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("work.local.scan", "Scanned local work folders")
                    .with_details(json!({
                        "scannedDirectories": report.scanned_directories,
                        "importedCount": report.imported_count,
                        "skippedNoId": report.skipped_no_id,
                        "skippedAmbiguous": report.skipped_ambiguous,
                        "skippedNonUtf8": report.skipped_non_utf8,
                        "skippedExisting": report.skipped_existing,
                        "metadataCandidateCount": report.metadata_candidate_count,
                        "metadataUpdatedCount": report.metadata_updated_count,
                        "metadataMissingCount": report.metadata_missing_count,
                        "metadataError": report.metadata_error.as_deref(),
                    })),
            )
            .await;
            Ok(LocalWorkImportReportDto::from(report))
        }
        Err(error) => {
            let message = command_error(error);
            record_audit(
                &state.audit,
                AuditEvent::failed("work.local.scan", "Failed to scan local work folders")
                    .with_error(Some("library"), message.clone()),
            )
            .await;
            Err(message)
        }
    }
}

#[tauri::command]
async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<dm_jobs::JobSnapshot>, String> {
    Ok(state.jobs.list_jobs())
}

#[tauri::command]
async fn get_job(
    state: State<'_, AppState>,
    request: JobIdRequest,
) -> Result<dm_jobs::JobSnapshot, String> {
    let job_id = normalize_required_id(request.job_id)?;

    state
        .jobs
        .get_job(&JobId::from(job_id))
        .ok_or_else(|| "job not found".to_owned())
}

#[tauri::command]
async fn cancel_job(
    state: State<'_, AppState>,
    request: JobIdRequest,
) -> Result<dm_jobs::CancelJobResult, String> {
    let job_id = normalize_required_id(request.job_id)?;

    state
        .jobs
        .cancel_job(&JobId::from(job_id))
        .map_err(command_error)
}

#[tauri::command]
async fn get_job_logs(
    state: State<'_, AppState>,
    request: JobLogsRequest,
) -> Result<JobLogPage, String> {
    let job_id = normalize_required_id(request.job_id)?;

    state
        .jobs
        .job_logs(&JobId::from(job_id), request.after_sequence, request.limit)
        .map_err(command_error)
}

#[tauri::command]
async fn clear_finished_jobs(
    state: State<'_, AppState>,
) -> Result<ClearFinishedJobsResponse, String> {
    Ok(ClearFinishedJobsResponse {
        removed_count: state.jobs.clear_finished(),
    })
}

#[tauri::command]
async fn list_audit_events(
    state: State<'_, AppState>,
    request: ListAuditEventsRequest,
) -> Result<Vec<AuditEvent>, String> {
    state
        .audit
        .recent_events(request.limit.unwrap_or(100))
        .await
        .map_err(command_error)
}

#[tauri::command]
async fn get_audit_log_dir(state: State<'_, AppState>) -> Result<AuditLogDirDto, String> {
    Ok(AuditLogDirDto {
        path: state.audit.log_dir().to_string_lossy().into_owned(),
    })
}

#[tauri::command]
async fn open_audit_log_dir(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let path = state.audit.log_dir().to_path_buf();

    match app
        .opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(command_error)
    {
        Ok(()) => {
            record_audit(
                &state.audit,
                AuditEvent::succeeded("audit.openLogDir", "Opened audit log directory")
                    .with_details(json!({
                        "path": path.to_string_lossy().to_string(),
                    })),
            )
            .await;
            Ok(())
        }
        Err(message) => {
            record_audit(
                &state.audit,
                AuditEvent::failed("audit.openLogDir", "Failed to open audit log directory")
                    .with_error(Some("opener"), message.clone())
                    .with_details(json!({
                        "path": path.to_string_lossy().to_string(),
                    })),
            )
            .await;
            Err(message)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsDto {
    library_root: Option<String>,
    download_root: Option<String>,
}

impl From<AppSettings> for AppSettingsDto {
    fn from(settings: AppSettings) -> Self {
        Self {
            library_root: settings.library_root,
            download_root: settings.download_root,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    library_root: Option<String>,
    download_root: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountDto {
    id: String,
    label: String,
    login_name: Option<String>,
    has_credential: bool,
    enabled: bool,
    created_at: String,
    updated_at: String,
    last_login_at: Option<String>,
    last_sync_at: Option<String>,
}

impl AccountDto {
    fn from_account(account: Account, has_credential: bool) -> Self {
        Self {
            id: account.id,
            label: account.label,
            login_name: account.login_name,
            has_credential,
            enabled: account.enabled,
            created_at: account.created_at,
            updated_at: account.updated_at,
            last_login_at: account.last_login_at,
            last_sync_at: account.last_sync_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveAccountCommandRequest {
    id: Option<String>,
    label: String,
    login_name: Option<String>,
    password: Option<String>,
}

impl SaveAccountCommandRequest {
    fn into_library_request(self) -> Result<SaveAccountRequest, String> {
        Ok(SaveAccountRequest {
            id: normalize_optional_id(self.id)?,
            label: normalize_label(self.label)?,
            login_name: normalize_optional_string(self.login_name)?,
            password: normalize_secret(self.password)?,
            remember_password: true,
            enabled: true,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetAccountEnabledRequest {
    account_id: String,
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveAccountRequest {
    account_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountRemovalReportDto {
    account_id: String,
    label: String,
    credential_deleted: bool,
}

impl From<AccountRemovalReport> for AccountRemovalReportDto {
    fn from(report: AccountRemovalReport) -> Self {
        Self {
            account_id: report.account_id,
            label: report.label,
            credential_deleted: report.credential_deleted,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListProductsRequest {
    search: Option<String>,
    account_id: Option<String>,
    account_ids: Option<Vec<String>>,
    type_group: Option<ProductTypeGroupDto>,
    type_groups: Option<Vec<ProductTypeGroupDto>>,
    age_category: Option<ProductAgeCategoryDto>,
    age_categories: Option<Vec<ProductAgeCategoryDto>>,
    source_groups: Option<Vec<ProductSourceGroupDto>>,
    maker_names: Option<Vec<String>>,
    custom_tag_names: Option<Vec<String>>,
    excluded_custom_tag_names: Option<Vec<String>>,
    sort: Option<ProductSortDto>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl ListProductsRequest {
    fn into_query(self) -> Result<ProductListQuery, String> {
        Ok(ProductListQuery {
            search: normalize_optional_string(self.search)?,
            account_id: normalize_optional_id(self.account_id)?,
            account_ids: normalize_optional_ids(self.account_ids)?,
            type_group: self.type_group.map(Into::into),
            type_groups: self
                .type_groups
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            age_category: self.age_category.map(Into::into),
            age_categories: self
                .age_categories
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            source_groups: self
                .source_groups
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            maker_names: normalize_optional_strings(self.maker_names)?,
            custom_tag_names: normalize_optional_strings(self.custom_tag_names)?,
            excluded_custom_tag_names: normalize_optional_strings(self.excluded_custom_tag_names)?,
            sort: self.sort.unwrap_or_default().into(),
            limit: self.limit.unwrap_or(100).clamp(1, 500),
            offset: self.offset.unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetProductDetailRequest {
    work_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetProductCustomTagsRequest {
    work_id: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProductSortDto {
    TitleAsc,
    #[default]
    LatestPurchaseDesc,
    PublishedAtDesc,
}

impl From<ProductSortDto> for ProductSort {
    fn from(sort: ProductSortDto) -> Self {
        match sort {
            ProductSortDto::TitleAsc => Self::TitleAsc,
            ProductSortDto::LatestPurchaseDesc => Self::LatestPurchaseDesc,
            ProductSortDto::PublishedAtDesc => Self::PublishedAtDesc,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProductAgeCategoryDto {
    All,
    R15,
    R18,
}

impl From<ProductAgeCategoryDto> for ProductAgeCategory {
    fn from(age_category: ProductAgeCategoryDto) -> Self {
        match age_category {
            ProductAgeCategoryDto::All => Self::All,
            ProductAgeCategoryDto::R15 => Self::R15,
            ProductAgeCategoryDto::R18 => Self::R18,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProductTypeGroupDto {
    Audio,
    Video,
    Game,
    Image,
    Other,
}

impl From<ProductTypeGroupDto> for ProductTypeGroup {
    fn from(type_group: ProductTypeGroupDto) -> Self {
        match type_group {
            ProductTypeGroupDto::Audio => Self::Audio,
            ProductTypeGroupDto::Video => Self::Video,
            ProductTypeGroupDto::Game => Self::Game,
            ProductTypeGroupDto::Image => Self::Image,
            ProductTypeGroupDto::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProductSourceGroupDto {
    Owned,
    LocalOnly,
}

impl From<ProductSourceGroupDto> for ProductSourceGroup {
    fn from(source_group: ProductSourceGroupDto) -> Self {
        match source_group {
            ProductSourceGroupDto::Owned => Self::Owned,
            ProductSourceGroupDto::LocalOnly => Self::LocalOnly,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductListPageDto {
    total_count: u64,
    products: Vec<ProductListItemDto>,
}

impl From<ProductListPage> for ProductListPageDto {
    fn from(page: ProductListPage) -> Self {
        Self {
            total_count: page.total_count,
            products: page
                .products
                .into_iter()
                .map(ProductListItemDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BulkWorkDownloadPreviewDto {
    total_count: u64,
    requested_count: usize,
    skipped_downloaded_count: usize,
    skipped_queued_count: usize,
    planned_count: usize,
    failed_count: usize,
    known_expected_bytes: u64,
    total_expected_bytes: Option<u64>,
    unknown_size_count: usize,
}

impl BulkWorkDownloadPreviewDto {
    fn from_preview(preview: BulkWorkDownloadPreview, skipped_queued_count: usize) -> Self {
        Self {
            total_count: preview.total_count,
            requested_count: preview.requested_count,
            skipped_downloaded_count: preview.skipped_downloaded_count,
            skipped_queued_count,
            planned_count: preview.planned_count,
            failed_count: preview.failed_count,
            known_expected_bytes: preview.known_expected_bytes,
            total_expected_bytes: preview.total_expected_bytes,
            unknown_size_count: preview.unknown_size_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductFilterFacetsDto {
    makers: Vec<ProductMakerFacetDto>,
    custom_tags: Vec<ProductCustomTagFacetDto>,
}

impl From<ProductFilterFacets> for ProductFilterFacetsDto {
    fn from(facets: ProductFilterFacets) -> Self {
        Self {
            makers: facets
                .makers
                .into_iter()
                .map(ProductMakerFacetDto::from)
                .collect(),
            custom_tags: facets
                .custom_tags
                .into_iter()
                .map(ProductCustomTagFacetDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductMakerFacetDto {
    name: String,
    count: u64,
}

impl From<ProductMakerFacet> for ProductMakerFacetDto {
    fn from(facet: ProductMakerFacet) -> Self {
        Self {
            name: facet.name,
            count: facet.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductCustomTagFacetDto {
    name: String,
    count: u64,
}

impl From<ProductCustomTagFacet> for ProductCustomTagFacetDto {
    fn from(facet: ProductCustomTagFacet) -> Self {
        Self {
            name: facet.name,
            count: facet.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductListItemDto {
    work_id: String,
    title: String,
    maker_name: Option<String>,
    work_type: Option<String>,
    age_category: Option<String>,
    thumbnail_url: Option<String>,
    published_at: Option<String>,
    updated_at: Option<String>,
    earliest_purchased_at: Option<String>,
    latest_purchased_at: Option<String>,
    credit_groups: Vec<ProductCreditGroupDto>,
    custom_tags: Vec<ProductCustomTagDto>,
    download: WorkDownloadStateDto,
    owners: Vec<ProductOwnerDto>,
}

impl From<ProductListItem> for ProductListItemDto {
    fn from(product: ProductListItem) -> Self {
        Self {
            work_id: product.work_id,
            title: product.title,
            maker_name: product.maker_name,
            work_type: product.work_type,
            age_category: product.age_category,
            thumbnail_url: product.thumbnail_url,
            published_at: product.published_at,
            updated_at: product.updated_at,
            earliest_purchased_at: product.earliest_purchased_at,
            latest_purchased_at: product.latest_purchased_at,
            credit_groups: product
                .credit_groups
                .into_iter()
                .map(ProductCreditGroupDto::from)
                .collect(),
            custom_tags: product
                .custom_tags
                .into_iter()
                .map(ProductCustomTagDto::from)
                .collect(),
            download: WorkDownloadStateDto::from(product.download),
            owners: product
                .owners
                .into_iter()
                .map(ProductOwnerDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductDetailDto {
    work_id: String,
    title: String,
    title_variants: Vec<ProductTextValueDto>,
    maker_id: Option<String>,
    maker_name: Option<String>,
    maker_names: Vec<ProductTextValueDto>,
    work_type: Option<String>,
    age_category: Option<String>,
    thumbnail_url: Option<String>,
    content_size_bytes: Option<u64>,
    registered_at: Option<String>,
    published_at: Option<String>,
    updated_at: Option<String>,
    last_detail_sync_at: String,
    earliest_purchased_at: Option<String>,
    latest_purchased_at: Option<String>,
    credit_groups: Vec<ProductCreditGroupDto>,
    tags: Vec<ProductTagDto>,
    custom_tags: Vec<ProductCustomTagDto>,
    download: WorkDownloadStateDto,
    owners: Vec<ProductOwnerDto>,
}

impl From<ProductDetail> for ProductDetailDto {
    fn from(detail: ProductDetail) -> Self {
        Self {
            work_id: detail.work_id,
            title: detail.title,
            title_variants: detail
                .title_variants
                .into_iter()
                .map(ProductTextValueDto::from)
                .collect(),
            maker_id: detail.maker_id,
            maker_name: detail.maker_name,
            maker_names: detail
                .maker_names
                .into_iter()
                .map(ProductTextValueDto::from)
                .collect(),
            work_type: detail.work_type,
            age_category: detail.age_category,
            thumbnail_url: detail.thumbnail_url,
            content_size_bytes: detail.content_size_bytes,
            registered_at: detail.registered_at,
            published_at: detail.published_at,
            updated_at: detail.updated_at,
            last_detail_sync_at: detail.last_detail_sync_at,
            earliest_purchased_at: detail.earliest_purchased_at,
            latest_purchased_at: detail.latest_purchased_at,
            credit_groups: detail
                .credit_groups
                .into_iter()
                .map(ProductCreditGroupDto::from)
                .collect(),
            tags: detail.tags.into_iter().map(ProductTagDto::from).collect(),
            custom_tags: detail
                .custom_tags
                .into_iter()
                .map(ProductCustomTagDto::from)
                .collect(),
            download: WorkDownloadStateDto::from(detail.download),
            owners: detail
                .owners
                .into_iter()
                .map(ProductOwnerDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductTextValueDto {
    language: String,
    value: String,
}

impl From<ProductTextValue> for ProductTextValueDto {
    fn from(value: ProductTextValue) -> Self {
        Self {
            language: value.language,
            value: value.value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkDownloadStateDto {
    status: WorkDownloadStatusDto,
    local_path: Option<String>,
    staging_path: Option<String>,
    unpack_policy: Option<String>,
    bytes_received: u64,
    bytes_total: Option<u64>,
    error_code: Option<String>,
    error_message: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    updated_at: Option<String>,
}

impl From<WorkDownloadState> for WorkDownloadStateDto {
    fn from(state: WorkDownloadState) -> Self {
        Self {
            status: WorkDownloadStatusDto::from(state.status),
            local_path: state.local_path,
            staging_path: state.staging_path,
            unpack_policy: state.unpack_policy,
            bytes_received: state.bytes_received,
            bytes_total: state.bytes_total,
            error_code: state.error_code,
            error_message: state.error_message,
            started_at: state.started_at,
            completed_at: state.completed_at,
            updated_at: state.updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum WorkDownloadStatusDto {
    NotDownloaded,
    Downloading,
    Downloaded,
    Failed,
    Cancelled,
}

impl From<WorkDownloadStatus> for WorkDownloadStatusDto {
    fn from(status: WorkDownloadStatus) -> Self {
        match status {
            WorkDownloadStatus::NotDownloaded => Self::NotDownloaded,
            WorkDownloadStatus::Downloading => Self::Downloading,
            WorkDownloadStatus::Downloaded => Self::Downloaded,
            WorkDownloadStatus::Failed => Self::Failed,
            WorkDownloadStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalWorkImportReportDto {
    scanned_directories: usize,
    imported_count: usize,
    skipped_no_id: usize,
    skipped_ambiguous: usize,
    skipped_non_utf8: usize,
    skipped_existing: usize,
    metadata_candidate_count: usize,
    metadata_updated_count: usize,
    metadata_missing_count: usize,
    metadata_error: Option<String>,
    imported_works: Vec<LocalWorkImportItemDto>,
}

impl From<LocalWorkImportReport> for LocalWorkImportReportDto {
    fn from(report: LocalWorkImportReport) -> Self {
        Self {
            scanned_directories: report.scanned_directories,
            imported_count: report.imported_count,
            skipped_no_id: report.skipped_no_id,
            skipped_ambiguous: report.skipped_ambiguous,
            skipped_non_utf8: report.skipped_non_utf8,
            skipped_existing: report.skipped_existing,
            metadata_candidate_count: report.metadata_candidate_count,
            metadata_updated_count: report.metadata_updated_count,
            metadata_missing_count: report.metadata_missing_count,
            metadata_error: report.metadata_error,
            imported_works: report
                .imported_works
                .into_iter()
                .map(LocalWorkImportItemDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalWorkImportItemDto {
    work_id: String,
    local_path: String,
}

impl From<dm_library::LocalWorkImportItem> for LocalWorkImportItemDto {
    fn from(item: dm_library::LocalWorkImportItem) -> Self {
        Self {
            work_id: item.work_id,
            local_path: item.local_path.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductCreditGroupDto {
    kind: String,
    label: String,
    names: Vec<String>,
}

impl From<ProductCreditGroup> for ProductCreditGroupDto {
    fn from(group: ProductCreditGroup) -> Self {
        Self {
            kind: group.kind,
            label: group.label,
            names: group.names,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductTagDto {
    class: String,
    name: String,
}

impl From<ProductTag> for ProductTagDto {
    fn from(tag: ProductTag) -> Self {
        Self {
            class: tag.class,
            name: tag.name,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductCustomTagDto {
    name: String,
}

impl From<ProductCustomTag> for ProductCustomTagDto {
    fn from(tag: ProductCustomTag) -> Self {
        Self { name: tag.name }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductOwnerDto {
    account_id: String,
    label: String,
    purchased_at: Option<String>,
}

impl From<ProductOwner> for ProductOwnerDto {
    fn from(owner: ProductOwner) -> Self {
        Self {
            account_id: owner.account_id,
            label: owner.label,
            purchased_at: owner.purchased_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartAccountSyncRequest {
    account_id: String,
    password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartWorkDownloadRequest {
    work_id: String,
    account_id: Option<String>,
    password: Option<String>,
    unpack_policy: Option<UnpackPolicyDto>,
    replace_existing: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkWorkDownloadCommandRequest {
    search: Option<String>,
    account_id: Option<String>,
    account_ids: Option<Vec<String>>,
    type_group: Option<ProductTypeGroupDto>,
    type_groups: Option<Vec<ProductTypeGroupDto>>,
    age_category: Option<ProductAgeCategoryDto>,
    age_categories: Option<Vec<ProductAgeCategoryDto>>,
    source_groups: Option<Vec<ProductSourceGroupDto>>,
    maker_names: Option<Vec<String>>,
    custom_tag_names: Option<Vec<String>>,
    excluded_custom_tag_names: Option<Vec<String>>,
    sort: Option<ProductSortDto>,
    unpack_policy: Option<UnpackPolicyDto>,
    skip_downloaded: Option<bool>,
}

impl BulkWorkDownloadCommandRequest {
    fn into_query(&self) -> Result<ProductListQuery, String> {
        Ok(ProductListQuery {
            search: normalize_optional_string(self.search.clone())?,
            account_id: normalize_optional_id(self.account_id.clone())?,
            account_ids: normalize_optional_ids(self.account_ids.clone())?,
            type_group: self.type_group.map(Into::into),
            type_groups: self
                .type_groups
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            age_category: self.age_category.map(Into::into),
            age_categories: self
                .age_categories
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            source_groups: self
                .source_groups
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            maker_names: normalize_optional_strings(self.maker_names.clone())?,
            custom_tag_names: normalize_optional_strings(self.custom_tag_names.clone())?,
            excluded_custom_tag_names: normalize_optional_strings(
                self.excluded_custom_tag_names.clone(),
            )?,
            sort: self.sort.unwrap_or_default().into(),
            limit: BULK_DOWNLOAD_PAGE_LIMIT,
            offset: 0,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenWorkDownloadRequest {
    work_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteWorkDownloadRequest {
    work_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkWorkDownloadedRequest {
    work_id: String,
    local_path: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum UnpackPolicyDto {
    KeepArchives,
    #[default]
    UnpackWhenRecognized,
}

impl From<UnpackPolicyDto> for dm_download::UnpackPolicy {
    fn from(policy: UnpackPolicyDto) -> Self {
        match policy {
            UnpackPolicyDto::KeepArchives => Self::KeepArchives,
            UnpackPolicyDto::UnpackWhenRecognized => Self::UnpackWhenRecognized,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartJobResponse {
    job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobIdRequest {
    job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobLogsRequest {
    job_id: String,
    after_sequence: Option<u64>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearFinishedJobsResponse {
    removed_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListAuditEventsRequest {
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditLogDirDto {
    path: String,
}

struct JobSyncProgressSink {
    context: JobContext,
}

impl SyncProgressSink for JobSyncProgressSink {
    fn emit(&self, progress: SyncProgress) {
        match progress {
            SyncProgress::LoggingIn => {
                self.context.set_phase("loggingIn");
                self.context.clear_progress();
                self.context.info("Signing in");
            }
            SyncProgress::LoadingCount => {
                self.context.set_phase("loadingCount");
                self.context.clear_progress();
                self.context.info("Checking library count");
            }
            SyncProgress::LoadingPurchases => {
                self.context.set_phase("loadingPurchases");
                self.context.clear_progress();
                self.context.info("Loading purchases");
            }
            SyncProgress::LoadingWorks { work_count } => {
                self.context.set_phase("loadingWorks");
                self.context
                    .set_progress(JobProgress::items(None, Some(work_count as u64)));
                self.context
                    .info(format!("Loading {work_count} work details"));
            }
            SyncProgress::Committing { work_count } => {
                self.context.set_phase("committing");
                self.context.set_progress(JobProgress::items(
                    Some(work_count as u64),
                    Some(work_count as u64),
                ));
                self.context.info("Saving product cache");
            }
            SyncProgress::Completed {
                sync_run_id,
                cached_work_count,
            } => {
                self.context.set_phase("completed");
                self.context.set_progress(JobProgress::items(
                    Some(cached_work_count as u64),
                    Some(cached_work_count as u64),
                ));
                self.context.info(format!(
                    "Sync run {sync_run_id} cached {cached_work_count} works"
                ));
            }
        }
    }
}

struct JobWorkDownloadProgressSink {
    context: JobContext,
    throttle: Mutex<WorkDownloadProgressThrottle>,
}

impl JobWorkDownloadProgressSink {
    fn new(context: JobContext) -> Self {
        Self {
            context,
            throttle: Mutex::new(WorkDownloadProgressThrottle::default()),
        }
    }
}

#[derive(Default)]
struct WorkDownloadProgressThrottle {
    last_download_phase: Option<dm_download::DownloadPhase>,
    last_download_emit_at: Option<Instant>,
}

impl WorkDownloadProgressThrottle {
    fn emit_decision(&mut self, phase: dm_download::DownloadPhase) -> Option<bool> {
        self.emit_decision_at(phase, Instant::now())
    }

    fn emit_decision_at(
        &mut self,
        phase: dm_download::DownloadPhase,
        now: Instant,
    ) -> Option<bool> {
        let phase_changed = self.last_download_phase != Some(phase);

        if phase_changed {
            self.last_download_phase = Some(phase);
            self.last_download_emit_at = Some(now);
            return Some(true);
        }

        if phase != dm_download::DownloadPhase::Downloading {
            self.last_download_emit_at = Some(now);
            return Some(false);
        }

        let interval_elapsed = match self.last_download_emit_at {
            Some(last_emit) => {
                now.duration_since(last_emit) >= WORK_DOWNLOAD_PROGRESS_EVENT_INTERVAL
            }
            None => true,
        };

        if interval_elapsed {
            self.last_download_emit_at = Some(now);
            return Some(false);
        }

        None
    }

    fn reset(&mut self) {
        self.last_download_phase = None;
        self.last_download_emit_at = None;
    }
}

impl WorkDownloadProgressSink for JobWorkDownloadProgressSink {
    fn emit(&self, progress: WorkDownloadProgress) {
        match progress {
            WorkDownloadProgress::LoggingIn => {
                self.context.set_phase("loggingIn");
                self.context.clear_progress();
                self.context.info("Signing in for download");
            }
            WorkDownloadProgress::ResolvingPlan => {
                self.context.set_phase("resolvingDownload");
                self.context.clear_progress();
                self.context.info("Resolving download files");
            }
            WorkDownloadProgress::Download(progress) => {
                if progress.phase == dm_download::DownloadPhase::ProbingMetadata {
                    return;
                }

                let Some(phase_changed) = self
                    .throttle
                    .lock()
                    .expect("download progress throttle lock")
                    .emit_decision(progress.phase)
                else {
                    return;
                };

                if phase_changed {
                    match progress.phase {
                        dm_download::DownloadPhase::ResolvingPlan => {
                            self.context.set_phase("resolvingDownload")
                        }
                        dm_download::DownloadPhase::ProbingMetadata => {}
                        dm_download::DownloadPhase::Downloading => {
                            self.context.set_phase("downloading")
                        }
                        dm_download::DownloadPhase::Finalizing => {
                            self.context.set_phase("finalizing")
                        }
                        dm_download::DownloadPhase::Unpacking => {
                            self.context.set_phase("unpacking")
                        }
                    }
                }

                self.context.set_progress(JobProgress::bytes(
                    Some(progress.bytes_received),
                    progress.bytes_total,
                ));
            }
            WorkDownloadProgress::Finalizing => {
                self.throttle
                    .lock()
                    .expect("download progress throttle lock")
                    .reset();
                self.context.set_phase("finalizing");
                self.context.clear_progress();
                self.context.info("Moving files into the library");
            }
            WorkDownloadProgress::Completed => {
                self.throttle
                    .lock()
                    .expect("download progress throttle lock")
                    .reset();
                self.context.set_phase("completed");
                self.context.clear_progress();
                self.context.info("Download completed");
            }
        }
    }
}

struct JobBulkWorkDownloadProgressSink {
    context: JobContext,
}

struct JobBulkWorkDownloadPreviewProgressSink {
    context: JobContext,
}

impl BulkWorkDownloadPreviewProgressSink for JobBulkWorkDownloadPreviewProgressSink {
    fn emit(&self, progress: BulkWorkDownloadPreviewProgress) {
        match progress {
            BulkWorkDownloadPreviewProgress::Selecting => {
                self.context.set_phase("loadingProducts");
                self.context.clear_progress();
                self.context.info("Selecting matching Library results");
            }
            BulkWorkDownloadPreviewProgress::Selected {
                total_count,
                requested_count,
                skipped_downloaded_count,
            } => {
                self.context.set_phase("bulkPlanning");
                self.context
                    .set_progress(JobProgress::items(Some(0), Some(requested_count as u64)));
                self.context.info(format!(
                    "Planning {requested_count} works from {total_count} matching products; skipped {skipped_downloaded_count} already downloaded"
                ));
            }
            BulkWorkDownloadPreviewProgress::WorkStarted { current, total, .. } => {
                self.context.set_phase("bulkPlanning");
                self.context.set_progress(JobProgress::items(
                    Some(current.saturating_sub(1) as u64),
                    Some(total as u64),
                ));
            }
            BulkWorkDownloadPreviewProgress::WorkPlanned { current, total, .. } => {
                self.context
                    .set_progress(JobProgress::items(Some(current as u64), Some(total as u64)));
            }
            BulkWorkDownloadPreviewProgress::WorkFailed {
                work_id,
                current,
                total,
                error_code,
                error_message,
            } => {
                self.context
                    .set_progress(JobProgress::items(Some(current as u64), Some(total as u64)));
                self.context.warn(format!(
                    "Failed to plan {work_id} ({current}/{total}): {error_code}: {error_message}"
                ));
            }
            BulkWorkDownloadPreviewProgress::Completed {
                planned_count,
                failed_count,
                ..
            } => {
                self.context.set_phase("completed");
                self.context.clear_progress();
                self.context.info(format!(
                    "Bulk download plan completed: {planned_count} planned, {failed_count} failed"
                ));
            }
        }
    }
}

impl BulkWorkDownloadProgressSink for JobBulkWorkDownloadProgressSink {
    fn emit(&self, progress: BulkWorkDownloadProgress) {
        match progress {
            BulkWorkDownloadProgress::Selecting => {
                self.context.set_phase("loadingProducts");
                self.context.clear_progress();
                self.context.info("Selecting matching Library results");
            }
            BulkWorkDownloadProgress::Selected {
                total_count,
                requested_count,
                skipped_downloaded_count,
            } => {
                self.context.set_phase("bulkDownloading");
                self.context
                    .set_progress(JobProgress::items(Some(0), Some(requested_count as u64)));
                self.context.info(format!(
                    "Selected {requested_count} works for download from {total_count} matching products; skipped {skipped_downloaded_count} already downloaded"
                ));
            }
            BulkWorkDownloadProgress::WorkStarted {
                work_id,
                current,
                total,
            } => {
                self.context.set_phase("bulkDownloading");
                self.context.set_progress(JobProgress::items(
                    Some(current.saturating_sub(1) as u64),
                    Some(total as u64),
                ));
                self.context
                    .info(format!("Downloading {work_id} ({current}/{total})"));
            }
            BulkWorkDownloadProgress::WorkCompleted {
                work_id,
                current,
                total,
            } => {
                self.context
                    .set_progress(JobProgress::items(Some(current as u64), Some(total as u64)));
                self.context.info(format!("Downloaded {work_id}"));
            }
            BulkWorkDownloadProgress::WorkFailed {
                work_id,
                current,
                total,
                error_code,
                error_message,
            } => {
                self.context
                    .set_progress(JobProgress::items(Some(current as u64), Some(total as u64)));
                self.context.warn(format!(
                    "Failed to download {work_id} ({current}/{total}): {error_code}: {error_message}"
                ));
            }
            BulkWorkDownloadProgress::Completed { report } => {
                self.context.set_phase("completed");
                self.context.clear_progress();
                self.context.info(format!(
                    "Bulk download completed: {} succeeded, {} failed",
                    report.succeeded_count, report.failed_count
                ));
            }
        }
    }
}

impl SaveSettingsRequest {
    fn into_app_settings(self) -> Result<AppSettings, String> {
        Ok(AppSettings {
            library_root: normalize_path_setting(self.library_root)?,
            download_root: normalize_path_setting(self.download_root)?,
        })
    }
}

fn normalize_path_setting(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Ok(None);
    }

    if value.contains('\0') {
        return Err("path contains a NUL byte".to_owned());
    }

    Ok(Some(value))
}

fn required_library_root(settings: &AppSettings) -> Result<PathBuf, String> {
    settings
        .library_root
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| "Library folder is required".to_owned())
}

fn effective_download_root(app: &AppHandle, settings: &AppSettings) -> Result<PathBuf, String> {
    settings
        .download_root
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| app.path().download_dir().ok())
        .ok_or_else(|| "Download staging folder is required".to_owned())
}

fn canonicalize_existing_path(path: impl AsRef<Path>) -> Result<PathBuf, String> {
    let path = path.as_ref();

    path.canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", path.display()))
}

fn canonical_open_roots(app: &AppHandle, settings: &AppSettings) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(root) = &settings.library_root {
        roots.push(PathBuf::from(root));
    }

    if let Some(root) = &settings.download_root {
        roots.push(PathBuf::from(root));
    } else if let Ok(download_dir) = app.path().download_dir() {
        roots.push(download_dir);
    }

    roots
        .into_iter()
        .filter_map(|root| canonicalize_existing_path(root).ok())
        .collect()
}

fn path_is_under_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

fn normalize_required_id(value: String) -> Result<String, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err("id is required".to_owned());
    }

    if value.contains('\0') {
        return Err("id contains a NUL byte".to_owned());
    }

    Ok(value)
}

fn normalize_required_path(value: String) -> Result<PathBuf, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err("path is required".to_owned());
    }

    if value.contains('\0') {
        return Err("path contains a NUL byte".to_owned());
    }

    Ok(PathBuf::from(value))
}

fn normalize_optional_id(value: Option<String>) -> Result<Option<String>, String> {
    normalize_optional_string(value).and_then(|value| match value {
        Some(value) => normalize_required_id(value).map(Some),
        None => Ok(None),
    })
}

fn normalize_optional_ids(values: Option<Vec<String>>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();

    for value in values.unwrap_or_default() {
        let Some(value) = normalize_optional_string(Some(value))? else {
            continue;
        };
        let value = normalize_required_id(value)?;
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }

    Ok(normalized)
}

fn normalize_label(value: String) -> Result<String, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err("label is required".to_owned());
    }

    if value.contains('\0') {
        return Err("label contains a NUL byte".to_owned());
    }

    Ok(value)
}

fn normalize_optional_string(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Ok(None);
    }

    if value.contains('\0') {
        return Err("value contains a NUL byte".to_owned());
    }

    Ok(Some(value))
}

fn normalize_optional_strings(values: Option<Vec<String>>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();

    for value in values.unwrap_or_default() {
        let Some(value) = normalize_optional_string(Some(value))? else {
            continue;
        };
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }

    Ok(normalized)
}

fn normalize_secret(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.is_empty() {
        return Ok(None);
    }

    if value.contains('\0') {
        return Err("secret contains a NUL byte".to_owned());
    }

    Ok(Some(value))
}

fn account_sync_failure(error: dm_library::LibraryError) -> JobFailure {
    if matches!(error, dm_library::LibraryError::Cancelled) {
        return JobFailure::cancelled();
    }

    library_job_failure(error)
}

fn work_download_failure(error: dm_library::LibraryError) -> JobFailure {
    if matches!(error, dm_library::LibraryError::Cancelled)
        || matches!(
            error,
            dm_library::LibraryError::Download(dm_download::DownloadError::Cancelled)
        )
    {
        return JobFailure::cancelled();
    }

    library_job_failure(error)
}

fn library_job_failure(error: dm_library::LibraryError) -> JobFailure {
    let code = error.failure_code().to_owned();
    let message = error.support_message();
    let details = error.support_details();

    job_failure_with_details(code, message, details)
}

fn job_failure_with_details(
    code: impl Into<String>,
    message: impl Into<String>,
    details: Value,
) -> JobFailure {
    let mut failure = JobFailure::with_code(code, message);

    match details {
        Value::Object(details) => {
            for (key, value) in details {
                failure = failure.with_detail(key, value);
            }
        }
        other => {
            failure = failure.with_detail("diagnostic", other);
        }
    }

    failure
}

fn command_error(error: impl ToString) -> String {
    error.to_string()
}

async fn record_audit(logger: &AuditLogger, event: AuditEvent) {
    if let Err(error) = logger.record(event).await {
        tracing::error!(target: "dlsite_manager::audit", error = %error, "failed to write audit event");
    }
}

fn job_audit_event(event: &dm_jobs::JobEvent) -> Option<AuditEvent> {
    if event.event_kind != JobEventKind::Finished {
        return None;
    }

    let operation = job_audit_operation(event.kind.as_str());
    let details = json!({
        "jobId": event.job_id.to_string(),
        "kind": event.kind.as_str(),
        "title": event.snapshot.title.clone(),
        "metadata": event.snapshot.metadata.clone(),
        "output": event.snapshot.output.clone(),
        "errorDetails": event.snapshot.error.as_ref().map(|error| error.details.clone()),
    });

    match event.status {
        JobStatus::Succeeded => {
            Some(AuditEvent::succeeded(operation, "Job succeeded").with_details(details))
        }
        JobStatus::Cancelled => {
            Some(AuditEvent::cancelled(operation, "Job cancelled").with_details(details))
        }
        JobStatus::Failed => {
            let error = event.snapshot.error.as_ref();
            let message = error
                .map(|error| error.message.clone())
                .unwrap_or_else(|| "Job failed".to_owned());

            Some(
                AuditEvent::failed(operation, "Job failed")
                    .with_error(error.and_then(|error| error.code.clone()), message)
                    .with_details(details),
            )
        }
        _ => None,
    }
}

fn job_audit_operation(kind: &str) -> String {
    match kind {
        "accountSync" => "account.sync".to_owned(),
        "workDownload" => "work.download".to_owned(),
        "bulkWorkDownload" => "work.bulkDownload".to_owned(),
        "bulkWorkDownloadPreview" => "work.bulkDownload.preview".to_owned(),
        _ => format!("job.{kind}"),
    }
}

fn job_download_reservation_id(event: &dm_jobs::JobEvent) -> Option<&str> {
    event
        .snapshot
        .metadata
        .get(DOWNLOAD_RESERVATION_METADATA_KEY)?
        .as_str()
}

fn unpack_policy_label(policy: dm_download::UnpackPolicy) -> &'static str {
    match policy {
        dm_download::UnpackPolicy::KeepArchives => "keepArchives",
        dm_download::UnpackPolicy::UnpackWhenRecognized => "unpackWhenRecognized",
    }
}

struct BulkDownloadCandidates {
    total_count: u64,
    skipped_downloaded_count: usize,
    work_ids: Vec<String>,
}

async fn bulk_download_candidates(
    library: &Library,
    query: &ProductListQuery,
    skip_downloaded: bool,
) -> dm_library::Result<BulkDownloadCandidates> {
    let total_count = library.list_products(query).await?.total_count;
    let mut query = query.clone();
    query.limit = BULK_DOWNLOAD_PAGE_LIMIT;
    query.offset = 0;

    let mut seen = BTreeSet::new();
    let mut work_ids = Vec::new();
    let mut skipped_downloaded_count = 0usize;

    loop {
        let page = library.list_products(&query).await?;
        let page_len = page.products.len();

        for product in page.products {
            if skip_downloaded && product.download.status == WorkDownloadStatus::Downloaded {
                skipped_downloaded_count += 1;
                continue;
            }

            if seen.insert(product.work_id.clone()) {
                work_ids.push(product.work_id);
            }
        }

        if page_len == 0 || page_len < query.limit as usize {
            break;
        }

        query.offset = query.offset.saturating_add(page_len as u32);
    }

    Ok(BulkDownloadCandidates {
        total_count,
        skipped_downloaded_count,
        work_ids,
    })
}

fn bulk_download_output(
    report: &BulkWorkDownloadReport,
    skipped_queued_count: usize,
) -> JobMetadata {
    let mut output = JobMetadata::new();

    output.insert("totalCount".to_owned(), json!(report.total_count));
    output.insert("requestedCount".to_owned(), json!(report.requested_count));
    output.insert(
        "skippedDownloadedCount".to_owned(),
        json!(report.skipped_downloaded_count),
    );
    output.insert("skippedQueuedCount".to_owned(), json!(skipped_queued_count));
    output.insert("succeededCount".to_owned(), json!(report.succeeded_count));
    output.insert("failedCount".to_owned(), json!(report.failed_count));
    output.insert(
        "succeededWorks".to_owned(),
        json!(report
            .succeeded_works
            .iter()
            .map(|success| {
                json!({
                    "workId": success.work_id.as_str(),
                    "localPath": success.local_path.to_string_lossy().to_string(),
                    "fileCount": success.file_count,
                    "archiveExtracted": success.archive_extracted,
                })
            })
            .collect::<Vec<_>>()),
    );
    output.insert(
        "failedWorks".to_owned(),
        json!(report
            .failed_works
            .iter()
            .map(|failure| {
                json!({
                    "workId": failure.work_id.as_str(),
                    "errorCode": failure.error_code.as_str(),
                    "errorMessage": failure.error_message.as_str(),
                })
            })
            .collect::<Vec<_>>()),
    );

    output
}

fn bulk_download_preview_output(preview: &BulkWorkDownloadPreviewDto) -> JobMetadata {
    let mut output = JobMetadata::new();

    output.insert("totalCount".to_owned(), json!(preview.total_count));
    output.insert("requestedCount".to_owned(), json!(preview.requested_count));
    output.insert(
        "skippedDownloadedCount".to_owned(),
        json!(preview.skipped_downloaded_count),
    );
    output.insert(
        "skippedQueuedCount".to_owned(),
        json!(preview.skipped_queued_count),
    );
    output.insert("plannedCount".to_owned(), json!(preview.planned_count));
    output.insert("failedCount".to_owned(), json!(preview.failed_count));
    output.insert(
        "knownExpectedBytes".to_owned(),
        json!(preview.known_expected_bytes),
    );
    output.insert(
        "totalExpectedBytes".to_owned(),
        json!(preview.total_expected_bytes),
    );
    output.insert(
        "unknownSizeCount".to_owned(),
        json!(preview.unknown_size_count),
    );

    output
}

fn bulk_download_noop_output(
    total_count: u64,
    skipped_downloaded_count: usize,
    skipped_queued_count: usize,
) -> JobMetadata {
    let mut output = JobMetadata::new();

    output.insert("totalCount".to_owned(), json!(total_count));
    output.insert("requestedCount".to_owned(), json!(0usize));
    output.insert(
        "skippedDownloadedCount".to_owned(),
        json!(skipped_downloaded_count),
    );
    output.insert("skippedQueuedCount".to_owned(), json!(skipped_queued_count));
    output.insert("succeededCount".to_owned(), json!(0usize));
    output.insert("failedCount".to_owned(), json!(0usize));
    output.insert(
        "succeededWorks".to_owned(),
        json!(Vec::<serde_json::Value>::new()),
    );
    output.insert(
        "failedWorks".to_owned(),
        json!(Vec::<serde_json::Value>::new()),
    );

    output
}

fn setup_tracing(
    log_dir: &Path,
) -> Result<tracing_appender::non_blocking::WorkerGuard, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "runtime.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(writer)
        .with_ansi(false)
        .json()
        .finish();

    if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("failed to initialize tracing subscriber: {error}");
    }

    Ok(guard)
}

fn forward_job_events(
    app: AppHandle,
    jobs: JobManager,
    audit: AuditLogger,
    download_reservations: DownloadReservations,
) {
    let mut receiver = jobs.subscribe();

    tauri::async_runtime::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if event.event_kind == JobEventKind::Finished {
                        if let Some(reservation_id) = job_download_reservation_id(&event) {
                            let released = download_reservations.release_owner(reservation_id);

                            if released > 0 {
                                tracing::debug!(
                                    target: "dlsite_manager::downloads",
                                    job_id = %event.job_id,
                                    reservation_id,
                                    released,
                                    "released download reservations"
                                );
                            }
                        }
                    }

                    if let Some(audit_event) = job_audit_event(&event) {
                        let audit = audit.clone();
                        tauri::async_runtime::spawn(async move {
                            record_audit(&audit, audit_event).await;
                        });
                    }
                    let _ = app.emit("dm-job-event", event);
                }
                Err(RecvError::Lagged(skipped)) => {
                    eprintln!("job event forwarder skipped {skipped} lagged events");
                }
                Err(RecvError::Closed) => {
                    eprintln!("job event forwarder stopped");
                    break;
                }
            }
        }
    });
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = app.path().app_log_dir()?;
    let tracing_guard = setup_tracing(&log_dir)?;
    let audit = AuditLogger::new(log_dir.clone())?;
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let database_path: PathBuf = app_data_dir.join("dlsite-manager.sqlite");
    let storage = tauri::async_runtime::block_on(async {
        let storage = Storage::open(&database_path).await?;
        storage.run_migrations().await?;
        dm_storage::Result::Ok(storage)
    })?;
    let credential_vault_path = app_data_dir.join("credentials").join("vault.json");
    let credentials: Arc<dyn CredentialStore> =
        Arc::new(LocalCredentialStore::open(&credential_vault_path)?);
    let library = Library::new(storage.clone(), credentials);
    let jobs = JobManager::default();
    let download_reservations = DownloadReservations::default();

    tracing::info!(
        target: "dlsite_manager::app",
        log_dir = %log_dir.display(),
        data_dir = %app_data_dir.display(),
        credential_vault = %credential_vault_path.display(),
        "app setup completed"
    );
    tauri::async_runtime::block_on(record_audit(
        &audit,
        AuditEvent::succeeded("app.startup", "Started application").with_details(json!({
            "logDir": log_dir.to_string_lossy().to_string(),
            "dataDir": app_data_dir.to_string_lossy().to_string(),
        })),
    ));

    forward_job_events(
        app.handle().clone(),
        jobs.clone(),
        audit.clone(),
        download_reservations.clone(),
    );
    app.manage(AppState {
        storage,
        library,
        jobs,
        audit,
        download_reservations,
        _tracing_guard: tracing_guard,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_download_progress_throttle_limits_steady_download_updates() {
        let start = Instant::now();
        let mut throttle = WorkDownloadProgressThrottle::default();

        assert_eq!(
            throttle.emit_decision_at(dm_download::DownloadPhase::Downloading, start),
            Some(true)
        );
        assert_eq!(
            throttle.emit_decision_at(
                dm_download::DownloadPhase::Downloading,
                start + Duration::from_millis(999),
            ),
            None
        );
        assert_eq!(
            throttle.emit_decision_at(
                dm_download::DownloadPhase::Downloading,
                start + WORK_DOWNLOAD_PROGRESS_EVENT_INTERVAL,
            ),
            Some(false)
        );
    }

    #[test]
    fn work_download_progress_throttle_keeps_phase_changes_immediate() {
        let start = Instant::now();
        let mut throttle = WorkDownloadProgressThrottle::default();

        assert_eq!(
            throttle.emit_decision_at(dm_download::DownloadPhase::ProbingMetadata, start),
            Some(true)
        );
        assert_eq!(
            throttle.emit_decision_at(
                dm_download::DownloadPhase::Downloading,
                start + Duration::from_millis(1),
            ),
            Some(true)
        );
        assert_eq!(
            throttle.emit_decision_at(
                dm_download::DownloadPhase::Finalizing,
                start + Duration::from_millis(2),
            ),
            Some(true)
        );
    }

    #[test]
    fn path_root_check_allows_descendant_paths() {
        let root = PathBuf::from("/Users/example/Downloads/dlsite-manager/library");
        let path = root.join("RJ01488944");

        assert!(path_is_under_any_root(&path, &[root]));
    }

    #[test]
    fn path_root_check_rejects_sibling_prefix_paths() {
        let root = PathBuf::from("/Users/example/Downloads/dlsite-manager/library");
        let path =
            PathBuf::from("/Users/example/Downloads/dlsite-manager/library-other/RJ01488944");

        assert!(!path_is_under_any_root(&path, &[root]));
    }

    #[test]
    fn download_reservations_claim_skip_and_release_by_owner() {
        let reservations = DownloadReservations::default();
        let first_owner = reservations.next_owner_id();
        let second_owner = reservations.next_owner_id();
        let first = reservations.claim_available(
            &["RJ000001".to_owned(), "RJ000002".to_owned()],
            &first_owner,
        );
        let second = reservations.claim_available(
            &["RJ000002".to_owned(), "RJ000003".to_owned()],
            &second_owner,
        );

        assert_eq!(
            first.available,
            vec!["RJ000001".to_owned(), "RJ000002".to_owned()]
        );
        assert!(first.reserved.is_empty());
        assert_eq!(second.available, vec!["RJ000003".to_owned()]);
        assert_eq!(second.reserved, vec!["RJ000002".to_owned()]);
        assert!(reservations.is_reserved("RJ000001"));
        assert!(reservations.is_reserved("RJ000002"));
        assert!(reservations.is_reserved("RJ000003"));

        assert_eq!(reservations.release_owner(&first_owner), 2);
        assert!(!reservations.is_reserved("RJ000001"));
        assert!(!reservations.is_reserved("RJ000002"));
        assert!(reservations.is_reserved("RJ000003"));
    }

    #[test]
    fn download_reservations_split_available_deduplicates_inputs() {
        let reservations = DownloadReservations::default();
        let owner = reservations.next_owner_id();
        reservations.claim_available(&["RJ000001".to_owned()], &owner);

        let split = reservations.split_available(&[
            "RJ000001".to_owned(),
            "RJ000001".to_owned(),
            "RJ000002".to_owned(),
            "RJ000002".to_owned(),
        ]);

        assert_eq!(split.reserved, vec!["RJ000001".to_owned()]);
        assert_eq!(split.available, vec!["RJ000002".to_owned()]);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup_app)
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_accounts,
            save_account,
            set_account_enabled,
            remove_account,
            list_products,
            list_product_filter_facets,
            get_product_detail,
            set_product_custom_tags,
            start_account_sync,
            start_work_download,
            start_bulk_work_download,
            preview_bulk_work_download,
            open_work_download,
            delete_work_download,
            mark_work_downloaded,
            scan_local_work_downloads,
            list_jobs,
            get_job,
            cancel_job,
            get_job_logs,
            clear_finished_jobs,
            list_audit_events,
            get_audit_log_dir,
            open_audit_log_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
