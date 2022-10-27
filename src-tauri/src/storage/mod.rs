use self::account::Account;
use crate::application_error::Result;
use rusqlite::Connection;
use std::path::Path;

pub mod account;

pub struct Storage {
    connection: Connection,
}

impl Storage {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            connection: Connection::open(path)?,
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn prepare(&self) -> Result<()> {
        self.connection.execute_batch(&format!(
            "
PRAGMA journal_mode = WAL;

BEGIN;
{}
COMMIT;
",
            Account::get_ddl()
        ))?;

        Ok(())
    }
}
