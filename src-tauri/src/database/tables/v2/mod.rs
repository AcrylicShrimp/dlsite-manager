mod account_table;
mod product_download_table;
mod product_table;
mod setting_table;

pub use account_table::*;
pub use product_download_table::*;
pub use product_table::*;
pub use setting_table::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBError {
    #[error("{0:?}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("{0:?}")]
    SerdeRusqliteError(#[from] serde_rusqlite::Error),
    #[error("{0:?}")]
    AnyError(#[from] anyhow::Error),
}

pub type DBResult<T> = Result<T, DBError>;
