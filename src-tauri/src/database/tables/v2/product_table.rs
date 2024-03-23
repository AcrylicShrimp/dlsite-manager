use crate::{
    application_error::{Error, Result},
    database::{models::v2::Product, Table},
    dlsite::v2::{DLsiteProductAgeCategory, DLsiteProductType},
    use_application,
};
use serde_rusqlite::*;

pub struct ProductTable;

impl Table for ProductTable {
    fn get_ddl() -> &'static str {
        r#"
CREATE TABLE IF NOT EXISTS v2_products (
    id TEXT NOT NULL PRIMARY KEY,
    account_id INTEGER NOT NULL,
    ty TEXT NOT NULL,
    age TEXT NOT NULL,
    title TEXT NOT NULL,
    thumbnail TEXT NOT NULL,
    group_id TEXT NOT NULL,
    group_name TEXT NOT NULL,
    registered_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(account_id) REFERENCES v2_accounts(id) ON UPDATE CASCADE ON DELETE CASCADE
);

CREATE TRIGGER IF NOT EXISTS v2_products_updated_at AFTER UPDATE ON v2_products
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE products SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE VIRTUAL TABLE IF NOT EXISTS v2_indexed_products USING fts5 (
    id,
    title,
    group_id,
    group_name,
    tokenize = 'trigram'
);
"#
    }
}

impl ProductTable {
    pub fn insert_many(products: impl Iterator<Item = Product>) -> Result<()> {
        let mut connection = use_application().connection();
        let tx = connection.transaction()?;
        {
            let mut insert_stmt = tx.prepare(
                r#"
INSERT INTO v2_products (
    id,
    account_id,
    ty,
    age,
    title,
    thumbnail,
    group_id,
    group_name,
    registered_at
) VALUES (
    :id,
    :account_id,
    :ty,
    :age,
    :title,
    :thumbnail,
    :group_id,
    :group_name,
    :registered_at
) ON CONFLICT (id) DO UPDATE SET
    account_id = excluded.account_id,
    ty = excluded.ty,
    age = excluded.age,
    title = excluded.title,
    thumbnail = excluded.thumbnail,
    group_id = excluded.group_id,
    group_name = excluded.group_name,
    registered_at = excluded.registered_at,
    updated_at = CURRENT_TIMESTAMP;
"#,
            )?;
            let mut index_stmt = tx.prepare(
                r#"
INSERT INTO v2_indexed_products (
    id,
    title,
    group_id,
    group_name
) VALUES (
    :id,
    :title,
    :group_id,
    :group_name
) ON CONFLICT (id) DO UPDATE SET
    title = excluded.title,
    group_id = excluded.group_id,
    group_name = excluded.group_name;
"#,
            )?;

            for product in products {
                insert_stmt.execute(to_params_named(&product)?.to_slice().as_slice())?;
                index_stmt.execute(
                    to_params_named_with_fields(
                        &product,
                        &["id", "title", "group_id", "group_name"],
                    )?
                    .to_slice()
                    .as_slice(),
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_many(
        query: Option<&str>,
        ty: Option<DLsiteProductType>,
        age: Option<DLsiteProductAgeCategory>,
        order_by_asc: bool,
    ) -> Result<Vec<Product>> {
        let mut where_clause = String::new();
        let mut params = vec![];
        where_clause.push_str("WHERE TRUE");

        if let Some(query) = query {
            let query = query.trim();

            if !query.is_empty() {
                where_clause.push_str(" AND v2_indexed_products MATCH :query");
                params.push(to_params_named(&query)?);
            }
        }

        if let Some(ty) = ty {
            where_clause.push_str(" AND product.ty = :ty");
            params.push(to_params_named(&ty)?);
        }

        if let Some(age) = age {
            where_clause.push_str(" AND product.age = :age");
            params.push(to_params_named(&age)?);
        }

        let params = params
            .iter()
            .map(|param| param.to_slice())
            .flatten()
            .collect::<Vec<_>>();

        let order_by_clause = if order_by_asc {
            "product.registered_at ASC, id ASC"
        } else {
            "product.registered_at DESC, id DESC"
        };

        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            format!(
                r#"
SELECT
    product.id,
    product.account_id,
    product.ty,
    product.age,
    product.title,
    product.thumbnail,
    product.group_id,
    product.group_name,
    product.registered_at
FROM v2_indexed_products
INNER JOIN v2_products AS product ON product.id = v2_indexed_products.id
WHERE {}
ORDER BY {}
"#,
                where_clause, order_by_clause
            )
            .as_str(),
        )?;

        let products = stmt
            .query_and_then(params.as_slice(), from_row::<Product>)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(products)
    }
}
