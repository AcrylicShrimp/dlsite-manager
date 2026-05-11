use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub type Result<T> = std::result::Result<T, CredentialsError>;

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error("invalid credential reference: {0}")]
    InvalidCredentialRef(&'static str),
    #[error("credential store lock is poisoned")]
    StorePoisoned,
    #[error("credential file I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("credential file JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialRef(String);

impl CredentialRef {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_identifier(&value).map_err(CredentialsError::InvalidCredentialRef)?;
        Ok(Self(value))
    }

    pub fn account_password(account_id: &str) -> Result<Self> {
        validate_identifier(account_id).map_err(CredentialsError::InvalidCredentialRef)?;
        Self::new(format!("account:{account_id}:password"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CredentialRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub trait CredentialStore: Send + Sync {
    fn save_password(&self, credential_ref: &CredentialRef, password: &str) -> Result<()>;
    fn load_password(&self, credential_ref: &CredentialRef) -> Result<Option<String>>;
    fn delete_password(&self, credential_ref: &CredentialRef) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct LocalCredentialStore {
    path: PathBuf,
    file_lock: Arc<Mutex<()>>,
}

impl LocalCredentialStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            set_private_dir_permissions(parent)?;
        }

        if !path.exists() {
            write_password_file(&path, &PasswordFile::default())?;
        }

        Ok(Self {
            path,
            file_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load_file(&self) -> Result<PasswordFile> {
        match fs::read_to_string(&self.path) {
            Ok(content) if content.trim().is_empty() => Ok(PasswordFile::default()),
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PasswordFile::default()),
            Err(error) => Err(error.into()),
        }
    }

    fn save_file(&self, file: &PasswordFile) -> Result<()> {
        write_password_file(&self.path, file)
    }
}

impl CredentialStore for LocalCredentialStore {
    fn save_password(&self, credential_ref: &CredentialRef, password: &str) -> Result<()> {
        let _guard = self
            .file_lock
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        let mut file = self.load_file()?;

        file.passwords
            .insert(credential_ref.as_str().to_owned(), password.to_owned());
        self.save_file(&file)
    }

    fn load_password(&self, credential_ref: &CredentialRef) -> Result<Option<String>> {
        let _guard = self
            .file_lock
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        Ok(self
            .load_file()?
            .passwords
            .get(credential_ref.as_str())
            .cloned())
    }

    fn delete_password(&self, credential_ref: &CredentialRef) -> Result<()> {
        let _guard = self
            .file_lock
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        let mut file = self.load_file()?;

        file.passwords.remove(credential_ref.as_str());
        self.save_file(&file)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PasswordFile {
    #[serde(default)]
    passwords: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct InMemoryCredentialStore {
    passwords: Arc<Mutex<HashMap<CredentialRef, String>>>,
}

impl fmt::Debug for InMemoryCredentialStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InMemoryCredentialStore")
            .field("passwords", &"<redacted>")
            .finish()
    }
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn save_password(&self, credential_ref: &CredentialRef, password: &str) -> Result<()> {
        let mut passwords = self
            .passwords
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        passwords.insert(credential_ref.clone(), password.to_owned());
        Ok(())
    }

    fn load_password(&self, credential_ref: &CredentialRef) -> Result<Option<String>> {
        let passwords = self
            .passwords
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        Ok(passwords.get(credential_ref).cloned())
    }

    fn delete_password(&self, credential_ref: &CredentialRef) -> Result<()> {
        let mut passwords = self
            .passwords
            .lock()
            .map_err(|_| CredentialsError::StorePoisoned)?;
        passwords.remove(credential_ref);
        Ok(())
    }
}

fn write_password_file(path: &Path, file: &PasswordFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_private_dir_permissions(parent)?;
    }

    let temporary_path = path.with_extension("json.tmp");
    let content = serde_json::to_vec_pretty(file)?;

    write_private_file(&temporary_path, &content)?;
    fs::rename(&temporary_path, path)?;
    set_private_file_permissions(path)?;

    Ok(())
}

#[cfg(unix)]
fn write_private_file(path: &Path, content: &[u8]) -> Result<()> {
    use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt};

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, content: &[u8]) -> Result<()> {
    use std::io::Write;

    let mut file = fs::File::create(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn validate_identifier(value: &str) -> std::result::Result<(), &'static str> {
    if value.is_empty() {
        return Err("value cannot be empty");
    }

    if value.contains('\0') {
        return Err("value contains a NUL byte");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_account_password_reference() -> Result<()> {
        let credential_ref = CredentialRef::account_password("local-account-id")?;

        assert_eq!(credential_ref.as_str(), "account:local-account-id:password");

        Ok(())
    }

    #[test]
    fn rejects_invalid_credential_references() {
        assert!(matches!(
            CredentialRef::new(""),
            Err(CredentialsError::InvalidCredentialRef(_))
        ));
        assert!(matches!(
            CredentialRef::account_password(""),
            Err(CredentialsError::InvalidCredentialRef(_))
        ));
        assert!(matches!(
            CredentialRef::new("account\0password"),
            Err(CredentialsError::InvalidCredentialRef(_))
        ));
    }

    #[test]
    fn in_memory_store_round_trips_passwords() -> Result<()> {
        let store = InMemoryCredentialStore::new();
        let credential_ref = CredentialRef::account_password("account-a")?;

        assert_eq!(store.load_password(&credential_ref)?, None);

        store.save_password(&credential_ref, "secret")?;
        assert_eq!(
            store.load_password(&credential_ref)?,
            Some("secret".to_owned())
        );

        store.delete_password(&credential_ref)?;
        assert_eq!(store.load_password(&credential_ref)?, None);

        Ok(())
    }

    #[test]
    fn in_memory_store_delete_is_idempotent() -> Result<()> {
        let store = InMemoryCredentialStore::new();
        let credential_ref = CredentialRef::account_password("account-a")?;

        store.delete_password(&credential_ref)?;
        store.delete_password(&credential_ref)?;

        Ok(())
    }

    #[test]
    fn local_store_round_trips_passwords_across_reopen() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("credentials").join("vault.json");
        let credential_ref = CredentialRef::account_password("account-a")?;

        let store = LocalCredentialStore::open(&path)?;
        assert_eq!(store.load_password(&credential_ref)?, None);

        store.save_password(&credential_ref, "secret")?;
        assert_eq!(
            store.load_password(&credential_ref)?,
            Some("secret".to_owned())
        );

        let reopened = LocalCredentialStore::open(&path)?;
        assert_eq!(
            reopened.load_password(&credential_ref)?,
            Some("secret".to_owned())
        );

        reopened.delete_password(&credential_ref)?;
        assert_eq!(store.load_password(&credential_ref)?, None);

        Ok(())
    }

    #[test]
    fn local_store_delete_is_idempotent() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let store = LocalCredentialStore::open(directory.path().join("vault.json"))?;
        let credential_ref = CredentialRef::account_password("account-a")?;

        store.delete_password(&credential_ref)?;
        store.delete_password(&credential_ref)?;

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn local_store_uses_private_permissions() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir()?;
        let vault_dir = directory.path().join("credentials");
        let path = vault_dir.join("vault.json");

        LocalCredentialStore::open(&path)?;

        assert_eq!(
            fs::metadata(&vault_dir)?.permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);

        Ok(())
    }

    #[test]
    fn trait_object_supports_swappable_stores() -> Result<()> {
        let store: Box<dyn CredentialStore> = Box::new(InMemoryCredentialStore::new());
        let credential_ref = CredentialRef::account_password("account-a")?;

        store.save_password(&credential_ref, "secret")?;

        assert_eq!(
            store.load_password(&credential_ref)?,
            Some("secret".to_owned())
        );

        Ok(())
    }
}
