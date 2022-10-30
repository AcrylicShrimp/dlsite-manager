use crate::{
    application_error::Result,
    window::{AccountAddWindow, AccountEditWindow, BuildableWindow},
};
use tauri::{Runtime, Window};

#[tauri::command]
pub fn show_window<R: Runtime>(window: Window<R>) -> Result<()> {
    window.show()?;
    Ok(())
}

#[tauri::command]
pub async fn spawn_window_account_add<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<()> {
    AccountAddWindow.build_or_focus(&app_handle)?;
    Ok(())
}

#[tauri::command]
pub async fn spawn_window_account_edit<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
) -> Result<()> {
    AccountEditWindow { account_id }.build_or_focus(&app_handle)?;
    Ok(())
}
