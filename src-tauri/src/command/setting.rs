use crate::{
    application_error::Result,
    database::{models::v2::Setting, tables::v2::SettingTable},
};
use tauri::{api::dialog::blocking::FileDialogBuilder, Runtime, Window};

#[tauri::command]
pub async fn setting_get() -> Result<Setting> {
    Ok(SettingTable::get()?.unwrap_or_default())
}

#[tauri::command]
pub async fn setting_browse_default_root_directory() -> Result<Option<String>> {
    Ok(FileDialogBuilder::new()
        .set_title("Pick a default root directory")
        .pick_folder()
        .map(|err| err.to_str().unwrap().to_owned()))
}

#[tauri::command]
pub async fn setting_close<R: Runtime>(window: Window<R>) -> Result<()> {
    window.close()?;
    Ok(())
}

#[tauri::command]
pub async fn setting_save_and_close<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    window: Window<R>,
    setting: Setting,
) -> Result<()> {
    SettingTable::insert(&setting)?;
    window.close()?;
    Ok(())
}
