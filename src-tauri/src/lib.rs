use dm_credentials::{CredentialStore, InMemoryCredentialStore, KeyringCredentialStore};
use dm_jobs::{JobContext, JobFailure, JobId, JobLogPage, JobManager, JobMetadata, JobProgress};
use dm_library::{
    AccountSyncRequest, DlsiteSyncSource, DlsiteWorkDownloadSource, Library, SaveAccountRequest,
    SyncProgress, SyncProgressSink, WorkDownloadProgress, WorkDownloadProgressSink,
    WorkDownloadRemovalRequest, WorkDownloadRequest,
};
use dm_storage::{
    Account, AppSettings, ProductAgeCategory, ProductCreditGroup, ProductListItem, ProductListPage,
    ProductListQuery, ProductOwner, ProductSort, ProductTypeGroup, Storage, WorkDownloadState,
    WorkDownloadStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
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
}

const WORK_DOWNLOAD_PROGRESS_EVENT_INTERVAL: Duration = Duration::from_secs(1);

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
    let settings = settings.into_app_settings()?;

    state
        .storage
        .save_app_settings(&settings)
        .await
        .map_err(command_error)?;

    Ok(AppSettingsDto::from(settings))
}

#[tauri::command]
async fn list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountDto>, String> {
    state
        .library
        .accounts()
        .await
        .map(|accounts| accounts.into_iter().map(AccountDto::from).collect())
        .map_err(command_error)
}

