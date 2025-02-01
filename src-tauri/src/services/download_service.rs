use super::dlsite_service::DLsiteServiceError;
use crate::{
    database::{
        models::v2::CreatingProductDownload,
        tables::v2::{DBError, ProductDownloadTable},
    },
    dlsite::{
        api::{
            download_product_files, download_voice_comic, get_product_files,
            get_voice_comic_request_info, get_voice_comic_zip_tree,
        },
        dto::DLsiteProductFiles,
    },
    services::dlsite_service::DLsiteService,
};
use anyhow::{Context, Error as AnyError};
use log::{error, info, warn};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadServiceError {
    #[error("{0:?}")]
    DBError(#[from] DBError),
    #[error("{0:?}")]
    IOError(#[from] std::io::Error),
    #[error("{0:?}")]
    ZipExtractError(#[from] zip_extract::ZipExtractError),
    #[error("{0:?}")]
    UnrarError(#[from] unrar::error::UnrarError),
    #[error("{0:?}")]
    AnyError(#[from] AnyError),
    #[error("{0:?}")]
    DLsiteServiceError(#[from] DLsiteServiceError),
}

pub struct DownloadService;

impl DownloadService {
    pub fn new() -> Self {
        Self
    }

    pub async fn download(
        &self,
        account_id: i64,
        product_id: impl AsRef<str>,
        base_path: impl AsRef<Path>,
        on_progress: impl Fn(u64, u64),
    ) -> Result<PathBuf, DownloadServiceError> {
        Ok(download(account_id, product_id, base_path, on_progress)
            .await?
            .base_path)
    }

    pub async fn download_with_decompression(
        &self,
        account_id: i64,
        product_id: impl AsRef<str>,
        base_path: impl AsRef<Path>,
        on_progress: impl Fn(u64, u64, bool),
    ) -> Result<PathBuf, DownloadServiceError> {
        let product_id = product_id.as_ref();
        let downloaded = download(account_id, product_id, base_path, |progress, total| {
            on_progress(progress, total, false);
        })
        .await?;

        if downloaded.product_files.files.len() == 1
            && downloaded.product_files.files[0]
                .file_name
                .to_ascii_lowercase()
                .ends_with(".zip")
        {
            on_progress(1, 1, true);

            if let Err(err) =
                decompress_single(&downloaded.product_files, &downloaded.base_path).await
            {
                warn!(
                    "[download_with_decompression] failed to decompress (single) the product `{}`: {:?}",
                    product_id, err
                );
            }
        }

        if downloaded.product_files.files.len() != 0
            && downloaded.product_files.files[0]
                .file_name
                .to_ascii_lowercase()
                .ends_with(".exe")
        {
            on_progress(1, 1, true);

            if let Err(err) =
                decompress_multiple(&downloaded.product_files, &downloaded.base_path).await
            {
                warn!(
                    "[download_with_decompression] failed to decompress (multiple) the product `{}`: {:?}",
                    product_id, err
                );
            }
        }

        Ok(downloaded.base_path)
    }

    pub async fn download_voice_comic(
        &self,
        account_id: i64,
        product_id: impl AsRef<str>,
        base_path: impl AsRef<Path>,
        on_progress: impl Fn(u64, u64),
    ) -> Result<PathBuf, DownloadServiceError> {
        let product_id = product_id.as_ref();
        let path = base_path.as_ref().join(product_id);

        info!(
            "[download_voice_comic] downloading product `{}` of the account id `{}` at path `{}`",
            product_id,
            account_id,
            path.display()
        );

        let cookie_store = DLsiteService::new().get_cookie_store(account_id).await?;
        let voice_comic_request_info = match get_voice_comic_request_info(
            cookie_store.clone(),
            product_id,
        )
        .await
        {
            Ok(request_info) => request_info,
            Err(err) => {
                error!("[download_voice_comic] failed to download product `{}` of the account id `{}` at path `{}`: {:?}",
                product_id,
                    account_id,
                    path.display(),
                    err
                );
                return Err(DownloadServiceError::AnyError(err));
            }
        };
        let voice_comic_zip_tree = match get_voice_comic_zip_tree(
            cookie_store.clone(),
            &voice_comic_request_info,
        )
        .await
        {
            Ok(request_info) => request_info,
            Err(err) => {
                error!("[download_voice_comic] failed to download product `{}` of the account id `{}` at path `{}`: {:?}",
                product_id,
                    account_id,
                    path.display(),
                    err
                );
                return Err(DownloadServiceError::AnyError(err));
            }
        };

        if let Err(err) = download_voice_comic(
            cookie_store,
            product_id,
            &voice_comic_request_info,
            &voice_comic_zip_tree,
            base_path.as_ref(),
            on_progress,
        )
        .await
        {
            error!(
            "[download_voice_comic] failed to download product `{}` of the account id `{}` at path `{}`: {:?}",
                product_id,
                account_id,
                path.display(),
                err
            );
            return Err(DownloadServiceError::AnyError(err));
        }

        if let Err(err) = ProductDownloadTable::insert_one(CreatingProductDownload {
            product_id,
            path: &path,
        }) {
            warn!(
            "[download_voice_comic] failed to insert the downloaded product `{}` to the database at path `{}`: {:?}",
                product_id,
                path.display(),
                err
            );
        }

        Ok(base_path.as_ref().to_owned())
    }

    pub fn remove_downloaded(
        &self,
        product_id: impl AsRef<str>,
        base_path: impl AsRef<Path>,
    ) -> Result<(), DownloadServiceError> {
        let product_id = product_id.as_ref();
        let path = base_path.as_ref().join(product_id);

        info!(
            "[remove_downloaded] removing the downloaded product `{}` at path `{}`",
            product_id,
            path.display()
        );

        if let Err(err) = ProductDownloadTable::remove_one(product_id) {
            warn!("[remove_downloaded] failed to remove the downloaded product `{}` from the database at path `{}`: {:?}",
                product_id,
                path.display(),
                err
            );
        }

        if let Err(err) = std::fs::remove_dir_all(&path) {
            warn!("[remove_downloaded] failed to remove the downloaded product `{}` to the file system at path `{}`: {:?}",
                product_id,
                path.display(),
                err
            );
        }

        Ok(())
    }
}

struct Downloaded {
    pub base_path: PathBuf,
    pub product_files: DLsiteProductFiles,
}

async fn download(
    account_id: i64,
    product_id: impl AsRef<str>,
    base_path: impl AsRef<Path>,
    on_progress: impl Fn(u64, u64),
) -> Result<Downloaded, DownloadServiceError> {
    let product_id = product_id.as_ref();
    let path = base_path.as_ref().join(product_id);

    info!(
        "[download] downloading product `{}` of the account id `{}` at path `{}`",
        product_id,
        account_id,
        path.display()
    );

    let cookie_store = DLsiteService::new().get_cookie_store(account_id).await?;
    let product_files = match get_product_files(product_id).await {
        Ok(product_files) => product_files,
        Err(err) => {
            error!("[download] failed to download product `{}` of the account id `{}` at path `{}`: {:?}",
                product_id,
                    account_id,
                    path.display(),
                    err
                );
            return Err(DownloadServiceError::AnyError(err));
        }
    };

    if let Err(err) = download_product_files(
        cookie_store,
        product_id,
        &product_files,
        base_path,
        on_progress,
    )
    .await
    {
        error!(
            "[download] failed to download product `{}` of the account id `{}` at path `{}`: {:?}",
            product_id,
            account_id,
            path.display(),
            err
        );
        return Err(DownloadServiceError::AnyError(err));
    }

    if let Err(err) = ProductDownloadTable::insert_one(CreatingProductDownload {
        product_id,
        path: &path,
    }) {
        warn!(
            "[download] failed to insert the downloaded product `{}` to the database at path `{}`: {:?}",
            product_id,
            path.display(),
            err
        );
    }

    Ok(Downloaded {
        base_path: path.to_owned(),
        product_files,
    })
}

async fn decompress_single(
    product_files: &DLsiteProductFiles,
    path: impl AsRef<Path>,
) -> Result<(), DownloadServiceError> {
    use std::fs::*;
    use std::io::BufReader;

    let path = path.as_ref();
    let tmp_path = path.join("__tmp__");
    let file_path = path.join(&product_files.files[0].file_name);
    let file = OpenOptions::new()
        .read(true)
        .open(&file_path)
        .with_context(|| format!("[decompress_single]"))
        .with_context(|| format!("opening file `{}`", file_path.display()))?;
    let reader = BufReader::new(file);

    zip_extract::extract(reader, &tmp_path, true)
        .with_context(|| format!("[decompress_single]"))
        .with_context(|| {
            format!(
                "extracting file `{}` to `{}`",
                file_path.display(),
                tmp_path.display()
            )
        })?;

    for content_path in read_dir(&tmp_path)? {
        let content_path = content_path?.path();

        rename(
            &content_path,
            path.join(content_path.strip_prefix(&tmp_path).unwrap()),
        )?;
    }

    remove_dir_all(&tmp_path).ok();
    remove_file(&file_path).ok();

    Ok(())
}

async fn decompress_multiple(
    product_files: &DLsiteProductFiles,
    path: impl AsRef<Path>,
) -> Result<(), DownloadServiceError> {
    use std::fs::*;
    use unrar::Archive;

    let path = path.as_ref();
    let rar_file_name = path
        .join(&product_files.files[0].file_name)
        .with_extension("rar");

    rename(path.join(&product_files.files[0].file_name), &rar_file_name)?;

    let tmp_path = path.join("__tmp__");
    let mut archive = Archive::new(&rar_file_name).open_for_processing()?;

    while let Some(header) = archive.read_header()? {
        archive = header.extract_with_base(&tmp_path)?;
    }

    rename(&rar_file_name, path.join(&product_files.files[0].file_name))?;

    let mut content_paths = read_dir(&tmp_path)?.collect::<std::io::Result<Vec<_>>>()?;
    let content_prefix_path;

    if content_paths.len() == 1 && content_paths[0].file_type()?.is_dir() {
        content_prefix_path = content_paths[0].path();
        content_paths = read_dir(content_paths[0].path())?.collect::<std::io::Result<Vec<_>>>()?;
    } else {
        content_prefix_path = tmp_path.clone();
    }

    for content_path in content_paths {
        let content_path = content_path.path();

        rename(
            &content_path,
            path.join(content_path.strip_prefix(&content_prefix_path).unwrap()),
        )
        .ok();
    }

    remove_dir_all(&tmp_path).ok();

    for file in &product_files.files {
        remove_file(&path.join(&file.file_name)).ok();
    }

    Ok(())
}
