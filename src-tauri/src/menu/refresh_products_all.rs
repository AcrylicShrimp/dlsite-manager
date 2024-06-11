use crate::{
    application::use_application,
    services::dlsite_service::DLsiteService,
    window::{MainWindow, WindowInfoProvider},
};
use anyhow::Error as AnyError;
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RefreshProductsAllProgressEvent {
    pub progress: u32,
    pub total_progress: u32,
}

pub async fn refresh_products_all() -> Result<(), AnyError> {
    if let Some(window) = use_application()
        .app_handle()
        .get_webview_window(&MainWindow.label())
    {
        window.emit("refresh-begin", ())?;
    }

    let result = DLsiteService::new()
        .refresh_products_all(|progress, total_progress| {
            if let Some(window) = use_application()
                .app_handle()
                .get_webview_window(&MainWindow.label())
            {
                window
                    .emit(
                        "refresh-progress",
                        RefreshProductsAllProgressEvent {
                            progress,
                            total_progress,
                        },
                    )
                    .ok();
            }
        })
        .await;

    if let Some(window) = use_application()
        .app_handle()
        .get_webview_window(&MainWindow.label())
    {
        window.emit("refresh-end", ())?;
    }

    Ok(result?)
}
