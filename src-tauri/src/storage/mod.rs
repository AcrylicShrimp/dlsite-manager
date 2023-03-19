use self::{
    account::Account, display_language_setting::DisplayLanguageSetting,
    latest_product_query::LatestProductQuery, product::Product, setting::Setting,
};
use crate::application_error::Result;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::Connection;
use std::path::Path;

pub mod account;
pub mod display_language_setting;
pub mod latest_product_query;
pub mod product;
pub mod setting;

pub struct Storage {
    connection: Mutex<Connection>,
}

impl Storage {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            connection: Connection::open(path)?.into(),
        })
    }

    pub fn connection(&self) -> MutexGuard<Connection> {
        self.connection.lock()
    }

    pub fn prepare(&self) -> Result<()> {
        self.connection.lock().execute_batch(&format!(
            "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

BEGIN;
{}
{}
{}
{}
{}
COMMIT;
",
            Setting::get_ddl(),
            DisplayLanguageSetting::get_ddl(),
            Account::get_ddl(),
            Product::get_ddl(),
            LatestProductQuery::get_ddl(),
        ))?;

        Ok(())
    }
}
