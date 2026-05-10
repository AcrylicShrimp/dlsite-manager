use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use dm_api::{
    ContentCount, ContentQuery, Credentials, DlsiteClient, DmApiError, Language, LocalizedText,
    Purchase, Work, WorkId,
};
use dm_credentials::{CredentialRef, CredentialStore, CredentialsError};
pub use dm_jobs::CancellationToken;
use dm_storage::{
    Account, AccountSyncCommit, AccountUpsert, AccountWork, CachedWork, ProductListPage,
    ProductListQuery, Storage, StorageError, SyncCancellation, SyncFailure,
};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, LibraryError>;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("storage error")]
    Storage(#[from] StorageError),
    #[error("credential error")]
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

fn now_string() -> String {
    datetime_to_string(Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_api::{AgeCategory, Maker, WorkKind, WorkThumbnail};
    use dm_credentials::InMemoryCredentialStore;
    use dm_storage::{ProductSort, SyncRunStatus};
    use std::sync::Mutex;
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

    #[derive(Debug, Default)]
    struct RecordingProgressSink {
        events: Mutex<Vec<SyncProgress>>,
    }

    impl SyncProgressSink for RecordingProgressSink {
        fn emit(&self, progress: SyncProgress) {
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

    #[tokio::test]
    async fn saves_account_and_password_reference() -> Result<()> {
        let library = migrated_library().await?;

        let account = library.save_account(save_account_request(true)).await?;

        assert_eq!(account.id, "account-a");
        assert_eq!(
            account.credential_ref,
            Some("account:account-a:password".to_owned())
        );

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
