use crate::{
    application::use_application,
    application_error::Result,
    dlsite::refresh_product,
    window::{MainWindow, WindowInfoProvider},
};
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RefreshProductListEvent {
    pub progress: usize,
    pub total_progress: usize,
}

pub async fn refresh_product_list() -> Result<()> {
    if let Some(window) = use_application()
        .app_handle()
        .get_window(&MainWindow.label())
    {
        window.emit("refresh-begin", ())?;
    }

    let result = refresh_product(|progress, total_progress| {
        if let Some(window) = use_application()
            .app_handle()
            .get_window(&MainWindow.label())
        {
            window.emit(
                "refresh-progress",
                RefreshProductListEvent {
                    progress,
                    total_progress,
                },
            )?;
        }

        Ok(())
    })
    .await;

    if let Some(window) = use_application()
        .app_handle()
        .get_window(&MainWindow.label())
    {
        window.emit("refresh-end", ())?;
    }

    result
}
