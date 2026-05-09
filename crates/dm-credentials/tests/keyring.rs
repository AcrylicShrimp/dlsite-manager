use dm_credentials::{CredentialRef, CredentialStore, KeyringCredentialStore};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn native_keyring_round_trip_when_enabled() -> dm_credentials::Result<()> {
    dotenvy::dotenv().ok();

    if std::env::var("DMSITE_CREDENTIALS_TEST_KEYRING")
        .ok()
        .as_deref()
        != Some("1")
    {
        return Ok(());
    }

    let service = std::env::var("DMSITE_CREDENTIALS_TEST_SERVICE")
        .unwrap_or_else(|_| "dlsite-manager-test".to_owned());
    let store = KeyringCredentialStore::native(service)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    let credential_ref = CredentialRef::new(format!("test:{}:{timestamp}", std::process::id()))?;

    store.delete_password(&credential_ref)?;
    store.save_password(&credential_ref, "temporary-test-password")?;

    assert_eq!(
        store.load_password(&credential_ref)?,
        Some("temporary-test-password".to_owned())
    );

    store.delete_password(&credential_ref)?;
    assert_eq!(store.load_password(&credential_ref)?, None);

    Ok(())
}
