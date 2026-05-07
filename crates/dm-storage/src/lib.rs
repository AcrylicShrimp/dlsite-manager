use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult},
    Sqlite, SqlitePool, Transaction,
};
use std::path::Path;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn runs_embedded_migrations_once() -> Result<()> {
        let storage = Storage::open_in_memory().await?;

        storage.run_migrations().await?;
        storage.run_migrations().await?;

        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&storage.pool)
            .await?;

        assert_eq!(migration_count, 1);

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
}
