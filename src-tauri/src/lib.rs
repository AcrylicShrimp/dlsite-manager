use dm_credentials::{CredentialStore, InMemoryCredentialStore, KeyringCredentialStore};
use dm_jobs::{JobContext, JobFailure, JobId, JobLogPage, JobManager, JobMetadata, JobProgress};
use dm_library::{
    AccountSyncRequest, DlsiteSyncSource, Library, SaveAccountRequest, SyncProgress,
    SyncProgressSink,
};
use dm_storage::{
    Account, AppSettings, ProductAgeCategory, ProductCreditGroup, ProductListItem, ProductListPage,
    ProductListQuery, ProductOwner, ProductSort, ProductTypeGroup, Storage,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::broadcast::error::RecvError;

struct AppState {
    storage: Storage,
    library: Library,
    jobs: JobManager,
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
            list_jobs,
            get_job,
            cancel_job,
            get_job_logs,
            clear_finished_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
