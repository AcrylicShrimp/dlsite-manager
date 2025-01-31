use crate::{
    application::use_application,
    command::get_product_download_path,
    database::{
        models::v2::{CreatingProduct, CreatingProductDownload},
        tables::v2::{ProductDownloadTable, ProductTable},
    },
    dlsite::{api::get_product_from_non_owner_api, dto::DLsiteProduct},
    window::{MainWindow, WindowInfoProvider},
};
use anyhow::Error as AnyError;
use log::error;
use std::{fs::read_dir, path::PathBuf};
use tauri::{Emitter as _, Manager};

pub async fn scan_downloaded_products() -> Result<(), AnyError> {
    if let Some(window) = use_application()
        .app_handle()
        .get_webview_window(&MainWindow.label())
    {
        window.emit("refresh-begin", "no-progress")?;
    }

    let download_path = get_product_download_path(use_application().app_handle())?;
    let contents = read_dir(download_path)?;

    ProductDownloadTable::remove_many()?;
    ProductTable::remove_many_not_owned()?;

    struct ScannedProductDownload {
        pub id: String,
        pub path: PathBuf,
    }

    let mut scanned_products = Vec::new();

    for entry in contents {
        let entry = entry?;

        if !entry.file_type()?.is_dir() {
            continue;
        }

        let file_name = match entry.file_name().into_string() {
            Ok(file_name) => file_name,
            Err(_) => {
                continue;
            }
        };
        let path = entry.path();

        scanned_products.push(ScannedProductDownload {
            id: file_name,
            path,
        });
    }

    struct ScannedProduct {
        pub id: String,
        pub path: PathBuf,
        pub product: DLsiteProduct,
    }

    let products = futures::future::join_all(scanned_products.into_iter().map(|product| async {
        let fetched_product = match get_product_from_non_owner_api(&product.id).await {
            Ok(product) => product,
            Err(_) => {
                return None;
            }
        };

        Some(ScannedProduct {
            id: product.id,
            path: product.path,
            product: fetched_product,
        })
    }))
    .await;

    if let Err(err) = ProductTable::insert_many(products.iter().filter_map(|product| {
        let product = match product {
            Some(product) => product,
            None => return None,
        };

        Some(CreatingProduct {
            id: &product.product.id,
            account_id: None,
            ty: product.product.ty.clone(),
            age: product.product.age.clone(),
            title: &product.product.title,
            thumbnail: &product.product.thumbnail,
            group_id: &product.product.group_id,
            group_name: &product.product.group_name,
            registered_at: product.product.registered_at,
        })
    })) {
        error!(
            "[scan_downloaded_products] failed to update the products to the database: {:?}",
            err
        );
        return Err(err.into());
    }

    for product in products {
        let product = match product {
            Some(product) => product,
            None => {
                continue;
            }
        };

        if let Err(err) = ProductDownloadTable::insert_one(CreatingProductDownload {
            product_id: &product.id,
            path: &product.path,
        }) {
            error!(
                "[scan_downloaded_products] failed to insert the scanned product `{}` at `{}` to the database: {:?}",
                product.id,
                product.path.display(),
                err
            );
        }
    }

    if let Some(window) = use_application()
        .app_handle()
        .get_webview_window(&MainWindow.label())
    {
        window.emit("refresh-end", ())?;
    }

    Ok(())
}
