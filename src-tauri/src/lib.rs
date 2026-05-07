use dm_storage::{AppSettings, Storage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Manager, State};

struct AppState {
    storage: Storage,
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, String> {
    state
        .storage
        .app_settings()
        .await
        .map(AppSettingsDto::from)
        .map_err(command_error)
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    settings: SaveSettingsRequest,
) -> Result<AppSettingsDto, String> {
    let settings = settings.into_app_settings()?;

    state
        .storage
        .save_app_settings(&settings)
        .await
        .map_err(command_error)?;

    Ok(AppSettingsDto::from(settings))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsDto {
    library_root: Option<String>,
    download_root: Option<String>,
}

impl From<AppSettings> for AppSettingsDto {
    fn from(settings: AppSettings) -> Self {
        Self {
            library_root: settings.library_root,
            download_root: settings.download_root,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    library_root: Option<String>,
    download_root: Option<String>,
}

impl SaveSettingsRequest {
    fn into_app_settings(self) -> Result<AppSettings, String> {
        Ok(AppSettings {
            library_root: normalize_path_setting(self.library_root)?,
            download_root: normalize_path_setting(self.download_root)?,
        })
    }
}

fn normalize_path_setting(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Ok(None);
    }

    if value.contains('\0') {
        return Err("path contains a NUL byte".to_owned());
    }

    Ok(Some(value))
}

fn command_error(error: dm_storage::StorageError) -> String {
    error.to_string()
}

fn setup_storage(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let database_path: PathBuf = app_data_dir.join("dlsite-manager.sqlite");
    let storage = tauri::async_runtime::block_on(async {
        let storage = Storage::open(&database_path).await?;
        storage.run_migrations().await?;
        dm_storage::Result::Ok(storage)
    })?;

    app.manage(AppState { storage });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(setup_storage)
        .invoke_handler(tauri::generate_handler![get_settings, save_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
