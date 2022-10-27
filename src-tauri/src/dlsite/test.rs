use crate::application_error::{Error, Result};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use std::{collections::HashMap, sync::Arc};

pub async fn test_account(
    username: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<Option<usize>> {
    let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .cookie_provider(cookie_store.clone())
        .build()?;

    client
        .get("https://www.dlsite.com/maniax/login/=/skip_register/1")
        .send()
        .await?;
    client.get("https://login.dlsite.com/login").send().await?;

    // We must ignore the response of this login request here.
    // The DLsite always responds with normal 200 status even if the login has been failed.
    client
        .post("https://login.dlsite.com/login")
        .form(&[
            ("login_id", username.as_ref()),
            ("password", password.as_ref()),
            ("_token", &{
                let cookie = cookie_store
                    .lock()
                    .unwrap()
                    .get("login.dlsite.com", "/", "XSRF-TOKEN")
                    .ok_or_else(|| Error::DLsiteCookieNotFoundError {
                        cookie_domain: "login.dlsite.com".to_owned(),
                        cookie_path: "/".to_owned(),
                        cookie_name: "XSRF-TOKEN".to_owned(),
                    })?
                    .value()
                    .to_owned();
                cookie
            }),
        ])
        .send()
        .await?;

    let response = client
        .get("https://play.dlsite.com/api/product_count")
        .send()
        .await?;

    // The body of the response will be a valid json if the login has been succeed.
    if let Ok(product_count) = response.json::<HashMap<String, usize>>().await {
        return Ok(Some(product_count.get("user").cloned().unwrap_or(0)));
    }

    Ok(None)
}
