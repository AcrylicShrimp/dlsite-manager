use super::{error::CommandResult, get_product_download_path};
use crate::{
    database::{
        models::v2::Product,
        tables::v2::{ProductDownloadTable, ProductTable},
    },
    dlsite::dto::{DLsiteProductAgeCategory, DLsiteProductType},
    services::download_service::DownloadService,
    window::{MainWindow, WindowInfoProvider},
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{api::shell, Manager, Runtime};

#[derive(Default, Debug, Clone, Deserialize)]
pub struct ProductQuery<'s> {
    pub query: Option<&'s str>,
    pub ty: Option<DLsiteProductType>,
    pub age: Option<DLsiteProductAgeCategory>,
    pub order_by_asc: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadProgressEvent<'s> {
    pub product_id: &'s str,
    pub progress: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadEndEvent<'s> {
    pub product_id: &'s str,
    pub downloaded_path: Option<&'s Path>,
}

#[tauri::command]
pub async fn product_list_products<'a>(
    query: Option<ProductQuery<'a>>,
) -> CommandResult<Vec<Product>> {
    let query = query.unwrap_or_default();
    Ok(ProductTable::get_many(query.query, query.ty, query.age, query.order_by_asc).unwrap())
}

#[tauri::command]
pub async fn product_download_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
    product_id: String,
    decompress: Option<bool>,
) -> CommandResult<()> {
    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("download-begin", &product_id)?;
    }

    let path = get_product_download_path(&app_handle)?;
    let downloaded_path = if decompress.unwrap_or(true) {
        DownloadService::new()
            .download_with_decompression(
                account_id,
                &product_id,
                &path,
                |progress, total_progress| {
                    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
                        window
                            .emit(
                                "download-progress",
                                ProductDownloadProgressEvent {
                                    product_id: &product_id,
                                    progress: (progress as f64 / total_progress as f64 * 100f64)
                                        .round()
                                        as usize,
                                },
                            )
                            .ok();
                    }
                },
            )
            .await
    } else {
        DownloadService::new()
            .download(
                account_id,
                &product_id,
                &path,
                |progress, total_progress| {
                    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
                        window
                            .emit(
                                "download-progress",
                                ProductDownloadProgressEvent {
                                    product_id: &product_id,
                                    progress: (progress as f64 / total_progress as f64 * 100f64)
                                        .round()
                                        as usize,
                                },
                            )
                            .ok();
                    }
                },
            )
            .await
    };

    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit(
            "download-end",
            ProductDownloadEndEvent {
                product_id: &product_id,
                downloaded_path: downloaded_path.as_ref().map(|path| path.as_path()).ok(),
            },
        )?;
    }

    match downloaded_path {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

#[tauri::command]
pub async fn product_open_downloaded_folder<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    product_id: String,
) -> CommandResult<()> {
    let path = if let Some(download) = ProductDownloadTable::get_one(&product_id)? {
        download.path
    } else {
        if let Some(window) = app_handle.get_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    };

    if !path.is_dir() {
        ProductDownloadTable::remove_one(&product_id)?;

        if let Some(window) = app_handle.get_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    }

    shell::open(&app_handle.shell_scope(), path.to_str().unwrap(), None)?;

    Ok(())
}

#[tauri::command]
pub async fn product_remove_downloaded_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    product_id: String,
) -> CommandResult<()> {
    let path = get_product_download_path(&app_handle)?;

    DownloadService::new()
        .remove_downloaded(&product_id, path)
        .ok();

    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("download-invalid", &product_id)?;
    }

    Ok(())
}
