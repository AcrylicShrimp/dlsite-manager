use crate::{
    application_error::{Error, Result},
    dlsite::{get_product, get_product_count, login},
    storage::{
        account::Account,
        product::{InsertedProduct, Product, ProductQuery},
    },
};

#[tauri::command]
pub fn product_list_products(query: Option<ProductQuery>) -> Result<Vec<Product>> {
    Ok(Product::list_all(&query.unwrap_or_default()).unwrap())
}

#[tauri::command]
pub async fn product_update_products() -> Result<Vec<Product>> {
    Product::remove_all()?;

    for account in Account::list_all()? {
        let cookie_store = login(&account.username, &account.password).await?;

        let mut page = 1;
        let mut count = 0;
        let product_count = match get_product_count(cookie_store.clone()).await {
            Ok(product_count) => product_count,
            Err(err) => {
                if let Error::DLsiteNotAuthenticated = err {
                    continue;
                } else {
                    return Err(err);
                }
            }
        };

        while count < product_count {
            let products = match get_product(cookie_store.clone(), page).await {
                Ok(products) => products,
                Err(err) => {
                    if let Error::DLsiteNotAuthenticated = err {
                        continue;
                    } else {
                        return Err(err);
                    }
                }
            };

            page += 1;
            count += products.len();

            Product::insert_all(products.into_iter().map(|product| InsertedProduct {
                account_id: account.id,
                product,
            }))?;
        }
    }

    Product::list_all(&ProductQuery::default())
}
