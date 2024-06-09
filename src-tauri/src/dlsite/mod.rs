use crate::database::tables::v2::{AccountTable, ProductTable};
use anyhow::Result;
use reqwest::ClientBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{
    io::BufWriter,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

pub mod api;
pub mod dto;

static PAGE_LIMIT: usize = 50;

pub async fn update_product(mut on_progress: impl FnMut(usize, usize) -> Result<()>) -> Result<()> {
    let accounts = AccountTable::get_all()?;
    let mut progress = 0;
    let mut total_progress = 0;
    let mut details = Vec::with_capacity(accounts.len());

    for account in &accounts {
        let prev_product_count = account.product_count as usize;
        let (new_product_count, cookie_store) =
            match get_product_count_and_cookie_store(account.id).await {
                Ok(product_count_and_cookie_store) => product_count_and_cookie_store,
                Err(err) => match err {
                    Error::DLsiteNotAuthenticated => continue,
                    _ => return Err(err),
                },
            };

        if new_product_count <= prev_product_count {
            continue;
        }

        total_progress += new_product_count - prev_product_count;
        details.push((
            account.id,
            prev_product_count,
            new_product_count,
            cookie_store,
        ));
    }

    if total_progress == 0 {
        return Ok(());
    }

    on_progress(progress, total_progress)?;

    for (account_id, mut prev_product_count, new_product_count, cookie_store) in details {
        while prev_product_count < new_product_count {
            let page = 1 + prev_product_count / PAGE_LIMIT;
            let products = match api::get_product(cookie_store.clone(), page).await {
                Ok(products) => products,
                Err(err) => match err {
                    Error::DLsiteNotAuthenticated => {
                        progress += new_product_count - prev_product_count;
                        on_progress(progress, total_progress)?;
                        break;
                    }
                    _ => return Err(err),
                },
            };

            let updated_prev_product_count = (page - 1) * PAGE_LIMIT + products.len();
            progress += updated_prev_product_count - prev_product_count;
            prev_product_count = updated_prev_product_count;

            on_progress(progress, total_progress)?;

            // TODO: use v2 api.
            // TODO: insert all products into the database.

            // ProductTable::insert_all(products.into_iter().map(|product| InsertedProduct {
            //     account_id,
            //     product,
            // }))?;
        }
    }

    Ok(())
}

pub async fn refresh_product(
    mut on_progress: impl FnMut(usize, usize) -> Result<()>,
) -> Result<()> {
    ProductTable::remove_many_owned()?;

    let accounts = AccountTable::get_all()?;
    let mut progress = 0;
    let mut total_progress = 0;
    let mut details = Vec::with_capacity(accounts.len());

    for account in accounts {
        let (new_product_count, cookie_store) =
            match get_product_count_and_cookie_store(account.id).await {
                Ok(product_count_and_cookie_store) => product_count_and_cookie_store,
                Err(err) => match err {
                    Error::DLsiteNotAuthenticated => continue,
                    _ => return Err(err),
                },
            };

        if new_product_count == 0 {
            continue;
        }

        total_progress += new_product_count;
        details.push((account, new_product_count, cookie_store));
    }

    if total_progress == 0 {
        return Ok(());
    }

    on_progress(progress, total_progress)?;

    for (account_id, new_product_count, cookie_store) in details {
        let mut prev_product_count = 0;

        while prev_product_count < new_product_count {
            let page = 1 + prev_product_count / PAGE_LIMIT;
            let products = match api::get_product(cookie_store.clone(), page).await {
                Ok(products) => products,
                Err(err) => match err {
                    Error::DLsiteNotAuthenticated => {
                        progress += new_product_count - prev_product_count;
                        on_progress(progress, total_progress)?;
                        break;
                    }
                    _ => return Err(err),
                },
            };
            prev_product_count += products.len();
            progress += products.len();

            on_progress(progress, total_progress)?;

            // TODO: use v2 api.
            // TODO: insert all products into the database.

            // ProductTable::insert_all(products.into_iter().map(|product| InsertedProduct {
            //     account_id,
            //     product,
            // }))?;
        }
    }

    Ok(())
}

pub async fn download_product(
    decompress: bool,
    account_id: i64,
    product_id: impl AsRef<str>,
    base_path: impl AsRef<Path>,
    on_progress: impl Fn(u64, u64) -> Result<()>,
) -> Result<PathBuf> {
    let files = api::get_product_files(product_id.as_ref()).await?.files;
    let file_size = files
        .iter()
        .fold(0, |acc, file| acc + file.file_size.parse::<u64>().unwrap());
    let file_urls;

    match files.len() {
        1 => {
            file_urls = vec![format!(
                "https://www.dlsite.com/maniax/download/=/product_id/{}.html",
                product_id.as_ref()
            )];
        }
        len => {
            file_urls = (1..=len)
                .map(|index| {
                    format!(
                        "https://www.dlsite.com/maniax/download/=/number/{}/product_id/{}.html",
                        index,
                        product_id.as_ref()
                    )
                })
                .collect()
        }
    }

    let path = base_path.as_ref().join(product_id.as_ref());

    if path.exists() {
        std::fs::remove_dir_all(&path)
            .map_err(|err| Error::ProductDirCreationError { io_error: err })?;
    }

    std::fs::create_dir_all(&path)
        .map_err(|err| Error::ProductDirCreationError { io_error: err })?;
    on_progress(0, file_size)?;

    let mut progress = 0;
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;

    for (index, file_url) in file_urls.into_iter().enumerate() {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join(&detail.contents[index].file_name))
            .map_err(|err| Error::ProductFileCreationError { io_error: err })?;
        let mut writer = BufWriter::with_capacity(1 * 1024 * 1024, file);
        let mut last_progress_time = Instant::now();
        let mut process_per_file = 0;

        'req: loop {
            let mut response = match client
                .get(&file_url)
                .header("range", format!("bytes={}-", process_per_file))
                .send()
                .await
            {
                Ok(response) => response,
                Err(_) => {
                    continue;
                }
            };

            while let Some(chunk) = match response.chunk().await {
                Ok(chunk) => chunk,
                Err(_) => {
                    continue 'req;
                }
            } {
                writer
                    .write_all(&chunk)
                    .map_err(|err| Error::ProductFileWriteError { io_error: err })?;
                progress += chunk.len();
                process_per_file += chunk.len();

                let now = Instant::now();

                if Duration::from_secs(1) <= now - last_progress_time {
                    last_progress_time = now;
                    on_progress(progress as u64, file_size)?;
                }
            }

            writer
                .flush()
                .map_err(|err| Error::ProductFileWriteError { io_error: err })?;
            break;
        }
    }

    on_progress(file_size, file_size)?;

    if !decompress {
        return Ok(path);
    }

    if detail.contents.len() == 1 && detail.contents[0].file_name.ends_with(".zip") {
        let tmp_path = path.join("__tmp__");
        let file_path = path.join(&detail.contents[0].file_name);
        let file = OpenOptions::new()
            .read(true)
            .open(&file_path)
            .map_err(|err| Error::ProductArchiveOpenError { io_error: err })?;
        let reader = BufReader::new(file);

        zip_extract::extract(reader, &tmp_path, true)
            .map_err(|err| Error::ProductArchiveExtractError { extract_error: err })?;

        remove_file(&file_path)
            .map_err(|err| Error::ProductArchiveDeleteError { io_error: err })?;

        for content_path in read_dir(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
        {
            let content_path = content_path
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
                .path();

            rename(
                &content_path,
                path.join(content_path.strip_prefix(&tmp_path).unwrap()),
            )
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        }

        remove_dir_all(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
    }

    if detail.contents.len() != 0 && detail.contents[0].file_name.ends_with(".exe") {
        let rar_filename = path
            .join(&detail.contents[0].file_name)
            .with_extension("rar");

        rename(path.join(&detail.contents[0].file_name), &rar_filename)
            .map_err(|err| Error::ProductRarArchiveRenameError { io_error: err })?;

        let tmp_path = path.join("__tmp__");
        let mut archive = Archive::new(
            &rar_filename
                .to_str()
                .ok_or_else(|| Error::NonUtf8PathError {
                    path: rar_filename.clone(),
                })?
                .to_owned(),
        )
        .open_for_processing()
        .map_err(|err| Error::ProductRarArchiveExtractOpenError { extract_error: err })?;

        while let Some(header) = archive
            .read_header()
            .map_err(|err| Error::ProductRarArchiveExtractProcessError { extract_error: err })?
        {
            archive = header.extract_with_base(&tmp_path).map_err(|err| {
                Error::ProductRarArchiveExtractProcessError { extract_error: err }
            })?;
        }

        rename(&rar_filename, path.join(&detail.contents[0].file_name))
            .map_err(|err| Error::ProductRarArchiveRenameError { io_error: err })?;

        for content in &detail.contents {
            remove_file(path.join(&content.file_name))
                .map_err(|err| Error::ProductArchiveDeleteError { io_error: err })?;
        }

        let mut content_paths = read_dir(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
            .collect::<IOResult<Vec<_>>>()
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        let content_prefix_path;

        if content_paths.len() == 1
            && content_paths[0]
                .file_type()
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
                .is_dir()
        {
            content_prefix_path = content_paths[0].path();
            content_paths = read_dir(content_paths[0].path())
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
                .collect::<IOResult<Vec<_>>>()
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        } else {
            content_prefix_path = tmp_path.clone();
        }

        for content_path in content_paths {
            let content_path = content_path.path();

            std::fs::rename(
                &content_path,
                path.join(content_path.strip_prefix(&content_prefix_path).unwrap()),
            )
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        }

        std::fs::remove_dir_all(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
    }

    Ok(path)
}

pub fn remove_downloaded_product(
    product_id: impl AsRef<str>,
    base_path: impl AsRef<Path>,
) -> Result<()> {
    let path = base_path.as_ref().join(product_id.as_ref());
    std::fs::remove_dir_all(&path)
        .map_err(|err| Error::ProductDirCreationError { io_error: err })?;
    Ok(())
}