#[tauri::command]
async fn save_account(
    state: State<'_, AppState>,
    request: SaveAccountCommandRequest,
) -> Result<AccountDto, String> {
    state
        .library
        .save_account(request.into_library_request()?)
        .await
        .map(AccountDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn set_account_enabled(
    state: State<'_, AppState>,
    request: SetAccountEnabledRequest,
) -> Result<(), String> {
    state
        .library
        .set_account_enabled(&normalize_required_id(request.account_id)?, request.enabled)
        .await
        .map_err(command_error)
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
async fn start_account_sync(
    state: State<'_, AppState>,
    request: StartAccountSyncRequest,
) -> Result<StartJobResponse, String> {
    let account_id = normalize_required_id(request.account_id)?;
    let password = normalize_secret(request.password)?;
    let library = state.library.clone();
    let mut metadata = JobMetadata::new();

    metadata.insert("accountId".to_owned(), json!(account_id));

    let job_id = state.jobs.spawn(
        "accountSync",
        format!("Sync {account_id}"),
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
                        account_id: &account_id,
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
    let work_id = normalize_required_id(request.work_id)?;
    let account_id = normalize_optional_id(request.account_id)?;
    let password = normalize_secret(request.password)?;
    let settings = state.storage.app_settings().await.map_err(command_error)?;
    let library_root = required_library_root(&settings)?;
    let download_root = effective_download_root(&app, &settings)?;
    let unpack_policy = request.unpack_policy.unwrap_or_default().into();
    let replace_existing = request.replace_existing.unwrap_or(false);
    let library = state.library.clone();
    let mut metadata = JobMetadata::new();

    metadata.insert("workId".to_owned(), json!(work_id));
    if let Some(account_id) = &account_id {
        metadata.insert("accountId".to_owned(), json!(account_id));
    }

    let job_work_id = work_id.clone();
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

    Ok(StartJobResponse {
        job_id: job_id.to_string(),
    })
}

#[tauri::command]
async fn open_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: OpenWorkDownloadRequest,
) -> Result<(), String> {
    let work_id = normalize_required_id(request.work_id)?;
    let download = state
        .storage
        .work_download_state(&work_id)
        .await
        .map_err(command_error)?;

    if download.status != WorkDownloadStatus::Downloaded {
        return Err(format!("{work_id} is not downloaded"));
    }

    let local_path = download
        .local_path
        .as_deref()
        .ok_or_else(|| format!("{work_id} does not have a local path"))?;
    let canonical_path = canonicalize_existing_path(local_path)?;
    let settings = state.storage.app_settings().await.map_err(command_error)?;
    let allowed_roots = canonical_open_roots(&app, &settings);

    if !path_is_under_any_root(&canonical_path, &allowed_roots) {
        return Err(format!(
            "download path is outside the configured library or staging folders: {}",
            canonical_path.display()
        ));
    }

    app.opener()
        .open_path(
            canonical_path.to_string_lossy().into_owned(),
            None::<String>,
        )
        .map_err(command_error)
}

#[tauri::command]
async fn delete_work_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DeleteWorkDownloadRequest,
) -> Result<WorkDownloadStateDto, String> {
    let work_id = normalize_required_id(request.work_id)?;
    let settings = state.storage.app_settings().await.map_err(command_error)?;
    let library_root = required_library_root(&settings)?;
    let download_root = effective_download_root(&app, &settings)?;

    state
        .library
        .remove_work_download(WorkDownloadRemovalRequest::new(
            &work_id,
            &library_root,
            &download_root,
        ))
        .await
        .map(WorkDownloadStateDto::from)
        .map_err(command_error)
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

impl From<Account> for AccountDto {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            label: account.label,
            login_name: account.login_name,
            has_credential: account.credential_ref.is_some(),
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
    remember_password: bool,
    enabled: bool,
}

impl SaveAccountCommandRequest {
    fn into_library_request(self) -> Result<SaveAccountRequest, String> {
        Ok(SaveAccountRequest {
            id: normalize_optional_id(self.id)?,
            label: normalize_label(self.label)?,
            login_name: normalize_optional_string(self.login_name)?,
            password: normalize_secret(self.password)?,
            remember_password: self.remember_password,
            enabled: self.enabled,
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
struct ListProductsRequest {
    search: Option<String>,
    account_id: Option<String>,
    type_group: Option<ProductTypeGroupDto>,
    age_category: Option<ProductAgeCategoryDto>,
    sort: Option<ProductSortDto>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl ListProductsRequest {
    fn into_query(self) -> Result<ProductListQuery, String> {
        Ok(ProductListQuery {
            search: normalize_optional_string(self.search)?,
            account_id: normalize_optional_id(self.account_id)?,
            type_group: self.type_group.map(Into::into),
            age_category: self.age_category.map(Into::into),
            sort: self.sort.unwrap_or_default().into(),
            limit: self.limit.unwrap_or(100).clamp(1, 500),
            offset: self.offset.unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProductSortDto {
    #[default]
    TitleAsc,
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
struct OpenWorkDownloadRequest {
    work_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteWorkDownloadRequest {
    work_id: String,
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
                        dm_download::DownloadPhase::ProbingMetadata => {
                            self.context.set_phase("probingDownload")
                        }
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

fn normalize_optional_id(value: Option<String>) -> Result<Option<String>, String> {
    normalize_optional_string(value).and_then(|value| match value {
        Some(value) => normalize_required_id(value).map(Some),
        None => Ok(None),
    })
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

    let code = match &error {
        dm_library::LibraryError::Storage(_) => "storage",
        dm_library::LibraryError::Credentials(_) => "credentials",
        dm_library::LibraryError::Api(_) => "api",
        dm_library::LibraryError::SyncSource(_) => "sync_source",
        dm_library::LibraryError::AccountNotFound(_) => "account_not_found",
        dm_library::LibraryError::AccountDisabled(_) => "account_disabled",
        dm_library::LibraryError::MissingLoginName(_) => "missing_login_name",
        dm_library::LibraryError::MissingPassword(_) => "missing_password",
        dm_library::LibraryError::Cancelled => "cancelled",
        dm_library::LibraryError::Download(_) => "download",
        dm_library::LibraryError::DownloadAccountNotFound(_) => "download_account_not_found",
        dm_library::LibraryError::DownloadTargetExists(_) => "download_target_exists",
        dm_library::LibraryError::DownloadPathOutsideRoots(_) => "download_path_outside_roots",
        dm_library::LibraryError::Io(_) => "io",
        dm_library::LibraryError::Json(_) => "json",
    };

    JobFailure::with_code(code, error.to_string())
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

    let code = match &error {
        dm_library::LibraryError::Storage(_) => "storage",
        dm_library::LibraryError::Credentials(_) => "credentials",
        dm_library::LibraryError::Api(_) => "api",
        dm_library::LibraryError::SyncSource(_) => "sync_source",
        dm_library::LibraryError::AccountNotFound(_) => "account_not_found",
        dm_library::LibraryError::AccountDisabled(_) => "account_disabled",
        dm_library::LibraryError::MissingLoginName(_) => "missing_login_name",
        dm_library::LibraryError::MissingPassword(_) => "missing_password",
        dm_library::LibraryError::Cancelled => "cancelled",
        dm_library::LibraryError::Download(_) => "download",
        dm_library::LibraryError::DownloadAccountNotFound(_) => "download_account_not_found",
        dm_library::LibraryError::DownloadTargetExists(_) => "download_target_exists",
        dm_library::LibraryError::DownloadPathOutsideRoots(_) => "download_path_outside_roots",
        dm_library::LibraryError::Io(_) => "io",
        dm_library::LibraryError::Json(_) => "json",
    };

    JobFailure::with_code(code, error.to_string())
}

fn command_error(error: impl ToString) -> String {
    error.to_string()
}

fn forward_job_events(app: AppHandle, jobs: JobManager) {
    let mut receiver = jobs.subscribe();

    tauri::async_runtime::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
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
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let database_path: PathBuf = app_data_dir.join("dlsite-manager.sqlite");
    let storage = tauri::async_runtime::block_on(async {
        let storage = Storage::open(&database_path).await?;
        storage.run_migrations().await?;
        dm_storage::Result::Ok(storage)
    })?;
    let credentials: Arc<dyn CredentialStore> = match KeyringCredentialStore::native_default() {
        Ok(store) => Arc::new(store),
        Err(error) => {
            eprintln!("failed to initialize native credential store: {error}");
            Arc::new(InMemoryCredentialStore::new())
        }
    };
    let library = Library::new(storage.clone(), credentials);
    let jobs = JobManager::default();

    forward_job_events(app.handle().clone(), jobs.clone());
    app.manage(AppState {
        storage,
        library,
        jobs,
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
            list_products,
            start_account_sync,
            start_work_download,
            open_work_download,
            delete_work_download,
            list_jobs,
            get_job,
            cancel_job,
            get_job_logs,
            clear_finished_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
