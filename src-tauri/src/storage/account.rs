use crate::{
    application::use_application,
    application_error::{Error, Result},
};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
    pub product_count: i32,
    pub cookie_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAccount {
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedAccount {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub memo: Option<String>,
}

impl<'stmt> TryFrom<&'stmt Row<'stmt>> for Account {
    type Error = rusqlite::Error;

    fn try_from(row: &'stmt Row<'stmt>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            id: row.get("id")?,
            username: row.get("username")?,
            password: row.get("password")?,
            memo: row.get("memo")?,
            product_count: row.get("product_count")?,
            cookie_json: row.get("cookie_json")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl Account {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    memo TEXT,
    product_count INTEGER NOT NULL DEFAULT 0,
    cookie_json STRING NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER IF NOT EXISTS accounts_updated_at AFTER UPDATE ON accounts
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE accounts SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
        "
    }

    pub fn list_all() -> Result<Vec<Self>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    id,
    username,
    password,
    memo,
    product_count,
    cookie_json,
    created_at,
    updated_at
FROM accounts
ORDER BY id ASC
        ",
            )?
            .query_map((), |row| Self::try_from(row))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn list_all_id() -> Result<Vec<i64>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    id
FROM accounts
ORDER BY id ASC
        ",
            )?
            .query_map((), |row| row.get("id"))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_one(id: i64) -> Result<Option<Account>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    id,
    username,
    password,
    memo,
    product_count,
    cookie_json,
    created_at,
    updated_at
FROM accounts
WHERE id = ?1
        ",
            )?
            .query_row(params![id], |row| Self::try_from(row))
            .optional()?)
    }

    pub fn get_one_username_and_password(id: i64) -> Result<Option<(String, String)>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    username,
    password
FROM accounts
WHERE id = ?1
        ",
            )?
            .query_row(params![id], |row| {
                Ok((row.get("username")?, row.get("password")?))
            })
            .optional()?)
    }

    pub fn get_one_product_count(id: i64) -> Result<Option<i32>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    product_count
FROM accounts
WHERE id = ?1
        ",
            )?
            .query_row(params![id], |row| Ok(row.get("product_count")?))
            .optional()?)
    }

    pub fn get_one_cookie_json(id: i64) -> Result<Option<String>> {
        Ok(use_application()
            .storage()
            .connection()
            .prepare(
                "
SELECT
    cookie_json
FROM accounts
WHERE id = ?1
        ",
            )?
            .query_row(params![id], |row| Ok(row.get("cookie_json")?))
            .optional()?)
    }

    pub fn create_one(account: CreatedAccount) -> Result<Account> {
        let id = use_application()
            .storage()
            .connection()
            .prepare(
                "
INSERT INTO accounts (
    username,
    password,
    memo
) VALUES (
    ?1,
    ?2,
    ?3
)
            ",
            )?
            .insert(params![account.username, account.password, account.memo])?;

        if let Some(account) = Self::get_one(id)? {
            Ok(account)
        } else {
            Err(Error::DatabaseCreatedItemNotAccessible)
        }
    }

    pub fn update_one(account: UpdatedAccount) -> Result<Account> {
        use_application()
            .storage()
            .connection()
            .prepare(
                "
UPDATE accounts
SET
    username = ?2,
    password = ?3,
    memo = ?4
WHERE id = ?1
        ",
            )?
            .execute(params![
                account.id,
                account.username,
                account.password,
                account.memo
            ])?;

        if let Some(account) = Self::get_one(account.id)? {
            Ok(account)
        } else {
            Err(Error::DatabaseUpdatedItemNotAccessible)
        }
    }

    pub fn update_one_product_count(id: i64, product_count: i32) -> Result<()> {
        use_application()
            .storage()
            .connection()
            .prepare(
                "
UPDATE accounts
SET
    product_count = ?2
WHERE id = ?1
        ",
            )?
            .execute(params![id, product_count])?;
        Ok(())
    }

    pub fn update_one_cookie_json(id: i64, cookie_json: impl AsRef<str>) -> Result<()> {
        use_application()
            .storage()
            .connection()
            .prepare(
                "
UPDATE accounts
SET
    cookie_json = ?2
WHERE id = ?1
        ",
            )?
            .execute(params![id, cookie_json.as_ref()])?;
        Ok(())
    }

    pub fn remove_one(id: i64) -> Result<()> {
        use_application()
            .storage()
            .connection()
            .prepare(
                "
DELETE
FROM accounts
WHERE id = ?1
        ",
            )?
            .execute(params![id])?;
        Ok(())
    }
}
