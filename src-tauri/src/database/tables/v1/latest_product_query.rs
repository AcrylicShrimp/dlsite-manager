use crate::{
    application::use_application, application_error::Result,
    database::models::v1::LatestProductQuery,
};
use rusqlite::{params, OptionalExtension};

pub struct LatestProductQueryTable;

impl LatestProductQueryTable {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS latest_product_query (
    query TEXT,
    ty TEXT,
    age TEXT,
    order_by TEXT NOT NULL,
    download TEXT
);"
    }

    pub fn get() -> Result<LatestProductQuery> {
        Ok(use_application()
            .connection()
            .prepare(
                "
SELECT
    query,
    ty,
    age,
    order_by,
    download
FROM latest_product_query;",
            )?
            .query_row((), |row| LatestProductQuery::try_from(row))
            .optional()?
            .unwrap_or_default())
    }

    pub fn set(query: LatestProductQuery) -> Result<()> {
        let connection = use_application().connection();
        connection.execute(
            "
DELETE FROM latest_product_query",
            (),
        )?;
        connection
            .prepare(
                "
INSERT INTO latest_product_query (
    query,
    ty,
    age,
    order_by,
    download
) VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5
)",
            )?
            .insert(params![
                &query.query.query,
                query.query.ty.as_ref().map(|ty| ty.to_string()),
                query.query.age.as_ref().map(|age| age.to_string()),
                <_ as Into<&'static str>>::into(&query.query.order_by),
                query
                    .download
                    .as_ref()
                    .map(|download| <_ as Into<&'static str>>::into(download)),
            ])?;

        Ok(())
    }
}
