use crate::{
    application::use_application,
    command::get_product_download_path,
    database::{models::v2::CreatingProductDownload, tables::v2::ProductDownloadTable},
};
use anyhow::Error as AnyError;
use std::fs::read_dir;

pub async fn refresh_product_download() -> Result<(), AnyError> {
    let download_path = get_product_download_path(use_application().app_handle())?;
    let contents = read_dir(download_path)?;

    ProductDownloadTable::remove_many_owned()?;

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

        ProductDownloadTable::insert_one(CreatingProductDownload {
            product_id: &file_name,
            path: &path,
        })?;
    }

    Ok(())
}
