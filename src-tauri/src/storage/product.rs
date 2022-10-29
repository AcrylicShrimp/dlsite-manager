use super::account::Account;
use crate::{
    application::use_application,
    application_error::Result,
    dlsite::api::{
        DLsiteProduct, DLsiteProductAgeCategory, DLsiteProductGroup, DLsiteProductIcon,
        DLsiteProductLocalizedString, DLsiteProductType,
    },
};
use rusqlite::{params, params_from_iter, Row};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: i64,
    pub account: Account,
    pub product: DLsiteProduct,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct ProductQuery {
    pub query: Option<String>,
    pub ty: Option<DLsiteProductType>,
    pub age: Option<DLsiteProductAgeCategory>,
}

#[derive(Debug, Clone)]
pub struct InsertedProduct {
    pub account_id: i64,
    pub product: DLsiteProduct,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for Product {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            account: Account {
                id: row.get("account_id")?,
                username: row.get("account_username")?,
                password: row.get("account_password")?,
                memo: row.get("account_memo")?,
                product_count: row.get("account_product_count")?,
                cookie_json: row.get("account_cookie_json")?,
                created_at: row.get("account_created_at")?,
                updated_at: row.get("account_updated_at")?,
            },
            product: DLsiteProduct {
                id: row.get("product_id")?,
                ty: <_>::from_str(&row.get::<_, String>("product_type")?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("product_type").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
                age: <_>::from_str(&row.get::<_, String>("product_age")?).map_err(
                    |err: strum::ParseError| {
                        rusqlite::Error::FromSqlConversionFailure(
                            row.as_ref().column_index("product_type").unwrap(),
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    },
                )?,
                title: DLsiteProductLocalizedString {
                    japanese: row.get("product_title_ja")?,
                    english: row.get("product_title_en")?,
                    korean: row.get("product_title_ko")?,
                    taiwanese: row.get("product_title_tw")?,
                    chinese: row.get("product_title_cn")?,
                },
                group: DLsiteProductGroup {
                    id: row.get("product_group_id")?,
                    name: DLsiteProductLocalizedString {
                        japanese: row.get("product_group_name_ja")?,
                        english: row.get("product_group_name_en")?,
                        korean: row.get("product_group_name_ko")?,
                        taiwanese: row.get("product_group_name_tw")?,
                        chinese: row.get("product_group_name_cn")?,
                    },
                },
                icon: DLsiteProductIcon {
                    main: row.get("product_icon_main")?,
                    small: row.get("product_icon_small")?,
                },
                registered_at: row.get("registered_at")?,
                upgraded_at: row.get("upgraded_at")?,
                purchased_at: row.get("purchased_at")?,
            },
        })
    }
}

impl Product {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY NOT NULL,
    account_id INTEGER NOT NULL,
    product_id TEXT NOT NULL UNIQUE,
    product_type TEXT NOT NULL,
    product_age TEXT NOT NULL,
    product_title_ja TEXT,
    product_title_en TEXT,
    product_title_ko TEXT,
    product_title_tw TEXT,
    product_title_cn TEXT,
    product_group_id TEXT NOT NULL,
    product_group_name_ja TEXT,
    product_group_name_en TEXT,
    product_group_name_ko TEXT,
    product_group_name_tw TEXT,
    product_group_name_cn TEXT,
    product_icon_main TEXT NOT NULL,
    product_icon_small TEXT NOT NULL,
    registered_at INTEGER,
    upgraded_at INTEGER,
    purchased_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(account_id) REFERENCES accounts(id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TRIGGER IF NOT EXISTS products_updated_at AFTER UPDATE ON products
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE products SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE VIRTUAL TABLE IF NOT EXISTS indexed_products USING fts5 (
    product_id,
    product_title_ja,
    product_title_en,
    product_title_ko,
    product_title_tw,
    product_title_cn,
    product_group_id,
    product_group_name_ja,
    product_group_name_en,
    product_group_name_ko,
    product_group_name_tw,
    product_group_name_cn,
    tokenize = 'trigram'
);"
    }

