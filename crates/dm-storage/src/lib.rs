use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult},
    Row, Sqlite, SqlitePool, Transaction,
};
use std::path::Path;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
const LIBRARY_ROOT_KEY: &str = "library_root";
const DOWNLOAD_ROOT_KEY: &str = "download_root";

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("database migration error")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("write transaction is already finished")]
    TransactionFinished,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppSettings {
    pub library_root: Option<String>,
    pub download_root: Option<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn runs_embedded_migrations_once() -> Result<()> {
        let storage = Storage::open_in_memory().await?;

        storage.run_migrations().await?;
        storage.run_migrations().await?;

        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&storage.pool)
            .await?;

        assert_eq!(migration_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn commits_write_transaction() -> Result<()> {
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;

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
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;

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
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;

        assert_eq!(storage.app_settings().await?, AppSettings::default());

        Ok(())
    }

    #[tokio::test]
    async fn saves_app_settings_in_one_transaction() -> Result<()> {
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;
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
        let storage = Storage::open_in_memory().await?;
        storage.run_migrations().await?;

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
}
