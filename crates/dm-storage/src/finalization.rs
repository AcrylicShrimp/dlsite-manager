use super::*;

/// Bounded recovery intent, not durable generic job history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFinalization {
    pub work_id: String,
    pub operation_id: String,
    pub staging_path: String,
    pub final_path: String,
    pub old_path: Option<String>,
    pub temporary_path: String,
    pub committed: bool,
}

impl Storage {
    pub async fn download_finalization(
        &self,
        work_id: &str,
    ) -> Result<Option<DownloadFinalization>> {
        let row = sqlx::query("SELECT * FROM download_finalizations WHERE work_id = ?1")
            .bind(work_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| {
            Ok(DownloadFinalization {
                work_id: row.try_get("work_id")?,
                operation_id: row.try_get("operation_id")?,
                staging_path: row.try_get("staging_path")?,
                final_path: row.try_get("final_path")?,
                old_path: row.try_get("old_path")?,
                temporary_path: row.try_get("temporary_path")?,
                committed: row.try_get("committed")?,
            })
        })
        .transpose()
    }

    pub async fn begin_download_finalization(&self, record: &DownloadFinalization) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let connection = tx
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;
        sqlx::query("INSERT INTO download_finalizations (work_id, operation_id, staging_path, final_path, old_path, temporary_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
            .bind(&record.work_id).bind(&record.operation_id).bind(&record.staging_path)
            .bind(&record.final_path).bind(&record.old_path).bind(&record.temporary_path)
            .execute(&mut **connection).await?;
        tx.commit().await
    }

    pub async fn commit_download_finalization(
        &self,
        operation_id: &str,
        update: &WorkDownloadUpdate,
    ) -> Result<()> {
        let mut tx = self.begin_write().await?;
        tx.save_work_download(update).await?;
        let connection = tx
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;
        let result = sqlx::query("UPDATE download_finalizations SET committed = 1 WHERE work_id = ?1 AND operation_id = ?2 AND committed = 0")
            .bind(&update.work_id).bind(operation_id).execute(&mut **connection).await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::NotFound {
                entity: "download finalization",
                id: operation_id.to_owned(),
            });
        }
        tx.commit().await
    }

    pub async fn clear_download_finalization(
        &self,
        work_id: &str,
        operation_id: &str,
    ) -> Result<()> {
        let mut tx = self.begin_write().await?;
        let connection = tx
            .transaction
            .as_mut()
            .ok_or(StorageError::TransactionFinished)?;
        sqlx::query("DELETE FROM download_finalizations WHERE work_id = ?1 AND operation_id = ?2")
            .bind(work_id)
            .bind(operation_id)
            .execute(&mut **connection)
            .await?;
        tx.commit().await
    }
}