    pub fn list_all(query: &ProductQuery) -> Result<Vec<Self>> {
        let mut where_clause = "TRUE".to_owned();
        let mut params = Vec::new();

        if let Some(query) = &query.query {
            let query = query.trim();
            if query.len() != 0 {
                where_clause.push_str(" AND indexed_products MATCH ?");
                params.push(query);
            }
        }

        if let Some(ty) = query.ty {
            where_clause.push_str(" AND product.product_type = ?");
            params.push(<_ as Into<&'static str>>::into(ty));
        }

        if let Some(age) = query.age {
            where_clause.push_str(" AND product.product_age = ?");
            params.push(<_ as Into<&'static str>>::into(age));
        }

        Ok(use_application()
            .storage()
            .connection()
            .prepare(&format!(
                "
SELECT
    account.username AS account_username,
    account.password AS account_password,
    account.memo AS account_memo,
    account.product_count AS account_product_count,
    account.cookie_json AS account_cookie_json,
    account.created_at AS account_created_at,
    account.updated_at AS account_updated_at,
    product.id,
    product.account_id,
    product.product_id,
    product.product_type,
    product.product_age,
    product.product_title_ja,
    product.product_title_en,
    product.product_title_ko,
    product.product_title_tw,
    product.product_title_cn,
    product.product_group_id,
    product.product_group_name_ja,
    product.product_group_name_en,
    product.product_group_name_ko,
    product.product_group_name_tw,
    product.product_group_name_cn,
    product.product_icon_main,
    product.product_icon_small,
    product.registered_at,
    product.upgraded_at,
    product.purchased_at,
    product.created_at,
    product.updated_at
FROM indexed_products
INNER JOIN products AS product ON product.product_id = indexed_products.product_id
INNER JOIN accounts AS account ON account.id = product.account_id
WHERE {}
GROUP BY product.product_id
ORDER BY product.purchased_at DESC, product.id ASC",
                where_clause
            ))?
            .query_map(params_from_iter(&params), |row| Self::try_from(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn insert_all(mut products: impl Iterator<Item = InsertedProduct>) -> Result<()> {
        let mut connection = use_application().storage().connection();
        let tx = connection.transaction()?;
        {
            let mut insert_stmt = tx.prepare(
                "
INSERT INTO products (
    account_id,
    product_id,
    product_type,
    product_age,
    product_title_ja,
    product_title_en,
    product_title_ko,
    product_title_tw,
    product_title_cn,
    product_group_id,
    product_group_name_ja,
    product_group_name_en,
    product_group_name_ko,
    product_group_name_tw,
    product_group_name_cn,
    product_icon_main,
    product_icon_small,
    registered_at,
    upgraded_at,
    purchased_at
) VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8,
    ?9,
    ?10,
    ?11,
    ?12,
    ?13,
    ?14,
    ?15,
    ?16,
    ?17,
    ?18,
    ?19,
    ?20
) ON CONFLICT (product_id) DO NOTHING",
            )?;
            let mut index_stmt = tx.prepare(
                "
INSERT INTO indexed_products (
    product_id,
    product_title_ja,
    product_title_en,
    product_title_ko,
    product_title_tw,
    product_title_cn,
    product_group_id,
    product_group_name_ja,
    product_group_name_en,
    product_group_name_ko,
    product_group_name_tw,
    product_group_name_cn
) VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8,
    ?9,
    ?10,
    ?11,
    ?12
)",
            )?;

            while let Some(product) = products.next() {
                insert_stmt.execute(params![
                    product.account_id,
                    &product.product.id,
                    <_ as Into<&'static str>>::into(product.product.ty),
                    <_ as Into<&'static str>>::into(product.product.age),
                    &product.product.title.japanese,
                    &product.product.title.english,
                    &product.product.title.korean,
                    &product.product.title.taiwanese,
                    &product.product.title.chinese,
                    &product.product.group.id,
                    &product.product.group.name.japanese,
                    &product.product.group.name.english,
                    &product.product.group.name.korean,
                    &product.product.group.name.taiwanese,
                    &product.product.group.name.chinese,
                    product.product.icon.main,
                    product.product.icon.small,
                    product.product.registered_at,
                    product.product.upgraded_at,
                    product.product.purchased_at,
                ])?;
                index_stmt.execute(params![
                    &product.product.id,
                    &product.product.title.japanese,
                    &product.product.title.english,
                    &product.product.title.korean,
                    &product.product.title.taiwanese,
                    &product.product.title.chinese,
                    &product.product.group.id,
                    &product.product.group.name.japanese,
                    &product.product.group.name.english,
                    &product.product.group.name.korean,
                    &product.product.group.name.taiwanese,
                    &product.product.group.name.chinese,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn remove_all() -> Result<()> {
        use_application().storage().connection().execute_batch(
            "
DELETE FROM products;
DELETE FROM indexed_products;
VACUUM;",
        )?;
        Ok(())
    }
}
