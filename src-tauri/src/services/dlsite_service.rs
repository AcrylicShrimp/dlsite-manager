use crate::{
    database::{
        models::v2::CreatingProduct,
        tables::v2::{AccountTable, DBError, ProductTable},
    },
    dlsite::{
        api::{get_product_count, get_products, login, test_cookie_store, LoginError},
        dto::DLsiteProduct,
    },
};
use anyhow::{anyhow, Error as AnyError};
use log::{debug, warn};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{io::BufWriter, sync::Arc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DLsiteServiceError {
    #[error("the given account id `{id}` is not valid")]
    InvalidAccountId { id: i64 },
    #[error("the given username `{username}` or password `{password}` is invalid")]
    InvalidCredentials { username: String, password: String },
    #[error("{0:?}")]
    DBError(#[from] DBError),
    #[error("{0:?}")]
    AnyError(#[from] AnyError),
}

pub struct DLsiteService;

impl DLsiteService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_cookie_store(
        &self,
        account_id: i64,
    ) -> Result<Arc<CookieStoreMutex>, DLsiteServiceError> {
        debug!(
            "[get_cookie_store] fetching cookie store of the account id `{}`",
            account_id
        );

        let account = match AccountTable::get_one(account_id)? {
            Some(account) => account,
            None => {
                warn!(
                    "[get_cookie_store] invalid account id detected: {}",
                    account_id
                );
                return Err(DLsiteServiceError::InvalidAccountId { id: account_id });
            }
        };

        debug!("[get_cookie_store] account: {:#?}", account);

        match CookieStore::load_json(account.cookie_json.as_bytes()) {
            Ok(cookies) => {
                debug!("[get_cookie_store] successfully parsed cookie_json of the account");
                let cookie_store = Arc::new(CookieStoreMutex::new(cookies));
                let is_valid = test_cookie_store(cookie_store.clone()).await?;

                if is_valid {
                    return Ok(cookie_store);
                }

                debug!("[get_cookie_store] the parsed cookie_json is invalid");
            }
            Err(err) => {
                debug!(
                    "[get_cookie_store] failed to parse cookie_json of the account: {:?}",
                    err
                );
            }
        }

        debug!("[get_cookie_store] fresh cookie_json is needed; logging in");

        match login(&account.username, &account.password).await {
            Ok(cookie_store) => {
                update_cookie_json(account_id, &cookie_store);
                Ok(cookie_store)
            }
            Err(err) => match err {
                LoginError::WrongCredentials => {
                    debug!("[get_cookie_store] invalid credentials: {:?}", account);
                    return Err(DLsiteServiceError::InvalidCredentials {
                        username: account.username.clone(),
                        password: account.password.clone(),
                    });
                }
                LoginError::Other(err) => {
                    warn!("[get_cookie_store] error occurred: {:?}", err);
                    return Err(DLsiteServiceError::AnyError(err));
                }
            },
        }
    }

    pub async fn get_product_count(
        &self,
        account_id: i64,
        cookie_store: Arc<CookieStoreMutex>,
    ) -> Result<u32, DLsiteServiceError> {
        debug!(
            "[get_product_count] fetching product count of the account id `{}`",
            account_id
        );

        let product_count = get_product_count(cookie_store.clone()).await?;
        update_cookie_json(account_id, &cookie_store);
        Ok(product_count)
    }

    pub async fn fetch_new_products(
        &self,
        mut on_progress: impl FnMut(u32, u32),
    ) -> Result<(), DLsiteServiceError> {
        debug!("[fetch_new_products] fetching new products for all accounts");

        struct AccountDetail {
            pub account_id: i64,
            pub cookie_store: Arc<CookieStoreMutex>,
            pub prev_product_count: u32,
            pub new_product_count: u32,
        }

        let accounts = AccountTable::get_all()?;
        let mut account_details = Vec::with_capacity(accounts.len());

        let mut progress = 0;
        let mut total_progress = 0;

        for account in &accounts {
            debug!(
                "[fetch_new_products] fetching account detail of the account id `{}`",
                account.id
            );

            let cookie_store = self.get_cookie_store(account.id).await?;
            let prev_product_count = account.product_count as u32;
            let new_product_count = self
                .get_product_count(account.id, cookie_store.clone())
                .await?;

            debug!(
                "[fetch_new_products] the account id `{}` has {} product(s) before, now has {} product(s)",
                account.id,
                prev_product_count,
                new_product_count
            );

            if new_product_count < prev_product_count {
                warn!("[fetch_new_products] the account id `{}` has fewer product(s) then before; this account will be ignored", account.id);
            }

            total_progress += new_product_count - prev_product_count;

            account_details.push(AccountDetail {
                account_id: account.id,
                cookie_store,
                prev_product_count,
                new_product_count,
            });
        }

        if total_progress == 0 {
            debug!("[fetch_new_products] nothing to update");
            return Ok(());
        }

        on_progress(progress, total_progress);

        for mut detail in account_details {
            const PAGE_LIMIT: u32 = 50;

            // The first page may contain some products that are already fetched before.
            let mut already_fetched_product_count =
                (detail.prev_product_count % PAGE_LIMIT) as usize;

            while detail.prev_product_count < detail.new_product_count {
                let page = 1 + detail.prev_product_count / PAGE_LIMIT;
                let products = match get_products(detail.cookie_store.clone(), page).await {
                    Ok(products) => products,
                    Err(err) => {
                        warn!("[fetch_new_products] failed to fetch products of {} page of the account id `{}`: {:?}", page, detail.account_id, err);
                        break;
                    }
                };
                let products = &products[already_fetched_product_count..];

                progress += products.len() as u32;
                detail.prev_product_count += products.len() as u32;

                if let Err(err) = AccountTable::update_one_product_count(
                    detail.account_id,
                    detail.prev_product_count as i32,
                ) {
                    warn!(
                        "[fetch_new_products] failed to update the product count of the account id `{}` to the database: {:?}",
                        detail.account_id,
                        err
                    );
                    break;
                }

                if let Err(err) = ProductTable::insert_many(
                    products
                        .into_iter()
                        .map(|product| make_creating_product(detail.account_id, product)),
                ) {
                    warn!("[fetch_new_products] failed to update the products from the account id `{}` to the database: {:?}", detail.account_id, err);
                    break;
                }

                on_progress(progress, total_progress);

                // first page is over
                already_fetched_product_count = 0;
            }
        }

        Ok(())
    }

    pub async fn refresh_products_all(&self) -> Result<(), DLsiteServiceError> {}
}

fn update_cookie_json(account_id: i64, cookie_store: &CookieStoreMutex) {
    debug!(
        "[update_cookie_json] updating cookie_json of the account id `{}`",
        account_id
    );

    match serialize_cookie_store(&cookie_store) {
        Ok(serialized) => {
            if let Err(err) = AccountTable::update_one_cookie_json(account_id, &serialized) {
                warn!(
                    "[update_cookie_json] failed to update the cookie_json of the account id `{}` to the database: {:?}",
                    account_id,
                    err
                );
            }
        }
        Err(err) => {
            warn!(
                "[update_cookie_json] failed to serialize the cookie store: {:?}",
                err
            );
        }
    }
}

fn serialize_cookie_store(cookie_store: &CookieStoreMutex) -> Result<String, AnyError> {
    let mut writer = BufWriter::new(Vec::new());

    let cookie_store_guard = cookie_store
        .lock()
        .map_err(|_| anyhow!("the cookie store mutex is poisoned"))?;

    cookie_store_guard
        .save_json(&mut writer)
        .map_err(|err| anyhow!("failed to serialize the cookie store: {:?}", err));

    drop(cookie_store_guard);

    let content = writer
        .into_inner()
        .map_err(|err| anyhow!("failed to access writer content: {:?}", err))?;
    let content_str = std::str::from_utf8(&content)
        .map_err(|err| anyhow!("the content is not valid UTF-8 string: {:?}", err))?;

    Ok(content_str.to_owned())
}

fn make_creating_product(account_id: i64, product: &DLsiteProduct) -> CreatingProduct {
    CreatingProduct {
        id: &product.id,
        account_id: Some(account_id),
        ty: product.ty.clone(),
        age: product.age.clone(),
        title: &product.title,
        thumbnail: &product.thumbnail,
        group_id: &product.group_id,
        group_name: &product.group_name,
        registered_at: product.registered_at,
    }
}
