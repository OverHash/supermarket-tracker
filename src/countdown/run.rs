use std::{collections::HashMap, fs};

use error_stack::{Report, ResultExt};
use sqlx::{PgPool, Row};

use crate::{
    countdown::{get_all_products, get_categories, COUNTDOWN_BASE_URL, DEFAULT_USER_AGENT},
    error::ApplicationError,
    CACHE_PATH,
};

/// Runs the countdown scraper.
///
/// `no_insert` indicates if the scraper should not insert data into the database.
#[allow(clippy::too_many_lines)]
pub async fn run(connection: PgPool, no_insert: bool) -> Result<(), Report<ApplicationError>> {
    let client = {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "accept-language",
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
        );
        default_headers.insert(
            "x-requested-with",
            reqwest::header::HeaderValue::from_static("OnlineShopping.WebApp"),
        );

        reqwest::Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .default_headers(default_headers)
            .build()
            .change_context(ApplicationError::HttpError)
    }?;

    // retrieve categories
    println!("Retrieving all categories...");
    let categories = get_categories(&client, COUNTDOWN_BASE_URL)
        .await
        .change_context(ApplicationError::CategoryRetrieval)?;
    println!(
        "{}",
        categories
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );

    // retrieve products from all categories concurrently
    let products = get_all_products(client, categories)
        .await
        .change_context(ApplicationError::ProductRetrieval)?;
    println!("{:?} products were found", products.len());

    // cache the result
    fs::write(
        CACHE_PATH,
        serde_json::to_string_pretty(&products).change_context(ApplicationError::CacheError)?,
    )
    .change_context(ApplicationError::CacheError)?;

    // create the products if not existing before
    let new_products_ids = if no_insert {
        vec![]
    } else {
        let mut names = Vec::with_capacity(products.len());
        let mut barcodes = Vec::with_capacity(products.len());
        let mut skus = Vec::with_capacity(products.len());

        for p in &products {
            names.push(p.name.clone());
            barcodes.push(p.barcode.clone());
            skus.push(p.sku.clone());
        }

        sqlx::query!(
            r"INSERT INTO countdown_products (
					name, barcode, sku
				) SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
				ON CONFLICT (sku) DO NOTHING
				RETURNING sku, id
			",
            &names[..],
            &barcodes[..],
            &skus[..]
        )
        .fetch_all(&connection)
        .await
        .change_context(ApplicationError::NewProductsInsertionError)?
    };

    let product_count = new_products_ids.len();
    // Map<sku, countdown_id>
    let mut new_skus = new_products_ids.into_iter().fold(
        HashMap::with_capacity(product_count),
        |mut map, product| {
            map.insert(product.sku, product.id);
            map
        },
    );

    if !no_insert {
        let new_products = products
            .iter()
            .filter_map(|p| new_skus.remove(&p.sku))
            .collect::<Vec<_>>();
        sqlx::query!(
            r"INSERT INTO PRODUCTS (
				countdown_id
			) SELECT * FROM UNNEST($1::integer[])",
            &new_products[..]
        )
        .execute(&connection)
        .await
        .change_context(ApplicationError::NewProductsInsertionError)?;

        println!("Found {product_count} new products");
    }

    // upload all price data
    {
        let mapped_product_ids = sqlx::query!(
            r"SELECT products.id, countdown_products.sku FROM products
				INNER JOIN countdown_products
				ON products.countdown_id = countdown_products.id
				WHERE countdown_id IS NOT NULL",
        )
        .fetch_all(&connection)
        .await
        .change_context(ApplicationError::PriceDataInsertionError)?;

        let mut product_ids = Vec::with_capacity(products.len());
        let mut cost_in_cents = Vec::with_capacity(products.len());
        let mut supermarket = Vec::with_capacity(products.len());

        let mut mapped_products = HashMap::with_capacity(mapped_product_ids.len());
        for product in &products {
            mapped_products.insert(&product.sku, product.per_unit_price);
        }

        let mut lost_skus = vec![];
        for mapped_product in mapped_product_ids {
            // retrieve the cost associated with this sku
            let product_cost = mapped_products.remove(&mapped_product.sku);

            // find the product
            match product_cost {
                Some(cost) => {
                    product_ids.push(mapped_product.id);
                    cost_in_cents.push(cost);
                    supermarket.push("countdown".to_string());
                }
                None => lost_skus.push(mapped_product.sku),
            }
        }

        if !mapped_products.is_empty() {
            println!(
					"Failed to find {} products inserted in database. This may be the case if the `--no-insert` flag was run",
					mapped_products.len()
				);
            println!(
                "Products not found in database: {:?}",
                mapped_products.keys()
            );
        }

        println!(
            "Failed to find {} skus, items are likely off-sale",
            lost_skus.len()
        );

        if no_insert {
            println!("Skipped inserting prices into database");
        } else {
            // now insert the rows
            sqlx::query!(
                "INSERT INTO prices (
					product_id,
					cost_in_cents,
					supermarket
				) SELECT * FROM UNNEST($1::integer[], $2::integer[], $3::text[])",
                &product_ids[..],
                &cost_in_cents[..],
                &supermarket[..]
            )
            .execute(&connection)
            .await
            .change_context(ApplicationError::PriceDataInsertionError)?;

            println!("Inserted {} prices", product_ids.len());
        };

        Ok(())
    }
}
