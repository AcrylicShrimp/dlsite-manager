pub mod models;
pub mod tables;

use self::tables::{
    v2::{AccountTable, ProductDownloadTable, ProductTable, SettingTable},
    Table,
};
use crate::application_error::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            connection: Connection::open(path)?,
        })
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn prepare(&self) -> Result<()> {
        self.connection.execute_batch(&format!(
            "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

BEGIN;
{}
{}
{}
{}
COMMIT;
",
            SettingTable::get_ddl(),
            AccountTable::get_ddl(),
            ProductTable::get_ddl(),
            ProductDownloadTable::get_ddl(),
        ))?;

        Ok(())
    }

    pub fn drop(self) -> Result<()> {
        self.connection.close().map_err(|(_, err)| err)?;
        Ok(())
    }
}
