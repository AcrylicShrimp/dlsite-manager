use crate::{
    application_error::{Error, Result},
    dlsite::{get_product_count, login},
    storage::account::*,
    window::{AccountEditWindow, AccountManagementWindow, WindowInfoProvider},
};
use tauri::{Manager, Runtime, Window};

#[tauri::command]
pub fn account_management_list_accounts() -> Result<Vec<Account>> {
    Account::list_all()
}

#[tauri::command]
pub fn account_management_get_account(account_id: i64) -> Result<Option<Account>> {
    Account::get_one(account_id)
}

#[tauri::command]
pub fn account_management_add_account<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    window: Window<R>,
    account: CreatedAccount,
) -> Result<()> {
    let account = Account::create_one(account)?;

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
    account: UpdatedAccount,
) -> Result<()> {
    let account = Account::update_one(account)?;

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
) -> Result<()> {
    Account::remove_one(account_id)?;

    if let Some(window) = app_handle.get_window(&AccountManagementWindow.label()) {
        window.emit("remove-account", account_id)?;
    }

    if let Some(window) = app_handle.get_window(&AccountEditWindow { account_id }.label()) {
        window.close()?;
    }

    Ok(())
}

#[tauri::command]
pub async fn account_management_test_account(username: String, password: String) -> Result<isize> {
    match get_product_count(login(username.clone(), password.clone()).await?).await {
        Ok(product_count) => Ok(product_count as isize),
        Err(err) => match err {
            Error::DLsiteNotAuthenticated => Ok(-1),
            _ => return Err(err),
        },
    }
}
