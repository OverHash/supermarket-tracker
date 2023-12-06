use std::fs;

use error_stack::{Report, ResultExt};
use sqlx::PgPool;

use crate::{
    countdown::{
        get_all_products, get_categories, get_off_sale_skus, save_prices, save_products,
        COUNTDOWN_BASE_URL, DEFAULT_USER_AGENT,
    },
    error::ApplicationError,
    CACHE_PATH,
};

/// Runs the countdown scraper.
///
/// `no_insert` indicates if the scraper should not insert data into the database.
///
/// # Errors
/// - If unable to create and perform HTTP tasks to countdown servers
/// - If unable to retrieve all categories of products
/// - If unable to retrieve all products
/// - If unable to compute the off-sale skus
/// - If unable to save prices
pub async fn run(connection: PgPool, should_insert: bool) -> Result<(), Report<ApplicationError>> {
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
    tracing::debug!("Retrieving all categories...");
    let categories = get_categories(&client, COUNTDOWN_BASE_URL)
        .await
        .change_context(ApplicationError::CategoryRetrieval)?;
    tracing::debug!(
        "Retrieved the following categories: {}",
        categories
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );

    // retrieve products from all categories concurrently
    // we turn from a HashSet<Product> into a Vec<Product> after
    tracing::debug!("Retrieving all products. This may take a while...");
    let products = get_all_products(client, categories)
        .await
        .change_context(ApplicationError::ProductRetrieval)?
        .into_iter()
        .collect::<Vec<_>>();
    tracing::debug!("{:?} products were found", products.len());

    // cache the result
    fs::write(
        CACHE_PATH,
        serde_json::to_string_pretty(&products).change_context(ApplicationError::CacheError)?,
    )
    .change_context(ApplicationError::CacheError)?;

    // create the products if not existing before
    if should_insert {
        save_products(
            &connection,
            products
                .iter()
                .map(|p| crate::countdown::Product {
                    name: p.name.clone(),
                    barcode: p.barcode.clone(),
                    per_unit_price: p.per_unit_price,
                    sku: p.sku.clone(),
                })
                .collect(),
        )
        .await?;
    }

    // log how many items are now off-sale
    let off_sale_skus = get_off_sale_skus(&connection, &products).await?;
    if !off_sale_skus.is_empty() {
        tracing::debug!(
            "Failed to find {} previously known skus. These items are likely now off-sale",
            off_sale_skus.len()
        );
    }

    // upload all price data
    save_prices(&connection, products, should_insert).await?;
    /*
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

        let mut mapped_products = HashMap::with_capacity(mapped_product_ids.len());
        for product in &products {
            mapped_products.insert(&product.sku, product.per_unit_price);
        }

        for mapped_product in mapped_product_ids {
            // retrieve the cost associated with this sku
            let product_cost = mapped_products.remove(&mapped_product.sku);

            // find the product
            if let Some(cost) = product_cost {
                product_ids.push(mapped_product.id);
                cost_in_cents.push(cost);
            }
        }

        if !mapped_products.is_empty() {
            tracing::warn!(
                    "Failed to find {} products inserted in database. This may be the case if the `--no-insert` flag was run",
                    mapped_products.len()
                );
            if should_insert {
                tracing::warn!(
                    "Products not found in database: {:?}",
                    mapped_products.keys()
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
            .execute(&connection)
            .await
            .change_context(ApplicationError::PriceDataInsertionError)?;

            tracing::debug!("Inserted {} prices", product_ids.len());
        } else {
            tracing::debug!("Skipped inserting prices into database");
        };

        Ok(())
    } */
    Ok(())
}
