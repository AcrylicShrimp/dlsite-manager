use super::get_product_download_path;
use crate::{
    application_error::{Error, Result},
    database::{
        models::v2::{Product, ProductDownload},
        tables::v2::{ProductDownloadTable, ProductTable},
    },
    dlsite::{
        api::{download_product, remove_downloaded_product},
        dto::{DLsiteProductAgeCategory, DLsiteProductType},
    },
    window::{MainWindow, WindowInfoProvider},
};
use serde::{Deserialize, Serialize};
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
    pub download: Option<ProductDownload>,
}

#[tauri::command]
pub async fn product_list_products<'a>(query: Option<ProductQuery<'a>>) -> Result<Vec<Product>> {
    let query = query.unwrap_or_default();
    Ok(ProductTable::get_many(query.query, query.ty, query.age, query.order_by_asc).unwrap())
}

#[tauri::command]
pub async fn product_download_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    account_id: i64,
    product_id: String,
    decompress: Option<bool>,
) -> Result<()> {
    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("download-begin", &product_id)?;
    }

    let path = get_product_download_path(&app_handle)?;
    let download = match download_product(
        decompress.unwrap_or(true),
        account_id,
        &product_id,
        &path,
        |progress, total_progress| {
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
        },
    )
    .await
    {
        Ok(path) => {
            let download = ProductDownload {
                product_id: product_id.clone(),
                path,
            };
            ProductDownloadTable::insert_one(&download)?;
            Some(download)
        }
        Err(..) => {
            remove_downloaded_product(&product_id, &path).ok();
            None
        }
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

    shell::open(&app_handle.shell_scope(), path.to_str().unwrap(), None)
        .map_err(|err| Error::ProductPathOpenError { tauri_error: err })?;

    Ok(())
}

#[tauri::command]
pub async fn product_remove_downloaded_product<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    product_id: String,
) -> Result<()> {
    let path = get_product_download_path(&app_handle)?;

    remove_downloaded_product(&product_id, path).ok();
    ProductDownloadTable::remove_one(&product_id)?;

    if let Some(window) = app_handle.get_window(&MainWindow.label()) {
        window.emit("download-invalid", &product_id)?;
    }

    Ok(())
}
