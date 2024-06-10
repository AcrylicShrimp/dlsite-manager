use super::error::CommandResult;
use crate::database::{models::v2::Setting, tables::v2::SettingTable};
use tauri::{api::dialog::blocking::FileDialogBuilder, Runtime, Window};

#[tauri::command]
pub async fn setting_get() -> CommandResult<Setting> {
    Ok(SettingTable::get()?.unwrap_or_default())
}

#[tauri::command]
pub async fn setting_browse_default_root_directory() -> CommandResult<Option<String>> {
    Ok(FileDialogBuilder::new()
        .set_title("Pick a default root directory")
        .pick_folder()
        .map(|err| err.to_str().unwrap().to_owned()))
}

#[tauri::command]
pub async fn setting_close<R: Runtime>(window: Window<R>) -> CommandResult<()> {
    window.close()?;
    Ok(())
}

#[tauri::command]
pub async fn setting_save_and_close<R: Runtime>(
    window: Window<R>,
    setting: Setting,
) -> CommandResult<()> {
    SettingTable::insert(&setting)?;
    window.close()?;
    Ok(())
}
