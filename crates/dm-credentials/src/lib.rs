use keyring_core::{set_default_store, Entry, Error as KeyringError};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

pub type Result<T> = std::result::Result<T, CredentialsError>;

pub const DEFAULT_SERVICE: &str = "dlsite-manager";

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error("invalid credential reference: {0}")]
    InvalidCredentialRef(&'static str),
    #[error("invalid credential service: {0}")]
    InvalidService(&'static str),
    #[error("credential store lock is poisoned")]
    StorePoisoned,
    #[error("keyring error: {0}")]
    Keyring(#[from] KeyringError),
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
pub struct KeyringCredentialStore {
    service: String,
}

impl KeyringCredentialStore {
    pub fn native_default() -> Result<Self> {
        Self::native(DEFAULT_SERVICE)
    }

    pub fn native(service: impl Into<String>) -> Result<Self> {
        let service = validate_service(service.into())?;

        use_native_store()?;

        Ok(Self { service })
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    fn entry(&self, credential_ref: &CredentialRef) -> Result<Entry> {
        Ok(Entry::new(&self.service, credential_ref.as_str())?)
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn save_password(&self, credential_ref: &CredentialRef, password: &str) -> Result<()> {
        self.entry(credential_ref)?.set_password(password)?;
        Ok(())
    }

    fn load_password(&self, credential_ref: &CredentialRef) -> Result<Option<String>> {
        match self.entry(credential_ref)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn delete_password(&self, credential_ref: &CredentialRef) -> Result<()> {
        match self.entry(credential_ref)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
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

fn validate_service(value: String) -> Result<String> {
    validate_identifier(&value).map_err(CredentialsError::InvalidService)?;
    Ok(value)
}

fn use_native_store() -> std::result::Result<(), KeyringError> {
    let config = HashMap::new();

    #[cfg(target_os = "android")]
    {
        use android_native_keyring_store::Store;
        set_default_store(Store::new_with_configuration(&config)?);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use apple_native_keyring_store::keychain::Store;
        set_default_store(Store::new_with_configuration(&config)?);
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        use zbus_secret_service_keyring_store::Store;
        set_default_store(Store::new_with_configuration(&config)?);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use windows_native_keyring_store::Store;
        set_default_store(Store::new_with_configuration(&config)?);
        Ok(())
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
    )))]
    {
        let _ = config;
        Err(KeyringError::NotSupportedByStore(
            "native credential store is not configured for this platform".to_owned(),
        ))
    }
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
