use super::error::CommandResult;
use crate::{
    database::{
        models::v2::{Account, CreatingAccount, UpdatingAccount},
        tables::v2::AccountTable,
    },
    dlsite::api::{get_product_count, login, LoginError},
    window::{AccountEditWindow, AccountManagementWindow, WindowInfoProvider},
};
use tauri::{Manager, Runtime, Window};

#[tauri::command]
pub fn account_management_list_accounts() -> CommandResult<Vec<Account>> {
    Ok(AccountTable::get_all()?)
}

#[tauri::command]
pub fn account_management_get_account(account_id: i64) -> CommandResult<Option<Account>> {
    Ok(AccountTable::get_one(account_id)?)
}

#[tauri::command]
pub fn account_management_add_account<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    window: Window<R>,
    account: CreatingAccount,
) -> CommandResult<()> {
    let account = AccountTable::insert_one(account)?;

    if let Some(window) = app_handle.get_window(&AccountManagementWindow.label()) {
        window.emit("add-account", account)?;
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
    let account = AccountTable::update_one(account)?;

    if let Some(window) = app_handle.get_window(&AccountManagementWindow.label()) {
        window.emit("edit-account", account)?;
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

    if let Some(window) = app_handle.get_window(&AccountManagementWindow.label()) {
        window.emit("remove-account", account_id)?;
    }

    if let Some(window) = app_handle.get_window(&AccountEditWindow { account_id }.label()) {
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
            return match err {
                LoginError::WrongCredentials => Ok(-1),
                LoginError::Other(err) => Err(err.into()),
            }
        }
    };

    Ok(get_product_count(cookie)
        .await
        .map(|count| count as isize)?)
}
