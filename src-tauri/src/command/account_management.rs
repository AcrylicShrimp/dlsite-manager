use super::error::CommandResult;
use crate::{
    database::{
        models::v2::{Account, CreatingAccount, SimpleAccount, UpdatingAccount},
        tables::v2::AccountTable,
    },
    dlsite::api::{get_product_count, login, LoginError},
    window::{AccountEditWindow, AccountManagementWindow, WindowInfoProvider},
};
use log::warn;
use tauri::{Emitter as _, Manager, Runtime, Window};

#[tauri::command]
pub fn account_management_list_accounts() -> CommandResult<Vec<Account>> {
    Ok(AccountTable::get_all()?)
}

#[tauri::command]
pub fn account_management_get_account(account_id: i64) -> CommandResult<Option<SimpleAccount>> {
    Ok(AccountTable::get_one_simple(account_id)?)
}

#[tauri::command]
pub fn account_management_add_account<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    window: Window<R>,
    account: CreatingAccount,
) -> CommandResult<()> {
    let account_id = AccountTable::insert_one(account.clone())?;

    if let Some(window) = app_handle.get_webview_window(&AccountManagementWindow.label()) {
        window.emit(
            "add-account",
            SimpleAccount {
                id: account_id,
                username: account.username.to_owned(),
                password: account.password.to_owned(),
                memo: account.memo.map(|memo| memo.to_owned()),
            },
        )?;
    }

    window.close()?;
    Ok(())
}

#[tauri::command]
pub fn account_management_update_account<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    window: Window<R>,
    account: UpdatingAccount,
) -> CommandResult<()> {
    AccountTable::update_one(account.clone())?;

    if let Some(window) = app_handle.get_webview_window(&AccountManagementWindow.label()) {
        window.emit(
            "edit-account",
            SimpleAccount {
                id: account.id,
                username: account.username.to_owned(),
                password: account.password.to_owned(),
                memo: account.memo.map(|memo| memo.to_owned()),
            },
        )?;
    }

    window.close()?;
    Ok(())
}

#[tauri::command]
pub fn account_management_remove_account<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
) -> CommandResult<()> {
    AccountTable::remove_one(account_id)?;

    if let Some(window) = app_handle.get_webview_window(&AccountManagementWindow.label()) {
        window.emit("remove-account", account_id)?;
    }

    if let Some(window) = app_handle.get_webview_window(&AccountEditWindow { account_id }.label()) {
        window.close()?;
    }

    Ok(())
}

#[tauri::command]
pub async fn account_management_test_account(
    username: String,
    password: String,
) -> CommandResult<isize> {
    let cookie = match login(username, password).await {
        Ok(cookie) => cookie,
        Err(err) => {
            warn!(
                "[account_management_test_account] failed to test account: {:?}",
                err
            );

            return match err {
                LoginError::WrongCredentials => Ok(-1),
                LoginError::Other(err) => Err(err.into()),
            };
        }
    };

    Ok(get_product_count(cookie)
        .await
        .map(|count| count as isize)?)
}
