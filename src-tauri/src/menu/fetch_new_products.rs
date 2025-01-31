use crate::{
    application::use_application,
    services::dlsite_service::DLsiteService,
    window::{MainWindow, WindowInfoProvider},
};
use anyhow::Error as AnyError;
use serde::Serialize;
use tauri::{Emitter as _, Manager};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct FetchNewProductsProgressEvent {
    pub progress: u32,
    pub total_progress: u32,
}

pub async fn fetch_new_products() -> Result<(), AnyError> {
    if let Some(window) = use_application()
        .app_handle()
        .get_webview_window(&MainWindow.label())
    {
        window.emit("refresh-begin", ())?;
    }

    let result = DLsiteService::new()
        .fetch_new_products(|progress, total_progress| {
            if let Some(window) = use_application()
                .app_handle()
                .get_webview_window(&MainWindow.label())
            {
                window
                    .emit(
                        "refresh-progress",
                        FetchNewProductsProgressEvent {
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
