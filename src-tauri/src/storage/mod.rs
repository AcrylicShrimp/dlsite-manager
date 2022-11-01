use self::{account::Account, product::Product, setting::Setting};
use crate::application_error::Result;
use parking_lot::{Mutex, MutexGuard};
use rusqlite::Connection;
use std::path::Path;

pub mod account;
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
COMMIT;
",
            Setting::get_ddl(),
            Account::get_ddl(),
            Product::get_ddl(),
        ))?;

        Ok(())
    }
}
