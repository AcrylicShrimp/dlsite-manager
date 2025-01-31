pub type Error = ApplicationError;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("cannot create app directory due to: {io_error}")]
    AppDirCreationFailure { io_error: std::io::Error },
    #[error("database error: {rusqlite_error}")]
    DatabaseOperationFailure {
        #[from]
        rusqlite_error: rusqlite::Error,
    },
    #[error("database conversion error: {serde_rusqlite_error}")]
    DatabaseConversionFailure {
        #[from]
        serde_rusqlite_error: serde_rusqlite::Error,
    },
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
}

impl serde::Serialize for ApplicationError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
