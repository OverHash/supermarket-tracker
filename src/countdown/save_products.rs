use error_stack::{Result, ResultExt};
use sqlx::PgPool;
use tracing::debug;

use crate::error::ApplicationError;

use super::Product;

/// Saves new products into the database.
///
/// If a product already exists in the database (by SKU), no change is performed.
///
/// Returns the new products that have not previously been tracked in the database.
#[tracing::instrument(name = "save products", level = "debug", skip_all, fields(
	product_count = %products.len()
))]
pub async fn save_products(pool: &PgPool, products: Vec<Product>) -> Result<(), ApplicationError> {
    let mut names = Vec::with_capacity(products.len());
    let mut barcodes = Vec::with_capacity(products.len());
    let mut skus = Vec::with_capacity(products.len());

    for product in products {
        names.push(product.name);
        barcodes.push(product.barcode);
        skus.push(product.sku);
    }

    // insert into `countdown_products` table
    let new_countdown_products = sqlx::query!(
        r#"
		INSERT INTO countdown_products (name, barcode, sku)
			SELECT * FROM UNNEST ($1::text[], $2::text[], $3::text[])
			ON CONFLICT (sku) DO NOTHING
			RETURNING sku, id
		"#,
        &names[..],
        &barcodes[..],
        &skus[..]
    )
    .fetch_all(pool)
    .await
    .change_context(ApplicationError::NewProductsInsertionError)?;
    if !new_countdown_products.is_empty() {
        // log first 10 new products
        let new_product_names = new_countdown_products
            .iter()
            .filter_map(|new_product| {
                // lookup by sku to get index
                skus.iter()
                    .position(|sku| &new_product.sku == sku)
                    .map(|index| names[index].clone())
            })
            .take(10)
            .collect::<Vec<_>>()
            .join(", ");
        debug!(
            "First {} new product names ({} new products): {}",
            new_countdown_products.len().min(10),
            new_countdown_products.len(),
            new_product_names
        );
    }

    // insert into `products` table
    sqlx::query!(
        r"INSERT INTO PRODUCTS (
			countdown_id
		) SELECT * FROM UNNEST($1::integer[])",
        &new_countdown_products
            .iter()
            .map(|product| product.id)
            .collect::<Vec<_>>()
    )
    .execute(pool)
    .await
    .change_context(ApplicationError::NewProductsInsertionError)?;

    Ok(())
}
