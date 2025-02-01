use super::dto::{
    DLsiteProduct, DLsiteProductFiles, DLsiteProductFromNonOwnerApi, DLsiteProductI18nString,
    DLsiteProductListFromOwnerApi, DLsiteVoiceComicRequestInfo, DLsiteVoiceComicZipTree,
};
use anyhow::{anyhow, Context, Error};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use lazy_static::lazy_static;
use reqwest::{Client, ClientBuilder};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::{
    fs::{create_dir_all, remove_dir_all, OpenOptions},
    io::{AsyncWriteExt, BufWriter},
};

lazy_static! {
    static ref GROUP_NAME_SELECTOR_STR: &'static str = "#work_maker>tbody>tr>td>span>a";
    // SAFETY: below selector is valid, so unwrap here is safe
    static ref GROUP_NAME_SELECTOR: scraper::Selector =
        scraper::Selector::parse(*GROUP_NAME_SELECTOR_STR).unwrap();
}

#[derive(Error, Debug)]
pub enum LoginError {
    #[error("wrong credentials")]
    WrongCredentials,
    #[error("{0}")]
    Other(#[from] Error),
}

pub async fn login(
    username: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<Arc<CookieStoreMutex>, LoginError> {
    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store.clone())
        .build()
        .with_context(|| "[login]")
        .with_context(|| "failed to create HTTP client")?;

    client
        .get("https://www.dlsite.com/maniax/login/=/skip_register/1")
        .send()
        .await
        .with_context(|| "[login]")
        .with_context(|| "request failed for `skip_register`")?;
    client
        .get("https://login.dlsite.com/login")
        .send()
        .await
        .with_context(|| "[login]")
        .with_context(|| "request failed for `fetch_initial_cookies`")?;

    let res = client
        .post("https://login.dlsite.com/login")
        .form(&[
            ("login_id", username.as_ref()),
            ("password", password.as_ref()),
            ("_token", &{
                let cookie = cookie_store
                    .lock()
                    .unwrap()
                    .get("login.dlsite.com", "/", "XSRF-TOKEN")
                    .ok_or_else(|| anyhow!("cookie `XSRF-TOKEN` not found"))?
                    .value()
                    .to_owned();
                cookie
            }),
        ])
        .send()
        .await
        .with_context(|| "[login]")
        .with_context(|| "request failed for `authenticate`")?;
    let text = res
        .text()
        .await
        .with_context(|| "[login]")
        .with_context(|| "parse failed for `authenticate`")?;

    if text.contains("ログインIDかパスワードが間違っています。") {
        return Err(LoginError::WrongCredentials);
    }

    Ok(cookie_store)
}

/// Tests the given cookie store. Returns `true` if the cookie store contains valid credentials, `false` otherwise.
pub async fn test_cookie_store(cookie_store: Arc<CookieStoreMutex>) -> Result<bool, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .with_context(|| "[get_product_count]")
        .with_context(|| "failed to create HTTP client")?;
    let res = client
        .get("https://play.dlsite.com/api/product_count")
        .send()
        .await
        .with_context(|| "[get_product_count]")
        .with_context(|| "request failed")?;

    let text = res.text().await?;

    Ok(!text.contains("status") && !text.contains("401"))
}

pub async fn get_product_count(cookie_store: Arc<CookieStoreMutex>) -> Result<u32, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .with_context(|| "[get_product_count]")
        .with_context(|| "failed to create HTTP client")?;
    let res = client
        .get("https://play.dlsite.com/api/product_count")
        .send()
        .await
        .with_context(|| "[get_product_count]")
        .with_context(|| "request failed")?;

    let product_count_map = res
        .json::<HashMap<String, u32>>()
        .await
        .with_context(|| "[get_product_count]")
        .with_context(|| "parse failed")?;

    product_count_map
        .get("user")
        .cloned()
        .ok_or_else(|| anyhow!("unable to get product count; `user` key not found in response"))
        .with_context(|| "[get_product_count]")
}

