use super::DBResult;
use crate::{
    application::use_application,
    database::{
        models::v2::{CreatingProductDownload, ProductDownload},
        tables::Table,
    },
};
use rusqlite::types::Value;
use serde_rusqlite::*;
use std::rc::Rc;

pub struct ProductDownloadTable;

impl Table for ProductDownloadTable {
    fn get_ddl() -> &'static str {
        r#"
CREATE TABLE IF NOT EXISTS v2_product_downloads (
    product_id TEXT NOT NULL PRIMARY KEY,
    path TEXT NOT NULL,

    FOREIGN KEY(product_id) REFERENCES v2_products(id) ON UPDATE CASCADE ON DELETE CASCADE
);
"#
    }
}

impl ProductDownloadTable {
    /// Inserts a single product download into the database.
    pub fn insert_one(download: CreatingProductDownload) -> DBResult<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
INSERT INTO v2_product_downloads (
    product_id,
    path
) VALUES (
    :product_id,
    :path
)
"#,
        )?;

        stmt.execute(to_params_named(download)?.to_slice().as_slice())?;
        Ok(())
    }

    /// Retrieves many product downloads from the database.
    pub fn get_many(product_ids: impl Iterator<Item = String>) -> DBResult<Vec<ProductDownload>> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
SELECT
    product_id,
    path
FROM v2_product_downloads WHERE product_id IN rarray(?)
"#,
        )?;

        let product_ids = Rc::new(product_ids.map(Value::from).collect::<Vec<_>>());
        let columns = columns_from_statement(&stmt);
        let product_downloads = stmt
            .query_and_then([product_ids], |row| {
                from_row_with_columns::<ProductDownload>(row, &columns)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(product_downloads)
    }

    /// Retrieves a single product download from the database.
    pub fn get_one(product_id: &str) -> DBResult<Option<ProductDownload>> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
SELECT
    product_id,
    path
FROM v2_product_downloads
WHERE product_id = :product_id
"#,
        )?;

        let mut rows = stmt.query_and_then(&[(":product_id", &product_id)], |row| {
            from_row::<ProductDownload>(row)
        })?;
        let product_download = rows.next().transpose()?;
        Ok(product_download)
    }

    /// Removes a single product download from the database.
    pub fn remove_one(product_id: &str) -> DBResult<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
DELETE FROM v2_product_downloads
WHERE product_id = :product_id
"#,
        )?;

        stmt.execute(&[(":product_id", &product_id)])?;
        Ok(())
    }

    /// Removes many product downloads from the database.
    pub fn remove_many() -> DBResult<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
DELETE FROM v2_product_downloads
"#,
        )?;

        stmt.execute([])?;
        Ok(())
    }
}
