use std::collections::HashMap;

use error_stack::{Result, ResultExt};
use sqlx::PgPool;
use tracing::{debug, trace, warn};

use crate::error::ApplicationError;

use super::Product;

/// Bulk saves `products` to a Postgres database.
#[tracing::instrument(
	name = "save prices",
	level = "debug",
	skip_all,
	fields(
		product_count = %products.len(),
		skip_insert = %!should_insert
	)
)]
pub async fn save_prices(
    pool: &PgPool,
    products: Vec<Product>,
    should_insert: bool,
) -> Result<(), ApplicationError> {
    // We perform the bulk save by first retrieving all the product IDs in
    // the Postgres database that are under the 'countdown' supermarket.
    // In the future, we might have to look at optimizing this in some other
    // way, to ensure that we are only getting the product IDs of items inside
    // `products`. Over-fetching is okay here, as it gives us considerably
    // less queries to perform.
    // Once we have all the products that have countdown IDs, we map from our
    // API fetched products to Postgres products by the matching the skus.

    // retrieve countdown skus and product ids
    let mut product_sku_to_id = sqlx::query!(
        r"SELECT products.id, countdown_products.sku FROM products
			INNER JOIN countdown_products
			ON products.countdown_id = countdown_products.id
			WHERE countdown_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .change_context(ApplicationError::PriceDataInsertionError)?
    .into_iter()
    .map(|product| (product.sku, product.id))
    .collect::<HashMap<_, _>>();

    let mut product_ids = Vec::with_capacity(products.len());
    let mut cost_in_cents = Vec::with_capacity(products.len());

    for product in products {
        // find the corresponding stored product
        let product_id = product_sku_to_id.remove(&product.sku);

        let Some(id) = product_id else {
            warn!(
                "Failed to get stored product ID for sku '{}' ('{}')",
                product.sku, product.name
            );
            continue;
        };

        product_ids.push(id);
        cost_in_cents.push(product.per_unit_price);
    }

    if !product_sku_to_id.is_empty() {
        warn!(
				"Failed to find {} products inserted in database. This may be the case if the `--no-insert` flag was run",
				product_sku_to_id.len()
			);
        if should_insert {
            warn!(
                "Products not found in database: {:?}",
                product_sku_to_id.keys()
            );
        }
    }

    if should_insert {
        // now insert the rows
        sqlx::query!(
            "INSERT INTO prices (
				product_id,
				cost_in_cents,
				supermarket
			) SELECT
				UNNEST($1::integer[]),
				UNNEST($2::integer[]),
				'countdown'
			",
            &product_ids[..],
            &cost_in_cents[..],
        )
        .execute(pool)
        .await
        .change_context(ApplicationError::PriceDataInsertionError)?;

        debug!("Inserted {} prices", product_ids.len());
    } else {
        debug!("Skipped inserting prices into database");
    };

    Ok(())
}
