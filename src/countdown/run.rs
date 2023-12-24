use std::fs;

use error_stack::{Report, ResultExt};
use sqlx::PgPool;

use crate::{
    countdown::{
        get_all_products, get_categories, get_off_sale_skus, save_prices, save_products,
        save_store, set_location, COUNTDOWN_BASE_URL, DEFAULT_USER_AGENT,
    },
    error::ApplicationError,
    CACHE_PATH,
};

use super::set_fulfillment_method;

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
            .cookie_store(true)
            .build()
            .change_context(ApplicationError::HttpError)
    }?;

    set_fulfillment_method(&client, COUNTDOWN_BASE_URL)
        .await
        .change_context(ApplicationError::SetLocation)
        .attach_printable("When setting fulfillment method to pickup")?;

    // search for a North Island and South Island countdown
    // since North Island and South Island have differing prices.
    // we pick the largest countdowns in each island.
    for (store_id, store_name) in [
        (1_906_076, "Countdown Mt Eden".to_string()),
        (1_352_617, "Woolworths Hornby".to_string()),
    ] {
        set_location(&client, COUNTDOWN_BASE_URL, store_id)
            .await
            .change_context(ApplicationError::SetLocation)?;

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
        let products = get_all_products(&client, categories)
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

        if should_insert {
            // create the products if not existing before
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

        // store the store if it has not been created before
        let store_id = save_store(&connection, store_id, store_name)
            .await
            .change_context(ApplicationError::SaveStore)?;

        // upload all price data
        save_prices(&connection, products, store_id, should_insert).await?;
    }

    Ok(())
}
