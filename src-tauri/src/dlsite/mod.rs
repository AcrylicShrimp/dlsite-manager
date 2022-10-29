pub mod api;

use crate::{
    application_error::{Error, Result},
    storage::{
        account::Account,
        product::{InsertedProduct, Product},
    },
};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{io::BufWriter, sync::Arc};

static PAGE_LIMIT: usize = 50;

async fn get_product_count_and_cookie_store(
    account_id: i64,
) -> Result<(usize, Arc<CookieStoreMutex>)> {
    let cookie_json = if let Some(cookie_json) = Account::get_one_cookie_json(account_id)? {
        cookie_json
    } else {
        return Err(Error::AccountNotExists { account_id });
    };

    if let Ok(cookie_store) = CookieStore::load_json(cookie_json.as_bytes()) {
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        match api::get_product_count(cookie_store.clone()).await {
            Ok(product_count) => {
                Account::update_one_product_count_and_cookie_json(
                    account_id,
                    product_count as i32,
                    {
                        let mut writer = BufWriter::new(Vec::new());
                        cookie_store
                            .lock()
                            .unwrap()
                            .save_json(&mut writer)
                            .map_err(|err| Error::ReqwestCookieStoreError {
                                reqwest_cookie_store_error: err,
                            })?;
                        String::from_utf8(writer.into_inner().unwrap()).unwrap()
                    },
                )?;
                return Ok((product_count, cookie_store));
            }
            Err(err) => match err {
                Error::DLsiteNotAuthenticated => {}
                _ => return Err(err),
            },
        }
    }

    let (username, password) =
        if let Some(username_and_password) = Account::get_one_username_and_password(account_id)? {
            username_and_password
        } else {
            return Err(Error::AccountNotExists { account_id });
        };
    let cookie_store = api::login(username, password).await?;

    let product_count = api::get_product_count(cookie_store.clone()).await?;
    Account::update_one_product_count_and_cookie_json(account_id, product_count as i32, {
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

    Ok((product_count, cookie_store))
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
            get_product_count_and_cookie_store(account_id).await?;

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
            let products = api::get_product(cookie_store.clone(), page).await?;
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
            get_product_count_and_cookie_store(account_id).await?;

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
            let products = api::get_product(cookie_store.clone(), page).await?;
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
