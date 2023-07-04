use std::path::PathBuf;

pub type Error = ApplicationError;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("app directory doesn't exists")]
    AppDirNotExist,
    #[error("cannot create app directory due to: {io_error}")]
    AppDirCreationError { io_error: std::io::Error },
    #[error("database error: {rusqlite_error}")]
    DatabaseError {
        #[from]
        rusqlite_error: rusqlite::Error,
    },
    #[error("created item is not accessible")]
    DatabaseCreatedItemNotAccessible,
    #[error("updated item is not accessible")]
    DatabaseUpdatedItemNotAccessible,
    #[error("tauri error: {tauri_error}")]
    TauriError {
        #[from]
        tauri_error: tauri::Error,
    },
    #[error("reqwest error: {reqwest_error}")]
    ReqwestError {
        #[from]
        reqwest_error: reqwest::Error,
    },
    #[error("reqwest cookie store error: {reqwest_cookie_store_error}")]
    ReqwestCookieStoreError {
        reqwest_cookie_store_error: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("the DLsite does not provide a required cookie(domain='{cookie_domain}', path='{cookie_path}', name='{cookie_name}')")]
    DLsiteCookieNotFound {
        cookie_domain: String,
        cookie_path: String,
        cookie_name: String,
    },
    #[error("you're not authenticated to the DLsite")]
    DLsiteNotAuthenticated,
    #[error("the product details from the DLsite is not in expected form")]
    DLsiteProductDetailMissingOrNotUnique,
    #[error("the Account(id='{account_id}') does not exists")]
    AccountNotExists { account_id: i64 },
    #[error("cannot create product directory due to: {io_error}")]
    ProductDirCreationError { io_error: std::io::Error },
    #[error("cannot open product file due to: {io_error}")]
    ProductFileCreationError { io_error: std::io::Error },
    #[error("cannot write to product file due to: {io_error}")]
    ProductFileWriteError { io_error: std::io::Error },
    #[error("cannot access product archive due to: {io_error}")]
    ProductArchiveOpenError { io_error: std::io::Error },
    #[error("cannot extract product archive due to: {extract_error}")]
    ProductArchiveExtractError {
        extract_error: zip_extract::ZipExtractError,
    },
    #[error("cannot delete product archive due to: {io_error}")]
    ProductArchiveDeleteError { io_error: std::io::Error },
    #[error("cannot cleanup product archive due to: {io_error}")]
    ProductArchiveCleanupError { io_error: std::io::Error },
    #[error("cannot open product path due to: {tauri_error}")]
    ProductPathOpenError { tauri_error: tauri::api::Error },
    #[error("cannot rename product rar archive due to: {io_error}")]
    ProductRarArchiveRenameError { io_error: std::io::Error },
    #[error("cannot extract product archive due to: {extract_error}")]
    ProductRarArchiveExtractOpenError {
        extract_error: unrar::error::UnrarError,
    },
    #[error("cannot extract product archive due to: {extract_error}")]
    ProductRarArchiveExtractProcessError {
        extract_error: unrar::error::UnrarError,
    },
    #[error("the given path is not a valid UTF-8 string: {path}")]
    NonUtf8PathError { path: PathBuf },
}

impl serde::Serialize for ApplicationError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
