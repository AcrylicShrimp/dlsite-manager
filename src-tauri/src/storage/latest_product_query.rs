use super::product::{ProductDownloadState, ProductQuery};
use crate::{application::use_application, application_error::Result};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LatestProductQuery {
    pub query: ProductQuery,
    pub download: Option<ProductDownloadState>,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for LatestProductQuery {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            query: ProductQuery {
                query: row.get::<_, Option<String>>("query")?,
                ty: row
                    .get_ref("ty")?
                    .as_str_or_null()?
                    .map(|ty| {
                        <_>::from_str(ty).map_err(|err: strum::ParseError| {
                            rusqlite::Error::FromSqlConversionFailure(
                                row.as_ref().column_index("ty").unwrap(),
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })
                    })
                    .transpose()?,
                age: row
                    .get_ref("age")?
                    .as_str_or_null()?
                    .map(|age| {
                        <_>::from_str(age).map_err(|err: strum::ParseError| {
                            rusqlite::Error::FromSqlConversionFailure(
                                row.as_ref().column_index("age").unwrap(),
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })
                    })
                    .transpose()?,
                order_by: <_>::from_str(row.get_ref("order_by")?.as_str()?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("order_by").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
            },
            download: row
                .get_ref("download")?
                .as_str_or_null()?
                .map(|download| {
                    <_>::from_str(download).map_err(|err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("download").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })
                })
                .transpose()?,
        })
    }
}

impl LatestProductQuery {
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

    pub fn get() -> Result<Self> {
        Ok(use_application()
            .storage()
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
            .query_row((), |row| Self::try_from(row))
            .optional()?
            .unwrap_or_default())
    }

    pub fn set(query: Self) -> Result<()> {
        let connection = use_application().storage().connection();
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
