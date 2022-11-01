use crate::{application_error::Result, storage::setting::Setting};
use tauri::{api::dialog::blocking::FileDialogBuilder, Runtime, Window};

#[tauri::command]
pub async fn setting_get() -> Result<Setting> {
    Setting::get()
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
pub async fn setting_save_and_close<R: Runtime>(window: Window<R>, setting: Setting) -> Result<()> {
    Setting::set(setting)?;
    window.close()?;
    Ok(())
}
