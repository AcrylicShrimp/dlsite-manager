use serde::Deserialize;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult},
    QueryBuilder, Row, Sqlite, SqlitePool, Transaction,
};
use std::path::Path;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
const LIBRARY_ROOT_KEY: &str = "library_root";
const DOWNLOAD_ROOT_KEY: &str = "download_root";
const MISSING_WORK_DETAIL_STATUS: &str = "missing_from_content_works";

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("database migration error")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("write transaction is already finished")]
    TransactionFinished,
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("invalid stored value for {field}: {value}")]
    InvalidStoredValue { field: &'static str, value: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppSettings {
    pub library_root: Option<String>,
    pub download_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub login_name: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpsert {
    pub id: String,
    pub label: String,
    pub login_name: Option<String>,
    pub credential_ref: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedWork {
    pub work_id: String,
    pub title: String,
    pub title_json: String,
    pub maker_id: Option<String>,
    pub maker_name: Option<String>,
    pub maker_json: Option<String>,
    pub work_type: Option<String>,
    pub age_category: Option<String>,
    pub thumbnail_url: Option<String>,
    pub registered_at: Option<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub raw_json: String,
    pub last_detail_sync_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWork {
    pub work_id: String,
    pub purchased_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSyncCommit {
    pub sync_run_id: String,
    pub account_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub works: Vec<CachedWork>,
    pub account_works: Vec<AccountWork>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFailure {
    pub sync_run_id: String,
    pub account_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCancellation {
    pub sync_run_id: String,
    pub account_id: String,
    pub started_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncRunStatus {
    Started,
    Completed,
    Failed,
    Cancelled,
}

impl SyncRunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "started" => Ok(Self::Started),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(StorageError::InvalidStoredValue {
                field: "sync_runs.status",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRun {
    pub id: String,
    pub account_id: String,
    pub status: SyncRunStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductSort {
    TitleAsc,
    LatestPurchaseDesc,
    PublishedAtDesc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductAgeCategory {
    All,
    R15,
    R18,
}

impl ProductAgeCategory {
    fn as_storage_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::R15 => "r15",
            Self::R18 => "r18",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductListQuery {
    pub search: Option<String>,
    pub account_id: Option<String>,
    pub age_category: Option<ProductAgeCategory>,
    pub sort: ProductSort,
    pub limit: u32,
    pub offset: u32,
}

impl Default for ProductListQuery {
    fn default() -> Self {
        Self {
            search: None,
            account_id: None,
            age_category: None,
            sort: ProductSort::TitleAsc,
            limit: 100,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductListPage {
    pub total_count: u64,
    pub products: Vec<ProductListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductListItem {
    pub work_id: String,
    pub title: String,
    pub maker_name: Option<String>,
    pub work_type: Option<String>,
    pub age_category: Option<String>,
    pub thumbnail_url: Option<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub earliest_purchased_at: Option<String>,
    pub latest_purchased_at: Option<String>,
    pub credit_groups: Vec<ProductCreditGroup>,
    pub owners: Vec<ProductOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductCreditGroup {
    pub kind: String,
    pub label: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductOwner {
    pub account_id: String,
    pub label: String,
    pub purchased_at: Option<String>,
}

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);

        Self::connect_with(options).await
    }

    pub async fn open_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .foreign_keys(true);

        Self::connect_with(options).await
    }

    pub async fn connect_with(options: SqliteConnectOptions) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    pub async fn begin_write(&self) -> Result<WriteTransaction<'_>> {
        let transaction = self.pool.begin().await?;
        Ok(WriteTransaction {
            transaction: Some(transaction),
        })
    }

    pub async fn app_settings(&self) -> Result<AppSettings> {
        let rows = sqlx::query("SELECT key, value FROM app_settings")
            .fetch_all(&self.pool)
            .await?;
        let mut settings = AppSettings::default();

        for row in rows {
            let key: String = row.try_get("key")?;
            let value: String = row.try_get("value")?;

            match key.as_str() {
                LIBRARY_ROOT_KEY => settings.library_root = Some(value),
                DOWNLOAD_ROOT_KEY => settings.download_root = Some(value),
                _ => {}
            }
        }

        Ok(settings)
    }

    pub async fn save_app_settings(&self, settings: &AppSettings) -> Result<()> {
        let mut transaction = self.begin_write().await?;

        transaction
            .set_setting(LIBRARY_ROOT_KEY, settings.library_root.as_deref())
            .await?;
        transaction
            .set_setting(DOWNLOAD_ROOT_KEY, settings.download_root.as_deref())
            .await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn accounts(&self) -> Result<Vec<Account>> {
        let rows = sqlx::query(
            "SELECT id, label, login_name, credential_ref, enabled, created_at,
                    updated_at, last_login_at, last_sync_at
             FROM accounts
             ORDER BY label COLLATE NOCASE ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(account_from_row).collect()
    }

    pub async fn save_account(&self, account: &AccountUpsert) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction.upsert_account(account).await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn set_account_enabled(&self, account_id: &str, enabled: bool) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction.set_account_enabled(account_id, enabled).await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn record_account_login(&self, account_id: &str, logged_in_at: &str) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction
            .record_account_login(account_id, logged_in_at)
            .await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn commit_account_sync(&self, sync: &AccountSyncCommit) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction.commit_account_sync(sync).await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn record_sync_failure(&self, failure: &SyncFailure) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction.record_sync_failure(failure).await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn record_sync_cancellation(&self, cancellation: &SyncCancellation) -> Result<()> {
        let mut transaction = self.begin_write().await?;
        transaction.record_sync_cancellation(cancellation).await?;
        transaction.commit().await?;

        Ok(())
    }

    pub async fn sync_runs_for_account(&self, account_id: &str) -> Result<Vec<SyncRun>> {
        let rows = sqlx::query(
            "SELECT id, account_id, status, started_at, completed_at, error_code,
                    error_message
             FROM sync_runs
             WHERE account_id = ?1
             ORDER BY started_at DESC, id DESC",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(sync_run_from_row).collect()
    }

    pub async fn list_products(&self, query: &ProductListQuery) -> Result<ProductListPage> {
        let total_count = self.count_products(query).await?;
        let products = self.fetch_product_page(query).await?;

        Ok(ProductListPage {
            total_count,
            products,
        })
    }

    async fn count_products(&self, query: &ProductListQuery) -> Result<u64> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(*) AS count FROM (
                 SELECT w.work_id
                 FROM works w
                 JOIN account_works aw ON aw.work_id = w.work_id
                 JOIN accounts a ON a.id = aw.account_id
                 WHERE aw.is_current = 1 AND a.enabled = 1",
        );

        push_product_filters(&mut builder, query);
        builder.push(" GROUP BY w.work_id)");

        let row = builder.build().fetch_one(&self.pool).await?;
        let count: i64 = row.try_get("count")?;

        Ok(count as u64)
    }

    async fn fetch_product_page(&self, query: &ProductListQuery) -> Result<Vec<ProductListItem>> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            "WITH visible_works AS (
                 SELECT
                    w.work_id,
                    lower(w.title) AS sort_title,
                    COALESCE(w.published_at, '') AS sort_published_at,
                    MIN(aw.purchased_at) AS earliest_purchased_at,
                    MAX(aw.purchased_at) AS latest_purchased_at
                 FROM works w
                 JOIN account_works aw ON aw.work_id = w.work_id
                 JOIN accounts a ON a.id = aw.account_id
                 WHERE aw.is_current = 1 AND a.enabled = 1",
        );

        push_product_filters(&mut builder, query);
        builder.push(" GROUP BY w.work_id ORDER BY ");
        push_product_sort(&mut builder, query.sort);
        builder.push(" LIMIT ");
        builder.push_bind(i64::from(query.limit));
        builder.push(" OFFSET ");
        builder.push_bind(i64::from(query.offset));
        builder.push(
            ")
             SELECT
                w.work_id,
                w.title,
                w.maker_name,
                w.work_type,
                w.age_category,
                w.thumbnail_url,
                w.published_at,
                w.updated_at,
                w.raw_json,
                vw.earliest_purchased_at,
                vw.latest_purchased_at,
                aw.account_id,
                a.label AS account_label,
                aw.purchased_at
             FROM visible_works vw
             JOIN works w ON w.work_id = vw.work_id
             JOIN account_works aw ON aw.work_id = w.work_id
             JOIN accounts a ON a.id = aw.account_id
             WHERE aw.is_current = 1 AND a.enabled = 1
             ORDER BY ",
        );
        push_outer_product_sort(&mut builder, query.sort);
        builder.push(", lower(a.label) ASC, aw.account_id ASC");

        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut products = Vec::<ProductListItem>::new();

        for row in rows {
            let work_id: String = row.try_get("work_id")?;
            let raw_json: String = row.try_get("raw_json")?;
            let owner = ProductOwner {
                account_id: row.try_get("account_id")?,
                label: row.try_get("account_label")?,
                purchased_at: row.try_get("purchased_at")?,
            };

            if let Some(product) = products
                .last_mut()
                .filter(|product| product.work_id == work_id)
            {
                product.owners.push(owner);
                continue;
            }

            products.push(ProductListItem {
                work_id,
                title: row.try_get("title")?,
                maker_name: row.try_get("maker_name")?,
                work_type: row.try_get("work_type")?,
                age_category: row.try_get("age_category")?,
                thumbnail_url: row.try_get("thumbnail_url")?,
                published_at: row.try_get("published_at")?,
                updated_at: row.try_get("updated_at")?,
                earliest_purchased_at: row.try_get("earliest_purchased_at")?,
                latest_purchased_at: row.try_get("latest_purchased_at")?,
                credit_groups: product_credit_groups_from_raw_json(&raw_json),
                owners: vec![owner],
            });
        }

        Ok(products)
    }
}

pub struct WriteTransaction<'storage> {
    transaction: Option<Transaction<'storage, Sqlite>>,
}

impl WriteTransaction<'_> {
    pub async fn execute(&mut self, sql: &'static str) -> Result<SqliteQueryResult> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        Ok(sqlx::query(sql).execute(&mut **transaction).await?)
    }

    pub async fn commit(mut self) -> Result<()> {
        let transaction = self
            .transaction
            .take()
            .ok_or(StorageError::TransactionFinished)?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<()> {
        let transaction = self
            .transaction
            .take()
            .ok_or(StorageError::TransactionFinished)?;
        transaction.rollback().await?;
        Ok(())
    }

    async fn set_setting(&mut self, key: &str, value: Option<&str>) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        match value {
            Some(value) => {
                sqlx::query(
                    "INSERT INTO app_settings (key, value, updated_at)
                     VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                     ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value,
                        updated_at = excluded.updated_at",
                )
                .bind(key)
                .bind(value)
                .execute(&mut **transaction)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM app_settings WHERE key = ?1")
                    .bind(key)
                    .execute(&mut **transaction)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_account(&mut self, account: &AccountUpsert) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        sqlx::query(
            "INSERT INTO accounts (
                id, label, login_name, credential_ref, enabled, created_at, updated_at
             )
             VALUES (
                ?1, ?2, ?3, ?4, ?5,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )
             ON CONFLICT(id) DO UPDATE SET
                label = excluded.label,
                login_name = excluded.login_name,
                credential_ref = excluded.credential_ref,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
        )
        .bind(&account.id)
        .bind(&account.label)
        .bind(&account.login_name)
        .bind(&account.credential_ref)
        .bind(bool_to_i64(account.enabled))
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    pub async fn set_account_enabled(&mut self, account_id: &str, enabled: bool) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        let result = sqlx::query(
            "UPDATE accounts
             SET enabled = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
        )
        .bind(account_id)
        .bind(bool_to_i64(enabled))
        .execute(&mut **transaction)
        .await?;

        ensure_changed(result, "account", account_id)
    }

    pub async fn record_account_login(
        &mut self,
        account_id: &str,
        logged_in_at: &str,
    ) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        let result = sqlx::query(
            "UPDATE accounts
             SET last_login_at = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
        )
        .bind(account_id)
        .bind(logged_in_at)
        .execute(&mut **transaction)
        .await?;

        ensure_changed(result, "account", account_id)
    }

    pub async fn commit_account_sync(&mut self, sync: &AccountSyncCommit) -> Result<()> {
        self.ensure_account_exists(&sync.account_id).await?;
        self.insert_sync_run(
            &sync.sync_run_id,
            &sync.account_id,
            SyncRunStatus::Completed,
            &sync.started_at,
            Some(&sync.completed_at),
            None,
            None,
        )
        .await?;

        for work in &sync.works {
            self.upsert_work(work).await?;
        }

        for account_work in &sync.account_works {
            self.upsert_account_work(
                &sync.account_id,
                account_work,
                &sync.sync_run_id,
                &sync.completed_at,
            )
            .await?;
        }

        self.mark_account_works_not_seen_in_sync(&sync.account_id, &sync.sync_run_id)
            .await?;
        self.record_account_sync_completed(&sync.account_id, &sync.completed_at)
            .await?;

        Ok(())
    }

    pub async fn record_sync_failure(&mut self, failure: &SyncFailure) -> Result<()> {
        self.ensure_account_exists(&failure.account_id).await?;
        self.insert_sync_run(
            &failure.sync_run_id,
            &failure.account_id,
            SyncRunStatus::Failed,
            &failure.started_at,
            Some(&failure.completed_at),
            failure.error_code.as_deref(),
            failure.error_message.as_deref(),
        )
        .await
    }

    pub async fn record_sync_cancellation(
        &mut self,
        cancellation: &SyncCancellation,
    ) -> Result<()> {
        self.ensure_account_exists(&cancellation.account_id).await?;
        self.insert_sync_run(
            &cancellation.sync_run_id,
            &cancellation.account_id,
            SyncRunStatus::Cancelled,
            &cancellation.started_at,
            Some(&cancellation.completed_at),
            Some("cancelled"),
            None,
        )
        .await
    }

    async fn ensure_account_exists(&mut self, account_id: &str) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;
        let row = sqlx::query("SELECT 1 FROM accounts WHERE id = ?1")
            .bind(account_id)
            .fetch_optional(&mut **transaction)
            .await?;

        if row.is_some() {
            Ok(())
        } else {
            Err(StorageError::NotFound {
                entity: "account",
                id: account_id.to_owned(),
            })
        }
    }

    async fn insert_sync_run(
        &mut self,
        sync_run_id: &str,
        account_id: &str,
        status: SyncRunStatus,
        started_at: &str,
        completed_at: Option<&str>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        sqlx::query(
            "INSERT INTO sync_runs (
                id, account_id, status, started_at, completed_at, error_code,
                error_message
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(sync_run_id)
        .bind(account_id)
        .bind(status.as_str())
        .bind(started_at)
        .bind(completed_at)
        .bind(error_code)
        .bind(error_message)
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    async fn upsert_work(&mut self, work: &CachedWork) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        sqlx::query(
            "INSERT INTO works (
                work_id, title, title_json, maker_id, maker_name, maker_json,
                work_type, age_category, thumbnail_url, registered_at,
                published_at, updated_at, raw_json, last_detail_sync_at
             )
             VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
             )
             ON CONFLICT(work_id) DO UPDATE SET
                title = excluded.title,
                title_json = excluded.title_json,
                maker_id = excluded.maker_id,
                maker_name = excluded.maker_name,
                maker_json = excluded.maker_json,
                work_type = excluded.work_type,
                age_category = excluded.age_category,
                thumbnail_url = excluded.thumbnail_url,
                registered_at = excluded.registered_at,
                published_at = excluded.published_at,
                updated_at = excluded.updated_at,
                raw_json = excluded.raw_json,
                last_detail_sync_at = excluded.last_detail_sync_at",
        )
        .bind(&work.work_id)
        .bind(&work.title)
        .bind(&work.title_json)
        .bind(&work.maker_id)
        .bind(&work.maker_name)
        .bind(&work.maker_json)
        .bind(&work.work_type)
        .bind(&work.age_category)
        .bind(&work.thumbnail_url)
        .bind(&work.registered_at)
        .bind(&work.published_at)
        .bind(&work.updated_at)
        .bind(&work.raw_json)
        .bind(&work.last_detail_sync_at)
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    async fn upsert_account_work(
        &mut self,
        account_id: &str,
        account_work: &AccountWork,
        sync_run_id: &str,
        seen_at: &str,
    ) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        sqlx::query(
            "INSERT INTO account_works (
                account_id, work_id, purchased_at, first_seen_at, last_seen_at,
                last_seen_sync_run_id, is_current
             )
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, 1)
             ON CONFLICT(account_id, work_id) DO UPDATE SET
                purchased_at = excluded.purchased_at,
                last_seen_at = excluded.last_seen_at,
                last_seen_sync_run_id = excluded.last_seen_sync_run_id,
                is_current = 1",
        )
        .bind(account_id)
        .bind(&account_work.work_id)
        .bind(&account_work.purchased_at)
        .bind(seen_at)
        .bind(sync_run_id)
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    async fn mark_account_works_not_seen_in_sync(
        &mut self,
        account_id: &str,
        sync_run_id: &str,
    ) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        sqlx::query(
            "UPDATE account_works
             SET is_current = 0
             WHERE account_id = ?1
               AND is_current = 1
               AND (
                    last_seen_sync_run_id IS NULL
                    OR last_seen_sync_run_id <> ?2
               )",
        )
        .bind(account_id)
        .bind(sync_run_id)
        .execute(&mut **transaction)
        .await?;

        Ok(())
    }

    async fn record_account_sync_completed(
        &mut self,
        account_id: &str,
        completed_at: &str,
    ) -> Result<()> {
        let transaction = self
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;

        let result = sqlx::query(
            "UPDATE accounts
             SET last_sync_at = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
        )
        .bind(account_id)
        .bind(completed_at)
        .execute(&mut **transaction)
        .await?;

        ensure_changed(result, "account", account_id)
    }
}

fn account_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Account> {
    Ok(Account {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        login_name: row.try_get("login_name")?,
        credential_ref: row.try_get("credential_ref")?,
        enabled: i64_to_bool(row.try_get("enabled")?, "accounts.enabled")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_login_at: row.try_get("last_login_at")?,
        last_sync_at: row.try_get("last_sync_at")?,
    })
}

fn sync_run_from_row(row: sqlx::sqlite::SqliteRow) -> Result<SyncRun> {
    let status: String = row.try_get("status")?;

    Ok(SyncRun {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        status: SyncRunStatus::from_str(&status)?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        error_code: row.try_get("error_code")?,
        error_message: row.try_get("error_message")?,
    })
}

fn push_product_filters(builder: &mut QueryBuilder<Sqlite>, query: &ProductListQuery) {
    builder.push(
        " AND COALESCE(
            CASE
                WHEN json_valid(w.raw_json) THEN json_extract(w.raw_json, '$.detail_status')
            END,
            ''
        ) <> ",
    );
    builder.push_bind(MISSING_WORK_DETAIL_STATUS);

    if let Some(account_id) = query.account_id.as_deref() {
        builder.push(" AND aw.account_id = ");
        builder.push_bind(account_id.to_owned());
    }

    if let Some(age_category) = query.age_category {
        builder.push(" AND w.age_category = ");
        builder.push_bind(age_category.as_storage_value());
    }

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let pattern = format!("%{}%", escape_like(search));
        builder.push(
            " AND (
                w.work_id LIKE ",
        );
        builder.push_bind(pattern.clone());
        builder.push(
            " ESCAPE '\\'
                OR w.title LIKE ",
        );
        builder.push_bind(pattern.clone());
        builder.push(
            " ESCAPE '\\'
                OR w.maker_name LIKE ",
        );
        builder.push_bind(pattern);
        builder.push(" ESCAPE '\\')");
    }
}

fn push_product_sort(builder: &mut QueryBuilder<Sqlite>, sort: ProductSort) {
    match sort {
        ProductSort::TitleAsc => {
            builder.push("sort_title ASC, w.work_id ASC");
        }
        ProductSort::LatestPurchaseDesc => {
            builder.push("latest_purchased_at DESC, sort_title ASC, w.work_id ASC");
        }
        ProductSort::PublishedAtDesc => {
            builder.push("sort_published_at DESC, sort_title ASC, w.work_id ASC");
        }
    }
}

fn push_outer_product_sort(builder: &mut QueryBuilder<Sqlite>, sort: ProductSort) {
    match sort {
        ProductSort::TitleAsc => {
            builder.push("vw.sort_title ASC, w.work_id ASC");
        }
        ProductSort::LatestPurchaseDesc => {
            builder.push("vw.latest_purchased_at DESC, vw.sort_title ASC, w.work_id ASC");
        }
        ProductSort::PublishedAtDesc => {
            builder.push("vw.sort_published_at DESC, vw.sort_title ASC, w.work_id ASC");
        }
    }
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }

    escaped
}

fn product_credit_groups_from_raw_json(raw_json: &str) -> Vec<ProductCreditGroup> {
    let Ok(work) = serde_json::from_str::<RawWorkCredits>(raw_json) else {
        return Vec::new();
    };

    let mut groups = credit_group_templates();

    let mut tags = work.tags;
    tags.sort_by(|left, right| {
        credit_sort_key(&left.class, &left.name).cmp(&credit_sort_key(&right.class, &right.name))
    });

    for tag in tags {
        let Some((kind, _label, _rank)) = credit_kind_label_and_rank(&tag.class) else {
            continue;
        };
        let name = tag.name.trim();

        if name.is_empty() {
            continue;
        }

        if let Some((_kind, _label, names)) = groups
            .iter_mut()
            .find(|(group_kind, _, _)| *group_kind == kind)
        {
            if !names.iter().any(|existing| existing.as_str() == name) {
                names.push(name.to_owned());
            }
        }
    }

    groups
        .into_iter()
        .filter_map(|(kind, label, names)| {
            if names.is_empty() {
                None
            } else {
                Some(ProductCreditGroup {
                    kind: kind.to_owned(),
                    label: label.to_owned(),
                    names,
                })
            }
        })
        .collect()
}

fn credit_group_templates() -> Vec<(&'static str, &'static str, Vec<String>)> {
    vec![
        ("voice", "CV", Vec::new()),
        ("illust", "Illust", Vec::new()),
        ("scenario", "Scenario", Vec::new()),
        ("creator", "Creator", Vec::new()),
        ("music", "Music", Vec::new()),
        ("other", "Other", Vec::new()),
    ]
}

fn credit_sort_key(class: &str, name: &str) -> (u8, String) {
    let rank = credit_kind_label_and_rank(class)
        .map(|(_kind, _label, rank)| rank)
        .unwrap_or(u8::MAX);

    (rank, name.trim().to_lowercase())
}

fn credit_kind_label_and_rank(class: &str) -> Option<(&'static str, &'static str, u8)> {
    match class {
        "voice_by" => Some(("voice", "CV", 0)),
        "illust_by" => Some(("illust", "Illust", 1)),
        "scenario_by" => Some(("scenario", "Scenario", 2)),
        "created_by" => Some(("creator", "Creator", 3)),
        "music_by" => Some(("music", "Music", 4)),
        "other_by" => Some(("other", "Other", 5)),
        _ if class.ends_with("_by") => Some(("other", "Other", 6)),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct RawWorkCredits {
    #[serde(default)]
    tags: Vec<RawWorkCreditTag>,
}

#[derive(Debug, Deserialize)]
struct RawWorkCreditTag {
    #[serde(rename = "class")]
    class: String,
    #[serde(rename = "name")]
    name: String,
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn i64_to_bool(value: i64, field: &'static str) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StorageError::InvalidStoredValue {
            field,
            value: value.to_string(),
        }),
    }
}

fn ensure_changed(result: SqliteQueryResult, entity: &'static str, id: &str) -> Result<()> {
    if result.rows_affected() == 0 {
        Err(StorageError::NotFound {
            entity,
            id: id.to_owned(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn migrated_storage() -> Result<Storage> {
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;
        Ok(storage)
    }

    fn account(id: &str, label: &str) -> AccountUpsert {
        AccountUpsert {
            id: id.to_owned(),
            label: label.to_owned(),
            login_name: Some(format!("{id}@example.test")),
            credential_ref: Some(format!("account:{id}:password")),
            enabled: true,
        }
    }

    fn work(work_id: &str, title: &str, maker_name: &str, published_at: &str) -> CachedWork {
        work_with_age(work_id, title, maker_name, published_at, "all")
    }

    fn work_with_age(
        work_id: &str,
        title: &str,
        maker_name: &str,
        published_at: &str,
        age_category: &str,
    ) -> CachedWork {
        CachedWork {
            work_id: work_id.to_owned(),
            title: title.to_owned(),
            title_json: format!(r#"{{"ja_JP":"{title}"}}"#),
            maker_id: Some(format!("maker-{maker_name}")),
            maker_name: Some(maker_name.to_owned()),
            maker_json: Some(format!(r#"{{"ja_JP":"{maker_name}"}}"#)),
            work_type: Some("SOU".to_owned()),
            age_category: Some(age_category.to_owned()),
            thumbnail_url: Some(format!("https://img.example.test/{work_id}.jpg")),
            registered_at: Some(published_at.to_owned()),
            published_at: Some(published_at.to_owned()),
            updated_at: Some(published_at.to_owned()),
            raw_json: format!(r#"{{"workno":"{work_id}"}}"#),
            last_detail_sync_at: "2026-05-09T00:00:00.000Z".to_owned(),
        }
    }

    fn work_with_raw_json(
        work_id: &str,
        title: &str,
        maker_name: &str,
        published_at: &str,
        raw_json: &str,
    ) -> CachedWork {
        CachedWork {
            raw_json: raw_json.to_owned(),
            ..work(work_id, title, maker_name, published_at)
        }
    }

    fn account_work(work_id: &str, purchased_at: &str) -> AccountWork {
        AccountWork {
            work_id: work_id.to_owned(),
            purchased_at: Some(purchased_at.to_owned()),
        }
    }

    fn sync_commit(
        account_id: &str,
        sync_run_id: &str,
        works: Vec<CachedWork>,
        account_works: Vec<AccountWork>,
    ) -> AccountSyncCommit {
        AccountSyncCommit {
            sync_run_id: sync_run_id.to_owned(),
            account_id: account_id.to_owned(),
            started_at: "2026-05-09T00:00:00.000Z".to_owned(),
            completed_at: "2026-05-09T00:01:00.000Z".to_owned(),
            works,
            account_works,
        }
    }

    #[tokio::test]
    async fn runs_embedded_migrations_once() -> Result<()> {
        let storage = Storage::open_in_memory().await?;

        storage.run_migrations().await?;
        storage.run_migrations().await?;

        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&storage.pool)
            .await?;

        assert_eq!(migration_count, 3);

        Ok(())
    }

    #[tokio::test]
    async fn commits_write_transaction() -> Result<()> {
        let storage = migrated_storage().await?;

        let mut transaction = storage.begin_write().await?;
        transaction
            .execute("CREATE TABLE tx_test (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .await?;
        transaction
            .execute("INSERT INTO tx_test (value) VALUES ('committed')")
            .await?;
        transaction.commit().await?;

        let value: String = sqlx::query_scalar("SELECT value FROM tx_test WHERE id = 1")
            .fetch_one(&storage.pool)
            .await?;

        assert_eq!(value, "committed");

        Ok(())
    }

    #[tokio::test]
    async fn rolls_back_write_transaction() -> Result<()> {
        let storage = migrated_storage().await?;

        let mut transaction = storage.begin_write().await?;
        transaction
            .execute("CREATE TABLE tx_test (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .await?;
        transaction.commit().await?;

        let mut transaction = storage.begin_write().await?;
        transaction
            .execute("INSERT INTO tx_test (value) VALUES ('rolled back')")
            .await?;
        transaction.rollback().await?;

        let row = sqlx::query("SELECT COUNT(*) AS count FROM tx_test")
            .fetch_one(&storage.pool)
            .await?;
        let count: i64 = row.try_get("count")?;

        assert_eq!(count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn opens_file_database_and_creates_it_if_missing() -> Result<()> {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("storage.sqlite");

        let storage = Storage::open(&path).await?;
        storage.run_migrations().await?;
        drop(storage);

        assert!(path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn reads_empty_app_settings_by_default() -> Result<()> {
        let storage = migrated_storage().await?;

        assert_eq!(storage.app_settings().await?, AppSettings::default());

        Ok(())
    }

    #[tokio::test]
    async fn saves_app_settings_in_one_transaction() -> Result<()> {
        let storage = migrated_storage().await?;
        let settings = AppSettings {
            library_root: Some("/library".to_owned()),
            download_root: Some("/downloads".to_owned()),
        };

        storage.save_app_settings(&settings).await?;

        assert_eq!(storage.app_settings().await?, settings);

        Ok(())
    }

    #[tokio::test]
    async fn clears_missing_app_settings() -> Result<()> {
        let storage = migrated_storage().await?;

        storage
            .save_app_settings(&AppSettings {
                library_root: Some("/library".to_owned()),
                download_root: Some("/downloads".to_owned()),
            })
            .await?;
        storage
            .save_app_settings(&AppSettings {
                library_root: Some("/library".to_owned()),
                download_root: None,
            })
            .await?;

        assert_eq!(
            storage.app_settings().await?,
            AppSettings {
                library_root: Some("/library".to_owned()),
                download_root: None,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn creates_updates_and_disables_accounts() -> Result<()> {
        let storage = migrated_storage().await?;

        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .record_account_login("account-a", "2026-05-09T00:02:00.000Z")
            .await?;
        storage
            .save_account(&AccountUpsert {
                id: "account-a".to_owned(),
                label: "Renamed".to_owned(),
                login_name: Some("renamed@example.test".to_owned()),
                credential_ref: None,
                enabled: true,
            })
            .await?;
        storage.set_account_enabled("account-a", false).await?;

        let accounts = storage.accounts().await?;

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "account-a");
        assert_eq!(accounts[0].label, "Renamed");
        assert_eq!(
            accounts[0].login_name,
            Some("renamed@example.test".to_owned())
        );
        assert_eq!(accounts[0].credential_ref, None);
        assert!(!accounts[0].enabled);
        assert_eq!(
            accounts[0].last_login_at,
            Some("2026-05-09T00:02:00.000Z".to_owned())
        );

        Ok(())
    }

    #[tokio::test]
    async fn missing_account_updates_return_not_found() -> Result<()> {
        let storage = migrated_storage().await?;

        assert!(matches!(
            storage.set_account_enabled("missing", true).await,
            Err(StorageError::NotFound {
                entity: "account",
                id
            }) if id == "missing"
        ));

        Ok(())
    }

    #[tokio::test]
    async fn unified_product_list_collapses_duplicate_ownership() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .save_account(&account("account-b", "Account B"))
            .await?;

        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work(
                        "RJ000001",
                        "Shared Work",
                        "Maker One",
                        "2026-01-01T00:00:00Z",
                    ),
                    work(
                        "RJ000002",
                        "Account A Work",
                        "Maker Two",
                        "2026-01-02T00:00:00Z",
                    ),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                ],
            ))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-b",
                "sync-b-1",
                vec![work(
                    "RJ000001",
                    "Shared Work",
                    "Maker One",
                    "2026-01-01T00:00:00Z",
                )],
                vec![account_work("RJ000001", "2026-03-01T00:00:00Z")],
            ))
            .await?;

        let page = storage
            .list_products(&ProductListQuery {
                sort: ProductSort::TitleAsc,
                ..ProductListQuery::default()
            })
            .await?;

        assert_eq!(page.total_count, 2);
        assert_eq!(page.products.len(), 2);
        assert_eq!(page.products[1].work_id, "RJ000001");
        assert_eq!(page.products[1].owners.len(), 2);
        assert_eq!(page.products[1].owners[0].account_id, "account-a");
        assert_eq!(page.products[1].owners[1].account_id, "account-b");
        assert_eq!(
            page.products[1].earliest_purchased_at,
            Some("2026-02-01T00:00:00Z".to_owned())
        );
        assert_eq!(
            page.products[1].latest_purchased_at,
            Some("2026-03-01T00:00:00Z".to_owned())
        );

        let filtered_page = storage
            .list_products(&ProductListQuery {
                account_id: Some("account-b".to_owned()),
                ..ProductListQuery::default()
            })
            .await?;

        assert_eq!(filtered_page.total_count, 1);
        assert_eq!(filtered_page.products[0].work_id, "RJ000001");
        assert_eq!(filtered_page.products[0].owners.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn disabled_accounts_are_excluded_from_default_product_list() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .save_account(&account("account-b", "Account B"))
            .await?;

        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work(
                        "RJ000001",
                        "Shared Work",
                        "Maker One",
                        "2026-01-01T00:00:00Z",
                    ),
                    work("RJ000002", "Only A", "Maker Two", "2026-01-02T00:00:00Z"),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                ],
            ))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-b",
                "sync-b-1",
                vec![work(
                    "RJ000001",
                    "Shared Work",
                    "Maker One",
                    "2026-01-01T00:00:00Z",
                )],
                vec![account_work("RJ000001", "2026-03-01T00:00:00Z")],
            ))
            .await?;

        storage.set_account_enabled("account-a", false).await?;

        let page = storage.list_products(&ProductListQuery::default()).await?;

        assert_eq!(page.total_count, 1);
        assert_eq!(page.products[0].work_id, "RJ000001");
        assert_eq!(page.products[0].owners.len(), 1);
        assert_eq!(page.products[0].owners[0].account_id, "account-b");

        Ok(())
    }

    #[tokio::test]
    async fn product_list_hides_missing_detail_placeholders_without_deleting_cache() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;

        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work(
                        "RJ000001",
                        "Visible Work",
                        "Maker One",
                        "2026-01-01T00:00:00Z",
                    ),
                    work_with_raw_json(
                        "RJ000002",
                        "RJ000002",
                        "",
                        "2026-01-02T00:00:00Z",
                        r#"{
                            "workno": "RJ000002",
                            "source": "content/sales",
                            "detail_status": "missing_from_content_works",
                            "sales_date": "2026-01-02T00:00:00Z"
                        }"#,
                    ),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                ],
            ))
            .await?;

        let page = storage.list_products(&ProductListQuery::default()).await?;
        let search_page = storage
            .list_products(&ProductListQuery {
                search: Some("RJ000002".to_owned()),
                ..ProductListQuery::default()
            })
            .await?;
        let cached_placeholder_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM works WHERE work_id = 'RJ000002'")
                .fetch_one(&storage.pool)
                .await?;
        let current_ownership_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM account_works WHERE work_id = 'RJ000002' AND is_current = 1",
        )
        .fetch_one(&storage.pool)
        .await?;

        assert_eq!(page.total_count, 1);
        assert_eq!(page.products.len(), 1);
        assert_eq!(page.products[0].work_id, "RJ000001");
        assert_eq!(search_page.total_count, 0);
        assert_eq!(search_page.products.len(), 0);
        assert_eq!(cached_placeholder_count, 1);
        assert_eq!(current_ownership_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn product_list_groups_source_credits_from_raw_json() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![work_with_raw_json(
                    "RJ000001",
                    "Credit Work",
                    "Maker One",
                    "2026-01-01T00:00:00Z",
                    r#"{
                        "workno": "RJ000001",
                        "tags": [
                            { "class": "genre", "name": "ASMR" },
                            { "class": "voice_by", "name": "Voice One" },
                            { "class": "voice_by", "name": "Voice One" },
                            { "class": "voice_by", "name": "Voice Two" },
                            { "class": "illust_by", "name": "Illust One" },
                            { "class": "scenario_by", "name": "Scenario One" },
                            { "class": "created_by", "name": "Creator One" },
                            { "class": "music_by", "name": "Music One" },
                            { "class": "other_by", "name": "Other One" },
                            { "class": "unknown_by", "name": "Unknown Credit" }
                        ]
                    }"#,
                )],
                vec![account_work("RJ000001", "2026-02-01T00:00:00Z")],
            ))
            .await?;

        let page = storage.list_products(&ProductListQuery::default()).await?;

        assert_eq!(page.products[0].credit_groups.len(), 6);
        assert_eq!(page.products[0].credit_groups[0].kind, "voice");
        assert_eq!(page.products[0].credit_groups[0].label, "CV");
        assert_eq!(
            page.products[0].credit_groups[0].names,
            vec!["Voice One".to_owned(), "Voice Two".to_owned()]
        );
        assert_eq!(page.products[0].credit_groups[1].kind, "illust");
        assert_eq!(page.products[0].credit_groups[2].kind, "scenario");
        assert_eq!(page.products[0].credit_groups[3].kind, "creator");
        assert_eq!(page.products[0].credit_groups[4].kind, "music");
        assert_eq!(page.products[0].credit_groups[5].kind, "other");
        assert_eq!(
            page.products[0].credit_groups[5].names,
            vec!["Other One".to_owned(), "Unknown Credit".to_owned()]
        );

        Ok(())
    }

    #[tokio::test]
    async fn complete_sync_marks_missing_ownership_stale_without_deleting_history() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;

        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work("RJ000001", "Old Work", "Maker One", "2026-01-01T00:00:00Z"),
                    work("RJ000002", "Kept Work", "Maker Two", "2026-01-02T00:00:00Z"),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                ],
            ))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-2",
                vec![work(
                    "RJ000002",
                    "Kept Work",
                    "Maker Two",
                    "2026-01-02T00:00:00Z",
                )],
                vec![account_work("RJ000002", "2026-02-02T00:00:00Z")],
            ))
            .await?;

        let page = storage.list_products(&ProductListQuery::default()).await?;
        let stale_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM account_works WHERE is_current = 0")
                .fetch_one(&storage.pool)
                .await?;
        let sync_runs = storage.sync_runs_for_account("account-a").await?;
        let accounts = storage.accounts().await?;

        assert_eq!(page.total_count, 1);
        assert_eq!(page.products[0].work_id, "RJ000002");
        assert_eq!(stale_count, 1);
        assert_eq!(sync_runs.len(), 2);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Completed);
        assert_eq!(
            accounts[0].last_sync_at,
            Some("2026-05-09T00:01:00.000Z".to_owned())
        );

        Ok(())
    }

    #[tokio::test]
    async fn sync_failure_is_recorded_without_marking_existing_cache_stale() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;

        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![work(
                    "RJ000001",
                    "Existing Work",
                    "Maker One",
                    "2026-01-01T00:00:00Z",
                )],
                vec![account_work("RJ000001", "2026-02-01T00:00:00Z")],
            ))
            .await?;
        storage
            .record_sync_failure(&SyncFailure {
                sync_run_id: "sync-a-failed".to_owned(),
                account_id: "account-a".to_owned(),
                started_at: "2026-05-09T00:02:00.000Z".to_owned(),
                completed_at: "2026-05-09T00:03:00.000Z".to_owned(),
                error_code: Some("network".to_owned()),
                error_message: Some("network unavailable".to_owned()),
            })
            .await?;

        let page = storage.list_products(&ProductListQuery::default()).await?;
        let sync_runs = storage.sync_runs_for_account("account-a").await?;

        assert_eq!(page.total_count, 1);
        assert_eq!(sync_runs.len(), 2);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Failed);
        assert_eq!(sync_runs[0].error_code, Some("network".to_owned()));

        Ok(())
    }

    #[tokio::test]
    async fn sync_cancellation_is_recorded_with_cancelled_status() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;

        storage
            .record_sync_cancellation(&SyncCancellation {
                sync_run_id: "sync-a-cancelled".to_owned(),
                account_id: "account-a".to_owned(),
                started_at: "2026-05-09T00:02:00.000Z".to_owned(),
                completed_at: "2026-05-09T00:03:00.000Z".to_owned(),
            })
            .await?;

        let sync_runs = storage.sync_runs_for_account("account-a").await?;

        assert_eq!(sync_runs.len(), 1);
        assert_eq!(sync_runs[0].status, SyncRunStatus::Cancelled);
        assert_eq!(sync_runs[0].error_code, Some("cancelled".to_owned()));

        Ok(())
    }

    #[tokio::test]
    async fn failed_sync_commit_rolls_back_partial_writes() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;

        let result = storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                Vec::new(),
                vec![account_work("RJ000001", "2026-02-01T00:00:00Z")],
            ))
            .await;

        assert!(result.is_err());
        assert_eq!(
            storage.sync_runs_for_account("account-a").await?,
            Vec::new()
        );
        assert_eq!(
            storage
                .list_products(&ProductListQuery::default())
                .await?
                .total_count,
            0
        );

        Ok(())
    }

    #[tokio::test]
    async fn explicit_write_transaction_can_roll_back_library_changes() -> Result<()> {
        let storage = migrated_storage().await?;
        let mut transaction = storage.begin_write().await?;

        transaction
            .upsert_account(&account("account-a", "Account A"))
            .await?;
        transaction
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![work(
                    "RJ000001",
                    "Rolled Back Work",
                    "Maker One",
                    "2026-01-01T00:00:00Z",
                )],
                vec![account_work("RJ000001", "2026-02-01T00:00:00Z")],
            ))
            .await?;
        transaction.rollback().await?;

        assert_eq!(storage.accounts().await?, Vec::new());
        assert_eq!(
            storage
                .list_products(&ProductListQuery::default())
                .await?
                .total_count,
            0
        );

        Ok(())
    }

    #[tokio::test]
    async fn product_list_search_and_sort_use_cached_columns() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work("RJ000001", "Alpha", "Circle One", "2026-01-01T00:00:00Z"),
                    work("RJ000002", "Beta", "Special Maker", "2026-03-01T00:00:00Z"),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                ],
            ))
            .await?;

        let search_page = storage
            .list_products(&ProductListQuery {
                search: Some("Special".to_owned()),
                ..ProductListQuery::default()
            })
            .await?;
        let sorted_page = storage
            .list_products(&ProductListQuery {
                sort: ProductSort::PublishedAtDesc,
                ..ProductListQuery::default()
            })
            .await?;

        assert_eq!(search_page.total_count, 1);
        assert_eq!(search_page.products[0].work_id, "RJ000002");
        assert_eq!(sorted_page.products[0].work_id, "RJ000002");

        Ok(())
    }

    #[tokio::test]
    async fn product_list_can_filter_by_age_category() -> Result<()> {
        let storage = migrated_storage().await?;
        storage
            .save_account(&account("account-a", "Account A"))
            .await?;
        storage
            .commit_account_sync(&sync_commit(
                "account-a",
                "sync-a-1",
                vec![
                    work_with_age(
                        "RJ000001",
                        "All Ages",
                        "Circle One",
                        "2026-01-01T00:00:00Z",
                        "all",
                    ),
                    work_with_age(
                        "RJ000002",
                        "Fifteen",
                        "Circle Two",
                        "2026-01-02T00:00:00Z",
                        "r15",
                    ),
                    work_with_age(
                        "RJ000003",
                        "Eighteen",
                        "Circle Three",
                        "2026-01-03T00:00:00Z",
                        "r18",
                    ),
                ],
                vec![
                    account_work("RJ000001", "2026-02-01T00:00:00Z"),
                    account_work("RJ000002", "2026-02-02T00:00:00Z"),
                    account_work("RJ000003", "2026-02-03T00:00:00Z"),
                ],
            ))
            .await?;

        let all_ages_page = storage
            .list_products(&ProductListQuery {
                age_category: Some(ProductAgeCategory::All),
                ..ProductListQuery::default()
            })
            .await?;
        let r15_page = storage
            .list_products(&ProductListQuery {
                age_category: Some(ProductAgeCategory::R15),
                ..ProductListQuery::default()
            })
            .await?;
        let r18_page = storage
            .list_products(&ProductListQuery {
                age_category: Some(ProductAgeCategory::R18),
                ..ProductListQuery::default()
            })
            .await?;

        assert_eq!(all_ages_page.total_count, 1);
        assert_eq!(all_ages_page.products[0].work_id, "RJ000001");
        assert_eq!(r15_page.total_count, 1);
        assert_eq!(r15_page.products[0].work_id, "RJ000002");
        assert_eq!(r18_page.total_count, 1);
        assert_eq!(r18_page.products[0].work_id, "RJ000003");

        Ok(())
    }
}
