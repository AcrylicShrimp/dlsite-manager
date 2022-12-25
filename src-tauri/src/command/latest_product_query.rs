use crate::{application_error::Result, storage::latest_product_query::LatestProductQuery};

#[tauri::command]
pub async fn latest_product_query_get() -> Result<LatestProductQuery> {
    LatestProductQuery::get()
}

#[tauri::command]
pub async fn latest_product_query_set(query: LatestProductQuery) -> Result<()> {
    LatestProductQuery::set(query)?;
    Ok(())
}
