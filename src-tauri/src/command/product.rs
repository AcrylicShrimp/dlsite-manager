use super::{error::CommandResult, get_product_download_path};
use crate::{
    database::{
        models::v2::{Product, ProductDownload},
        tables::v2::{ProductDownloadTable, ProductTable},
    },
    dlsite::dto::{DLsiteProductAgeCategory, DLsiteProductType},
    services::download_service::DownloadService,
    window::{MainWindow, WindowInfoProvider},
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Manager, Runtime};
use tauri_plugin_shell::ShellExt;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct ProductQuery<'a> {
    pub query: Option<&'a str>,
    pub ty: Option<DLsiteProductType>,
    pub age: Option<DLsiteProductAgeCategory>,
    pub order_by_asc: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadProgressEvent<'a> {
    pub product_id: &'a str,
    pub progress: usize,
    pub decompressing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadEndEvent<'a> {
    pub product_id: &'a str,
    pub downloaded_path: Option<&'a Path>,
}

#[tauri::command]
pub async fn product_list_products<'a>(
    query: Option<ProductQuery<'a>>,
) -> CommandResult<Vec<Product>> {
    let query = query.unwrap_or_default();
    let results = ProductTable::get_many(query.query, query.ty, query.age, query.order_by_asc)
        .with_context(|| format!("[command/product_list_products] ProductTable::get_many"))?;
    Ok(results)
}

#[tauri::command]
pub async fn product_list_product_downloads(
    product_ids: Vec<String>,
) -> CommandResult<Vec<ProductDownload>> {
    let results = ProductDownloadTable::get_many(product_ids.into_iter()).with_context(|| {
        format!("[command/product_list_product_downloads] ProductDownloadTable::get_many")
    })?;
    Ok(results)
}

#[tauri::command]
pub async fn product_download_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
    product_id: String,
    decompress: Option<bool>,
    is_voice_comic: Option<bool>,
) -> CommandResult<()> {
    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
        window.emit("download-begin", &product_id)?;
    }

    let path = get_product_download_path(&app_handle)?;
    let downloaded_path = if is_voice_comic.unwrap_or_default() {
        DownloadService::new()
            .download_voice_comic(
                account_id,
                &product_id,
                &path,
                |progress, total_progress| {
                    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
                        window
                            .emit(
                                "download-progress",
                                ProductDownloadProgressEvent {
                                    product_id: &product_id,
                                    progress: (progress as f64 / total_progress as f64 * 100f64)
                                        .round()
                                        as usize,
                                    decompressing: false,
                                },
                            )
                            .ok();
                    }
                },
            )
            .await
    } else if decompress.unwrap_or(true) {
        DownloadService::new()
            .download_with_decompression(
                account_id,
                &product_id,
                &path,
                |progress, total_progress, decompressing| {
                    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
                        window
                            .emit(
                                "download-progress",
                                ProductDownloadProgressEvent {
                                    product_id: &product_id,
                                    progress: (progress as f64 / total_progress as f64 * 100f64)
                                        .round()
                                        as usize,
                                    decompressing,
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
                    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
                        window
                            .emit(
                                "download-progress",
                                ProductDownloadProgressEvent {
                                    product_id: &product_id,
                                    progress: (progress as f64 / total_progress as f64 * 100f64)
                                        .round()
                                        as usize,
                                    decompressing: false,
                                },
                            )
                            .ok();
                    }
                },
            )
            .await
    };

    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
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
        if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    };

    if !path.is_dir() {
        ProductDownloadTable::remove_one(&product_id)?;

        if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    }

    app_handle.shell().open(path.to_str().unwrap(), None)?;

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

    if let Some(window) = app_handle.get_webview_window(&MainWindow.label()) {
        window.emit("download-invalid", &product_id)?;
    }

    Ok(())
}