pub async fn get_products(
    cookie_store: Arc<CookieStoreMutex>,
    page: u32,
) -> Result<Vec<DLsiteProduct>, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .with_context(|| format!("[get_products]"))
        .with_context(|| format!("failed to create HTTP client for page `{}`", page))?;
    let url = format!("https://play.dlsite.com/api/purchases?page={}", page);
    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("[get_products]"))
        .with_context(|| format!("request failed for page `{}` with url: `{}`", page, url))?;
    let product_list = res
        .json::<DLsiteProductListFromOwnerApi>()
        .await
        .with_context(|| format!("[get_products]"))
        .with_context(|| format!("parse failed for page `{}`", page))?;

    fn get_localized_string(i18n: &DLsiteProductI18nString) -> Result<String, Error> {
        i18n.japanese
            .as_ref()
            .or_else(|| i18n.english.as_ref())
            .or_else(|| i18n.korean.as_ref())
            .or_else(|| i18n.taiwanese.as_ref())
            .or_else(|| i18n.chinese.as_ref())
            .cloned()
            .ok_or_else(|| anyhow!("localized string is empty"))
    }

    let product_list = product_list
        .works
        .into_iter()
        .map(|product| -> Result<_, Error> {
            Ok(DLsiteProduct {
                id: product.id.clone(),
                ty: product.ty,
                age: product.age,
                title: get_localized_string(&product.title)
                    .with_context(|| format!("mapping `title` of product id `{}`", product.id))?,
                thumbnail: product.icon.main,
                group_id: product.group.id,
                group_name: get_localized_string(&product.group.name).with_context(|| {
                    format!("mapping `group_name` of product id `{}`", product.id)
                })?,
                registered_at: product.registered_at,
            })
        })
        .collect::<Result<Vec<_>, Error>>()
        .with_context(|| format!("[get_products]"))
        .with_context(|| format!("mapping failed for page `{}`", page))?;

    Ok(product_list)
}

pub async fn get_product_from_non_owner_api(id: &str) -> Result<DLsiteProduct, Error> {
    let url = format!(
        "https://www.dlsite.com/maniax/api/=/product.json?workno={}",
        id
    );
    let res = reqwest::get(&url)
        .await
        .with_context(|| format!("[get_product_from_non_owner_api]"))
        .with_context(|| format!("request failed for product id `{}` with url: `{}`", id, url))?;
    let products = res
        .json::<Vec<DLsiteProductFromNonOwnerApi>>()
        .await
        .with_context(|| format!("[get_product_from_non_owner_api]"))
        .with_context(|| format!("parse failed for product id `{}`", id))?;

    if products.is_empty() {
        return Err(anyhow!("product list is empty"));
    }

    let product = products.into_iter().next().unwrap();
    let utc_registered_at = match product.registered_at {
        Some(registered_at) => {
            let naive_registered_at =
                NaiveDateTime::parse_from_str(&registered_at, "%Y-%m-%d %H:%M:%S")?;
            let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap();
            let jst_registered_at = jst_offset
                .from_local_datetime(&naive_registered_at)
                .single()
                .unwrap();
            Some(jst_registered_at.to_utc())
        }
        None => None,
    };

    Ok(DLsiteProduct {
        id: id.to_owned(),
        ty: product.ty,
        age: product.age,
        title: product.title,
        thumbnail: if product.image.url.starts_with("http") {
            product.image.url
        } else {
            format!("https:{}", product.image.url)
        },
        group_id: product.group_id,
        group_name: product.group_name,
        registered_at: utc_registered_at,
    })
}

pub async fn get_product_files(id: &str) -> Result<DLsiteProductFiles, Error> {
    let url = format!(
        "https://www.dlsite.com/maniax/api/=/product.json?workno={}",
        id
    );
    let res = reqwest::get(&url)
        .await
        .with_context(|| format!("[get_product_files]"))
        .with_context(|| format!("request failed for product id `{}` with url: `{}`", id, url))?;

    let product_details_list = res
        .json::<Vec<DLsiteProductFiles>>()
        .await
        .with_context(|| format!("[get_product_files]"))
        .with_context(|| format!("parse failed for product id `{}`", id))?;

    if product_details_list.is_empty() {
        return Err(anyhow!("product details list is empty"));
    }

    Ok(product_details_list.into_iter().next().unwrap())
}

pub async fn get_voice_comic_request_info(
    cookie_store: Arc<CookieStoreMutex>,
    id: &str,
) -> Result<DLsiteVoiceComicRequestInfo, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .with_context(|| format!("[get_voice_comic_request_info]"))
        .with_context(|| format!("failed to create HTTP client for id `{}`", id))?;
    let url = format!(
        "https://play.dl.dlsite.com/api/download/sign/cookie?workno={}",
        id
    );
    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("[get_voice_comic_request_info]"))
        .with_context(|| format!("request failed for id `{}` with url: `{}`", id, url))?;
    let request_info = res
        .json::<DLsiteVoiceComicRequestInfo>()
        .await
        .with_context(|| format!("[get_voice_comic_request_info]"))
        .with_context(|| format!("parse failed for id `{}`", id))?;

    Ok(request_info)
}

