use std::collections::HashSet;

use error_stack::{Result, ResultExt};
use sqlx::PgPool;

use crate::error::ApplicationError;

use super::Product;

/// Computes all the skus that are now off-sale against historically known skus,
/// by comparing against a list of known products that are currently on sale.
///
/// # Errors
/// If unable to retrieve skus from the database
pub async fn get_off_sale_skus(
    pool: &PgPool,
    fetched_products: &[Product],
) -> Result<Vec<String>, ApplicationError> {
    // retrieve all skus we have retrieved in the history of the database
    let stored_skus: HashSet<_> = sqlx::query!(
        r#"
	SELECT sku FROM countdown_products
	"#
    )
    .fetch_all(pool)
    .await
    .change_context(ApplicationError::ProductRetrieval)?
    .into_iter()
    .map(|product| product.sku)
    .collect();

    // now compare with the products we fetched
    let fetched_products_skus = fetched_products
        .iter()
        .map(|product| product.sku.clone())
        .collect();

    let lost_skus = stored_skus
        .difference(&fetched_products_skus)
        .cloned()
        .collect::<Vec<_>>();

    Ok(lost_skus)
}
