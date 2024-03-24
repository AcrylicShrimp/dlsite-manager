use crate::{
    application_error::Result,
    database::{models::v1::LatestProductQuery, tables::v1::LatestProductQueryTable},
};

#[tauri::command]
pub async fn latest_product_query_get() -> Result<LatestProductQuery> {
    LatestProductQueryTable::get()
}

#[tauri::command]
pub async fn latest_product_query_set(query: LatestProductQuery) -> Result<()> {
    LatestProductQueryTable::set(query)?;
    Ok(())
}
