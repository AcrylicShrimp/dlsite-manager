pub mod api;

use crate::{
    application_error::{Error, Result},
    dlsite::api::DLsiteProductDetail,
    storage::{
        account::Account,
        product::{InsertedProduct, Product},
    },
};
use reqwest::ClientBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{
    fs::{create_dir_all, read_dir, remove_dir_all, remove_file, rename, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

static PAGE_LIMIT: usize = 50;

macro_rules! with_cookie_store {
    ($account_id:ident, $f:ident) => {
        let cookie_json = if let Some(cookie_json) = Account::get_one_cookie_json($account_id)? {
            cookie_json
        } else {
            return Err(Error::AccountNotExists { $account_id });
        };

        if let Ok(cookie_store) = CookieStore::load_json(cookie_json.as_bytes()) {
            match $f(Arc::new(CookieStoreMutex::new(cookie_store))).await {
                Ok(result) => {
                    Account::update_one_cookie_json($account_id, cookie_json)?;
                    return Ok(result);
                }
                Err(err) => match err {
                    Error::DLsiteNotAuthenticated => {}
                    _ => return Err(err),
                },
            }
        }

        let (username, password) = if let Some(username_and_password) =
            Account::get_one_username_and_password($account_id)?
        {
            username_and_password
        } else {
            return Err(Error::AccountNotExists { $account_id });
        };
        let cookie_store = api::login(username, password).await?;

        match $f(cookie_store.clone()).await {
            Ok(result) => {
                Account::update_one_cookie_json($account_id, {
                    let mut writer = BufWriter::new(Vec::new());
                    cookie_store
                        .lock()
                        .unwrap()
                        .save_json(&mut writer)
                        .map_err(|err| Error::ReqwestCookieStoreError {
                            reqwest_cookie_store_error: err,
                        })?;
                    String::from_utf8(writer.into_inner().unwrap()).unwrap()
                })?;
                return Ok(result);
            }
            Err(err) => return Err(err),
        }
    };
}

async fn get_product_count_and_cookie_store(
    account_id: i64,
) -> Result<(usize, Arc<CookieStoreMutex>)> {
    async fn body(
        account_id: i64,
        cookie_store: Arc<CookieStoreMutex>,
    ) -> Result<(usize, Arc<CookieStoreMutex>)> {
        let product_count = api::get_product_count(cookie_store.clone()).await?;
        Account::update_one_product_count(account_id, product_count as i32)?;
        Ok((product_count, cookie_store))
    }

    let body = move |cookie_store: Arc<CookieStoreMutex>| body(account_id, cookie_store);

    with_cookie_store!(account_id, body);
}

async fn get_product_details_and_cookie_store(
    account_id: i64,
    product_id: impl AsRef<str>,
) -> Result<(Vec<DLsiteProductDetail>, Arc<CookieStoreMutex>)> {
    async fn body(
        product_id: impl AsRef<str>,
        cookie_store: Arc<CookieStoreMutex>,
    ) -> Result<(Vec<DLsiteProductDetail>, Arc<CookieStoreMutex>)> {
        Ok((
            api::get_product_details(cookie_store.clone(), product_id).await?,
            cookie_store,
        ))
    }

    let body = |cookie_store: Arc<CookieStoreMutex>| body(product_id.as_ref(), cookie_store);

    with_cookie_store!(account_id, body);
}

pub async fn update_product(mut on_progress: impl FnMut(usize, usize) -> Result<()>) -> Result<()> {
    let account_ids = Account::list_all_id()?;
    let mut progress = 0;
    let mut total_progress = 0;
    let mut details = Vec::with_capacity(account_ids.len());

    for account_id in account_ids {
        let prev_product_count =
            Account::get_one_product_count(account_id)?.unwrap_or_else(|| 0) as usize;
        let (new_product_count, cookie_store) =
            match get_product_count_and_cookie_store(account_id).await {
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
            account_id,
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
            prev_product_count += products.len();
            progress += products.len();

            on_progress(progress, total_progress)?;

            Product::insert_all(products.into_iter().map(|product| InsertedProduct {
                account_id,
                product,
            }))?;
        }
    }

    Ok(())
}

pub async fn refresh_product(
    mut on_progress: impl FnMut(usize, usize) -> Result<()>,
) -> Result<()> {
    Product::remove_all()?;

    let account_ids = Account::list_all_id()?;
    let mut progress = 0;
    let mut total_progress = 0;
    let mut details = Vec::with_capacity(account_ids.len());

    for account_id in account_ids {
        let (new_product_count, cookie_store) =
            match get_product_count_and_cookie_store(account_id).await {
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
        details.push((account_id, new_product_count, cookie_store));
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

            Product::insert_all(products.into_iter().map(|product| InsertedProduct {
                account_id,
                product,
            }))?;
        }
    }

    Ok(())
}

pub async fn download_product(
    account_id: i64,
    product_id: impl AsRef<str>,
    base_path: impl AsRef<Path>,
    on_progress: impl Fn(u64, u64) -> Result<()>,
) -> Result<PathBuf> {
    let (details, cookie_store) =
        get_product_details_and_cookie_store(account_id, product_id.as_ref()).await?;

    if details.len() != 1 {
        return Err(Error::DLsiteProductDetailMissingOrNotUnique);
    }

    let detail = details.into_iter().next().unwrap();
    let file_size = detail.contents.iter().fold(0, |acc, detail| {
        acc + detail.file_size.parse::<u64>().unwrap()
    });
    let file_urls;

    match detail.contents.len() {
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
        remove_dir_all(&path).map_err(|err| Error::ProductDirCreationError { io_error: err })?;
    }

    create_dir_all(&path).map_err(|err| Error::ProductDirCreationError { io_error: err })?;
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
        let mut writer = BufWriter::new(file);
        let mut response = client.get(file_url).send().await?;

        while let Some(chunk) = response.chunk().await? {
            writer
                .write_all(&chunk)
                .map_err(|err| Error::ProductFileWriteError { io_error: err })?;
            progress += chunk.len();

            on_progress(progress as u64, file_size)?;
        }
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

    #[cfg(target_family = "windows")]
    if detail.contents.len() != 0 && detail.contents[0].file_name.ends_with(".exe") {
        use std::{io::Result as IOResult, process::Command};

        let tmp_path = path.join("__tmp__");
        let file_path = path.join(&detail.contents[0].file_name);
        let sfx_output = Command::new(file_path)
            .args(["-s2", "-d__tmp__"])
            .current_dir(&path)
            .spawn()
            .map_err(|err| Error::ProductSfxExtractError { io_error: err })?
            .wait_with_output()
            .map_err(|err| Error::ProductSfxExtractError { io_error: err })?;

        if !sfx_output.status.success() {
            return Err(Error::ProductSfxExtractFailed {
                std_err: String::from_utf8_lossy(&sfx_output.stderr).into_owned(),
            });
        }

        let mut content_paths = read_dir(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
            .collect::<IOResult<Vec<_>>>()
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;

        if content_paths.len() == 1
            && content_paths[0]
                .file_type()
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
                .is_dir()
        {
            content_paths = read_dir(content_paths[0].path())
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?
                .collect::<IOResult<Vec<_>>>()
                .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        }

        for content_path in content_paths {
            let content_path = content_path.path();

            rename(
                &content_path,
                path.join(content_path.strip_prefix(&tmp_path).unwrap()),
            )
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
        }

        remove_dir_all(&tmp_path)
            .map_err(|err| Error::ProductArchiveCleanupError { io_error: err })?;
    }

    Ok(path)
}
