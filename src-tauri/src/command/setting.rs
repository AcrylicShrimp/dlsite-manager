use crate::{
    application_error::Result,
    database::{
        models::v1::{DisplayLanguageSetting, Setting},
        tables::v1::{DisplayLanguageSettingTable, SettingTable},
    },
    window::{MainWindow, WindowInfoProvider},
};
use tauri::{api::dialog::blocking::FileDialogBuilder, Manager, Runtime, Window};

#[tauri::command]
pub async fn setting_get() -> Result<Setting> {
    SettingTable::get()
}

#[tauri::command]
pub async fn display_language_setting_get() -> Result<DisplayLanguageSetting> {
    DisplayLanguageSettingTable::get()
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
    display_language_setting: DisplayLanguageSetting,
) -> Result<()> {
    SettingTable::set(setting)?;
    DisplayLanguageSettingTable::set(&display_language_setting)?;
    window.close()?;

    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("display-language-changed", display_language_setting)?;
    }

    Ok(())
}