pub async fn get_voice_comic_zip_tree(
    cookie_store: Arc<CookieStoreMutex>,
    request_info: &DLsiteVoiceComicRequestInfo,
) -> Result<DLsiteVoiceComicZipTree, Error> {
    request_info.apply_cookies("https://play.dl.dlsite.com", cookie_store.clone());

    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()
        .with_context(|| format!("[get_voice_comic_zip_tree]"))
        .with_context(|| {
            format!(
                "failed to create HTTP client for request_info url `{}`",
                &request_info.url
            )
        })?;
    let url = format!("{}ziptree.json", &request_info.url);
    let res = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("[get_voice_comic_zip_tree]"))
        .with_context(|| {
            format!(
                "request failed for request_info url `{}` with url: `{}`",
                &request_info.url, url
            )
        })?;
    let zip_tree = res
        .json::<DLsiteVoiceComicZipTree>()
        .await
        .with_context(|| format!("[get_voice_comic_zip_tree]"))
        .with_context(|| {
            format!(
                "parse failed for request_info url `{}` with url: `{}`",
                &request_info.url, url
            )
        })?;

    Ok(zip_tree)
}

// download vtt from: https://play.dlsite.fun/work/<PRODUCT-ID>/<HASH>.vtt
// download mp4 from: https://play.dlsite.fun/work/<PRODUCT-ID>/<HASH>.mp4

pub async fn download_product_files(
    cookie_store: Arc<CookieStoreMutex>,
    id: &str,
    product_files: &DLsiteProductFiles,
    base_path: impl AsRef<Path>,
    on_progress: impl Fn(u64, u64),
) -> Result<(), Error> {
    let total_file_size = product_files.files.iter().fold(0, |acc, detail| {
        acc + detail.file_size.parse::<u64>().unwrap()
    });
    let file_urls = resolve_file_urls(id, product_files);
    let target_path = prepare_target_path(id, base_path).await?;

    let last_callback_time = AtomicU64::new(0);
    let progress = AtomicU64::new(0);
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;

    let on_chunk_received = |chunk_received| {
        let prev_progress = progress.fetch_add(chunk_received, Ordering::SeqCst);

        let now = UNIX_EPOCH.elapsed().unwrap().as_secs();

        if now == last_callback_time.swap(now, Ordering::SeqCst) {
            return;
        }

        on_progress(prev_progress + chunk_received, total_file_size);
    };

    let results =
        futures::future::try_join_all(file_urls.iter().enumerate().map(|(index, file_url)| {
            download_single_file(
                &client,
                file_url,
                &target_path,
                &product_files.files[index].file_name,
                on_chunk_received,
            )
        }))
        .await;

    if let Err(err) = results {
        // ignore errors occurred during cleanup
        remove_dir_all(&target_path).await.ok();

        return Err(err)
            .with_context(|| format!("[download_product_files]"))
            .with_context(|| {
                format!("failed to download product files for product id `{}`", id)
            })?;
    }

    Ok(())
}

fn resolve_file_urls(id: &str, product_files: &DLsiteProductFiles) -> Vec<String> {
    match product_files.files.len() {
        0 => vec![],
        1 => vec![format!(
            "https://www.dlsite.com/maniax/download/=/product_id/{}.html",
            id
        )],
        len => (1..=len)
            .map(|index| {
                format!(
                    "https://www.dlsite.com/maniax/download/=/number/{}/product_id/{}.html",
                    index, id
                )
            })
            .collect(),
    }
}

