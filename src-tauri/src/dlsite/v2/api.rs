use super::dto::{
    DLsiteProduct, DLsiteProductFromNonOwnerApi, DLsiteProductI18nString,
    DLsiteProductListFromOwnerApi,
};
use anyhow::{anyhow, Context, Error};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use lazy_static::lazy_static;
use reqwest::ClientBuilder;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{collections::HashMap, sync::Arc};
use tokio::try_join;

lazy_static! {
    // SAFETY: below selector is valid, so unwrap here is safe
    static ref GROUP_NAME_SELECTOR: scraper::Selector =
        scraper::Selector::parse("#work_maker>tbody>tr>td>span>a").unwrap();
}

pub async fn login(
    username: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<Arc<CookieStoreMutex>, Error> {
    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store.clone())
        .build()?;

    client
        .get("https://www.dlsite.com/maniax/login/=/skip_register/1")
        .send()
        .await?;
    client.get("https://login.dlsite.com/login").send().await?;

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
        .await?;
    let text = res.text().await?;

    if text.contains("ログインIDかパスワードが間違っています。") {
        return Err(anyhow!("login failed"));
    }

    Ok(cookie_store)
}

pub async fn get_product_count(cookie_store: Arc<CookieStoreMutex>) -> Result<u32, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;
    let res = client
        .get("https://play.dlsite.com/api/product_count")
        .send()
        .await?;

    match res.json::<HashMap<String, u32>>().await {
        Ok(product_count) => Ok(product_count.get("user").cloned().unwrap_or(0)),
        Err(err) => Err(anyhow!(
            "unable to get product count; parse failed: {}",
            err
        )),
    }
}

pub async fn get_products(
    cookie_store: Arc<CookieStoreMutex>,
    page: u32,
) -> Result<Vec<DLsiteProduct>, Error> {
    let client = ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store)
        .build()?;
    let url = format!("https://play.dlsite.com/api/purchases?page={}", page);
    let res = client.get(&url).send().await?;
    let product_list = res.json::<DLsiteProductListFromOwnerApi>().await?;

    fn get_localized_string(i18n: &DLsiteProductI18nString) -> Result<String, Error> {
        i18n.japanese
            .as_ref()
            .or_else(|| i18n.english.as_ref())
            .or_else(|| i18n.korean.as_ref())
            .or_else(|| i18n.taiwanese.as_ref())
            .or_else(|| i18n.chinese.as_ref())
            .cloned()
            .ok_or_else(|| anyhow!("localized string not found"))
    }

    let product_list = product_list
        .works
        .into_iter()
        .map(|product| -> Result<_, Error> {
            Ok(DLsiteProduct {
                id: product.id,
                ty: product.ty,
                age: product.age,
                title: get_localized_string(&product.title)?,
                thumbnail: product.icon.main,
                group_id: product.group.id,
                group_name: get_localized_string(&product.group.name)?,
                registered_at: product.registered_at,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(product_list)
}

pub async fn get_product_from_non_owner_api(id: &str) -> Result<DLsiteProduct, Error> {
    match try_join!(
        get_product_without_group_name(id),
        get_product_group_name(id)
    ) {
        Ok((product, group_name)) => {
            let naive_registered_at =
                NaiveDateTime::parse_from_str(&product.registered_at, "%Y-%m-%d %H:%M:%S")?;
            let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap();
            let jst_registered_at = jst_offset
                .from_local_datetime(&naive_registered_at)
                .single()
                .unwrap();
            let utc_registered_at = jst_registered_at.to_utc();

            Ok(DLsiteProduct {
                id: id.to_owned(),
                ty: product.ty,
                age: product.age,
                title: product.title,
                thumbnail: if product.thumbnail.starts_with("http") {
                    product.thumbnail
                } else {
                    format!("https:{}", product.thumbnail)
                },
                group_id: product.group_id,
                group_name,
                registered_at: utc_registered_at,
            })
        }
        Err(e) => Err(e),
    }
}

pub async fn get_product_without_group_name(
    id: &str,
) -> Result<DLsiteProductFromNonOwnerApi, Error> {
    let url = format!(
        "https://www.dlsite.com/maniax/product/info/ajax?product_id={}",
        id
    );
    let res = reqwest::get(&url)
        .await
        .with_context(|| format!("request failed for product id `{}` with url: `{}`", id, url))?;
    let map = res
        .json::<HashMap<String, DLsiteProductFromNonOwnerApi>>()
        .await
        .with_context(|| format!("parse failed for product id `{}`", id))?;

    map.get(id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("product not found"))
}

pub async fn get_product_group_name(id: &str) -> Result<String, Error> {
    let url = format!(
        "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
        id
    );
    let content = reqwest::get(&url).await?.text().await?;
    let html = scraper::Html::parse_document(&content);

    let group_name = html
        .select(&GROUP_NAME_SELECTOR)
        .next()
        .ok_or_else(|| anyhow::anyhow!("group name not found"))?
        .text()
        .collect::<String>();

    Ok(group_name)
}
