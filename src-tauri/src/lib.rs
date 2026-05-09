use dm_credentials::{CredentialStore, InMemoryCredentialStore, KeyringCredentialStore};
use dm_library::{
    AccountSyncRequest, DlsiteSyncSource, Library, SaveAccountRequest, SyncProgress,
    SyncProgressSink,
};
use dm_storage::{
    Account, AppSettings, ProductListItem, ProductListPage, ProductListQuery, ProductOwner,
    ProductSort, Storage,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    storage: Storage,
    library: Library,
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
async fn sync_account(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SyncAccountCommandRequest,
) -> Result<AccountSyncReportDto, String> {
    let account_id = normalize_required_id(request.account_id)?;
    let progress_sink = TauriSyncProgressSink {
        app,
        account_id: account_id.clone(),
    };
    let client =
        dm_api::DlsiteClient::new(dm_api::DlsiteClientConfig::default()).map_err(command_error)?;
    let source = DlsiteSyncSource::new(client);
    let password = normalize_secret(request.password)?;

    state
        .library
        .sync_account_with_source(
            AccountSyncRequest {
                account_id: &account_id,
                password: password.as_deref(),
                cancellation_token: None,
                progress_sink: Some(&progress_sink),
            },
            &source,
        )
        .await
        .map(AccountSyncReportDto::from)
        .map_err(command_error)
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
    sort: Option<ProductSortDto>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl ListProductsRequest {
    fn into_query(self) -> Result<ProductListQuery, String> {
        Ok(ProductListQuery {
            search: normalize_optional_string(self.search)?,
            account_id: normalize_optional_id(self.account_id)?,
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
struct SyncAccountCommandRequest {
    account_id: String,
    password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountSyncReportDto {
    account_id: String,
    sync_run_id: String,
    purchased_count: usize,
    cached_work_count: usize,
    page_limit: Option<usize>,
    concurrency: Option<usize>,
}

impl From<dm_library::AccountSyncReport> for AccountSyncReportDto {
    fn from(report: dm_library::AccountSyncReport) -> Self {
        Self {
            account_id: report.account_id,
            sync_run_id: report.sync_run_id,
            purchased_count: report.purchased_count,
            cached_work_count: report.cached_work_count,
            page_limit: report.page_limit,
            concurrency: report.concurrency,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncProgressEvent {
    account_id: String,
    phase: &'static str,
    work_count: Option<usize>,
    sync_run_id: Option<String>,
    cached_work_count: Option<usize>,
}

struct TauriSyncProgressSink {
    app: AppHandle,
    account_id: String,
}

impl SyncProgressSink for TauriSyncProgressSink {
    fn emit(&self, progress: SyncProgress) {
        let event = SyncProgressEvent::from_progress(self.account_id.clone(), progress);
        let _ = self.app.emit("library-sync-progress", event);
    }
}

impl SyncProgressEvent {
    fn from_progress(account_id: String, progress: SyncProgress) -> Self {
        match progress {
            SyncProgress::LoggingIn => Self {
                account_id,
                phase: "loggingIn",
                work_count: None,
                sync_run_id: None,
                cached_work_count: None,
            },
            SyncProgress::LoadingCount => Self {
                account_id,
                phase: "loadingCount",
                work_count: None,
                sync_run_id: None,
                cached_work_count: None,
            },
            SyncProgress::LoadingPurchases => Self {
                account_id,
                phase: "loadingPurchases",
                work_count: None,
                sync_run_id: None,
                cached_work_count: None,
            },
            SyncProgress::LoadingWorks { work_count } => Self {
                account_id,
                phase: "loadingWorks",
                work_count: Some(work_count),
                sync_run_id: None,
                cached_work_count: None,
            },
            SyncProgress::Committing { work_count } => Self {
                account_id,
                phase: "committing",
                work_count: Some(work_count),
                sync_run_id: None,
                cached_work_count: None,
            },
            SyncProgress::Completed {
                sync_run_id,
                cached_work_count,
            } => Self {
                account_id,
                phase: "completed",
                work_count: None,
                sync_run_id: Some(sync_run_id),
                cached_work_count: Some(cached_work_count),
            },
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

fn command_error(error: impl ToString) -> String {
    error.to_string()
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

    app.manage(AppState { storage, library });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(setup_app)
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_accounts,
            save_account,
            set_account_enabled,
            list_products,
            sync_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
