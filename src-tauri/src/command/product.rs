use crate::{
    application_error::Result,
    dlsite::{get_product, get_product_count, login},
    storage::{
        account::Account,
        product::{InsertedProduct, Product},
    },
};

#[tauri::command]
pub fn product_list_products() -> Result<Vec<Product>> {
    Product::list_all()
}

#[tauri::command]
pub async fn product_update_products() -> Result<Vec<Product>> {
    Product::remove_all()?;

    for account in Account::list_all()? {
        let cookie_store = login(&account.username, &account.password).await?;

        let mut page = 1;
        let mut count = 0;
        let product_count = get_product_count(cookie_store.clone()).await?;

        while count < product_count {
            let products = get_product(cookie_store.clone(), page).await?;

            page += 1;
            count += products.len();

            Product::insert_all(products.into_iter().map(|product| InsertedProduct {
                account_id: account.id,
                product,
            }))?;
        }
    }

    Product::list_all()
}
