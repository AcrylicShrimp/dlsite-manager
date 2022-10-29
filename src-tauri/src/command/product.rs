use crate::{
    application_error::Result,
    storage::product::{Product, ProductQuery},
};

#[tauri::command]
pub async fn product_list_products(query: Option<ProductQuery>) -> Result<Vec<Product>> {
    Ok(Product::list_all(&query.unwrap_or_default()).unwrap())
}