pub async fn download_voice_comic(
    cookie_store: Arc<CookieStoreMutex>,
    id: &str,
    request_info: &DLsiteVoiceComicRequestInfo,
    zip_tree: &DLsiteVoiceComicZipTree,
    base_path: impl AsRef<Path>,
    on_progress: impl Fn(u64, u64),
) -> Result<(), Error> {
    request_info.apply_cookies("https://play.dlsite.fun", cookie_store.clone());

    struct VoiceComicFile {
        pub name: String,
        pub file_name: String,
    }

    let mut files = Vec::new();

    for item in &zip_tree.items {
        if item.ty != "file" {
            continue;
        }

        let voice_comic = match zip_tree.play_files.get(&item.play_file_key) {
            Some(voice_comic) => voice_comic,
            None => {
                continue;
            }
        };
        let voice_comic = match voice_comic.items.get(&voice_comic.ty) {
            Some(voice_comic) => voice_comic,
            None => {
                continue;
            }
        };
        let name = &voice_comic.optimized.name;
        let (is_mp4, file_name) = match name.rsplit_once('.') {
            Some((_, extension)) => {
                let is_mp4 = extension.eq_ignore_ascii_case("mp4");

                match item.name.rsplit_once(".") {
                    Some((file_name, _)) => (is_mp4, format!("{}.{}", file_name, extension)),
                    None => (is_mp4, item.name.to_owned()),
                }
            }
            None => (false, item.name.to_owned()),
        };

        files.push(VoiceComicFile {
            name: name.clone(),
            file_name,
        });

        if is_mp4 {
            if let Some((hash, _)) = name.rsplit_once(".") {
                let name = format!("{}.vtt", hash);
                let file_name = match item.name.rsplit_once(".") {
                    Some((file_name, _)) => format!("{}.vtt", file_name),
                    None => item.name.clone(),
                };
                files.push(VoiceComicFile { name, file_name });
            }
        }
    }

    let file_urls = files
        .iter()
        .map(|file| format!("https://play.dlsite.fun/work/{}/{}", id, &file.name))
        .collect::<Vec<_>>();

    let target_path = prepare_target_path(id, base_path).await?;

    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;

    on_progress(1, 2);

    let results =
        futures::future::try_join_all(file_urls.iter().enumerate().map(|(index, file_url)| {
            download_single_file(
                &client,
                file_url,
                &target_path,
                &files[index].file_name,
                |_| {},
            )
        }))
        .await;

    if let Err(err) = results {
        // ignore errors occurred during cleanup
        remove_dir_all(&target_path).await.ok();

        return Err(err)
            .with_context(|| format!("[download_product_files]"))
            .with_context(|| {
                format!("failed to download product files for product id `{}`", id)
            })?;
    }

    Ok(())
}

async fn prepare_target_path(id: &str, base_path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let target_path = base_path.as_ref().join(id);

    if target_path.exists() {
        remove_dir_all(&target_path)
            .await
            .with_context(|| format!("[prepare_target_path]"))
            .with_context(|| {
                format!(
                    "failed to cleanup existing target path `{}`",
                    target_path.display()
                )
            })?;
    }

    create_dir_all(&target_path)
        .await
        .with_context(|| format!("[prepare_target_path]"))
        .with_context(|| format!("failed to create target path `{}`", target_path.display()))?;

    Ok(target_path)
}

async fn download_single_file(
    client: &Client,
    url: &str,
    target_path: impl AsRef<Path>,
    file_name: &str,
    mut on_chunk_received: impl FnMut(u64),
) -> Result<(), Error> {
    let file_path = target_path.as_ref().join(file_name);
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
        .await
        .with_context(|| format!("[download_single_file]"))
        .with_context(|| format!("failed to open file `{}`", file_path.display()))?;

    let mut writer = BufWriter::with_capacity(1 * 1024 * 1024, file);
    let mut total_chunk_received: u64 = 0;
    let mut retry_count = 0;
    const MAX_RETRY_COUNT: u32 = 3;

    'req: loop {
        let mut res = match client
            .get(url)
            .header("range", format!("bytes={}-", total_chunk_received))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                retry_count += 1;

                if MAX_RETRY_COUNT < retry_count {
                    return Err(anyhow!("max retry count reached").context(err))
                        .with_context(|| format!("[download_single_file]"))
                        .with_context(|| {
                            format!("failed to download file `{}`", file_path.display())
                        })?;
                }

                // wait for 5 seconds
                tokio::time::sleep(Duration::from_secs(5)).await;

                continue;
            }
        };

        while let Some(chunk) = match res.chunk().await {
            Ok(chunk) => chunk,
            Err(_) => {
                continue 'req;
            }
        } {
            writer
                .write_all(&chunk)
                .await
                .with_context(|| format!("[download_single_file]"))
                .with_context(|| {
                    format!("failed to write chunk to file `{}`", file_path.display())
                })?;
            total_chunk_received += chunk.len() as u64;
            on_chunk_received(chunk.len() as u64);
        }

        writer
            .flush()
            .await
            .with_context(|| format!("[download_single_file]"))
            .with_context(|| format!("failed to flush file `{}`", file_path.display()))?;

        break;
    }

    Ok(())
}
