pub type Error = ApplicationError;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("app directory doesn't exists")]
    AppDirNotExist,
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
    #[error("the DLsite does not provide a required cookie(domain='{cookie_domain}', path='{cookie_path}', name='{cookie_name}')")]
    DLsiteCookieNotFound {
        cookie_domain: String,
        cookie_path: String,
        cookie_name: String,
    },
    #[error("you're not authenticated to the DLsite")]
    DLsiteNotAuthenticated,
}

impl serde::Serialize for ApplicationError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
