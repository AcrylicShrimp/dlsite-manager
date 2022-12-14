use crate::{
    application_error::{Error, Result},
    dlsite::download_product,
    storage::{
        product::{Product, ProductDownload, ProductQuery},
        setting::Setting,
    },
    window::{MainWindow, WindowInfoProvider},
};
use serde::Serialize;
use tauri::{
    api::{path::download_dir, shell},
    Manager, Runtime,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadProgressEvent<'s> {
    pub product_id: &'s str,
    pub progress: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductDownloadEndEvent<'s> {
    pub product_id: &'s str,
    pub download: Option<ProductDownload>,
}

#[tauri::command]
pub async fn product_list_products(query: Option<ProductQuery>) -> Result<Vec<Product>> {
    Ok(Product::list_all(&query.unwrap_or_default()).unwrap())
}

#[tauri::command]
pub async fn product_download_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
    product_id: String,
) -> Result<()> {
    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("download-begin", &product_id)?;
    }

    let setting = Setting::get()?;
    let path = setting.download_root_dir.unwrap_or_else(|| {
        download_dir()
            .unwrap_or_else(|| app_handle.path_resolver().app_dir().unwrap())
            .join("DLsite")
    });

    let download =
        match download_product(account_id, &product_id, path, |progress, total_progress| {
            if let Some(window) = app_handle.get_window(&MainWindow.label()) {
                window.emit(
                    "download-progress",
                    ProductDownloadProgressEvent {
                        product_id: &product_id,
                        progress: (progress as f64 / total_progress as f64 * 100f64).round()
                            as usize,
                    },
                )?;
            }

            Ok(())
        })
        .await
        {
            Ok(path) => Some(Product::insert_download(
                &product_id,
                path.to_str().unwrap(),
            )?),
            Err(..) => None,
        };

    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit(
            "download-end",
            ProductDownloadEndEvent {
                product_id: &product_id,
                download,
            },
        )?;
    }

    Ok(())
}

#[tauri::command]
pub async fn product_open_downloaded_folder<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    product_id: String,
) -> Result<()> {
    let path = if let Some(download) = Product::get_one_download(&product_id)? {
        download.path
    } else {
        if let Some(window) = app_handle.get_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    };

    if !path.is_dir() {
        Product::remove_one_download(&product_id)?;

        if let Some(window) = app_handle.get_window(&MainWindow.label()) {
            window.emit("download-invalid", &product_id)?;
        }

        return Ok(());
    }

    shell::open(&app_handle.shell_scope(), path.to_str().unwrap(), None)
        .map_err(|err| Error::ProductPathOpenError { tauri_error: err })?;

    Ok(())
}
