use crate::{
    application::use_application,
    application_error::Result,
    database::{
        models::v2::{Account, CreatingAccount, UpdatingAccount},
        tables::Table,
    },
};
use rusqlite::{named_params, OptionalExtension};
use serde_rusqlite::*;

pub struct AccountTable;

impl Table for AccountTable {
    fn get_ddl() -> &'static str {
        r#"
CREATE TABLE IF NOT EXISTS v2_accounts (
    id INTEGER PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    memo TEXT,
    product_count INTEGER NOT NULL DEFAULT 0,
    cookie_json TEXT NOT NULL DEFAULT '{}'
);
"#
    }
}

impl AccountTable {
    /// Inserts a single account into the database.
    /// Returns the ID of the inserted account.
    pub fn insert_one(account: CreatingAccount) -> Result<i64> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
INSERT INTO v2_accounts (
    username,
    password,
    memo
) VALUES (
    :username,
    :password,
    :memo
)
"#,
        )?;

        let id = stmt.insert(to_params_named(&account)?.to_slice().as_slice())?;
        Ok(id)
    }

    /// Retrieves all accounts from the database.
    pub fn get_all() -> Result<Vec<Account>> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
SELECT
    id,
    username,
    password,
    memo,
    product_count,
    cookie_json
FROM v2_accounts
ORDER BY id ASC
"#,
        )?;

        let columns = columns_from_statement(&stmt);
        let accounts = stmt
            .query_and_then([], |row| from_row_with_columns::<Account>(row, &columns))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(accounts)
    }

    /// Retrieves a single account from the database.
    pub fn get_one(id: i64) -> Result<Option<Account>> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
SELECT
    username,
    password,
    memo
FROM v2_accounts
WHERE id = :id
"#,
        )?;

        let account = stmt
            .query_row(
                named_params! {
                    ":id": id,
                },
                |row| Ok(from_row::<Account>(row)),
            )
            .optional()?
            .transpose()?;
        Ok(account)
    }

    /// Updates a single account in the database.
    pub fn update_one(account: UpdatingAccount) -> Result<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
UPDATE v2_accounts
SET
    username = :username,
    password = :password,
    memo = :memo
WHERE id = :id
"#,
        )?;

        stmt.execute(named_params! {
            ":id": account.id,
            ":username": account.username,
            ":password": account.password,
            ":memo": account.memo
        })?;
        Ok(())
    }

    /// Updates a single account's product count in the database.
    pub fn update_one_product_count(id: i64, product_count: i32) -> Result<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
UPDATE v2_accounts
SET
    product_count = :product_count
WHERE id = :id
"#,
        )?;

        stmt.execute(named_params! {
            ":id": id,
            ":product_count": product_count
        })?;
        Ok(())
    }

    /// Updates a single account's cookie JSON in the database.
    pub fn update_one_cookie_json(id: i64, cookie_json: &str) -> Result<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
UPDATE v2_accounts
SET
    cookie_json = :cookie_json
WHERE id = :id
"#,
        )?;

        stmt.execute(named_params! {
            ":id": id,
            ":cookie_json": cookie_json
        })?;
        Ok(())
    }

    /// Removes a single account from the database.
    pub fn remove_one(id: i64) -> Result<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
DELETE FROM v2_accounts
WHERE id = :id
"#,
        )?;

        stmt.execute(&[(":id", &id)])?;
        Ok(())
    }
}
