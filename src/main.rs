use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::{env, fs};

use countdown::{get_categories, Category, Product};
use dotenvy::dotenv;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::Row;
use tokio::task;

use crate::countdown::get_products;
use crate::initialize_database::initialize_database;

const BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";
const CACHE_PATH: &str = "cache.json";
/// The amount of milliseconds to wait between performing iterations on the pages.
const PAGE_ITERATION_INTERVAL: Duration = Duration::from_millis(500);

mod countdown;
mod initialize_database;

/// Retrieves all the products for a given category.
///
/// Yields for 500ms between requests, to prevent rate-limiting.
async fn get_all_products(
    client: reqwest::Client,
    category: Category,
) -> Result<Vec<Product>, reqwest::Error> {
    let mut current_page = Some(1);
    let mut products = Vec::new();
    while let Some(current) = current_page {
        eprintln!("Getting page {current} for category {category}");

        let res = get_products(&client, BASE_URL, current, Some(&category)).await?;
        current_page = res.next_page;
        products.extend(res.products);

        // give the API some time to rest
        // so we don't get rate limited
        tokio::time::sleep(PAGE_ITERATION_INTERVAL).await;
    }

    Ok(products)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ignore any error attempting to load .env file
    dotenv().ok();

    let args = env::args().skip(1).collect::<HashSet<_>>();
    let no_insert = args.contains("--no-insert");

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
            .expect("Failed to create http client")
    };

    // connect to database
    let connection = PgPoolOptions::new()
        .max_connections(5)
        .connect(
            &env::var("DATABASE_URL").expect("Failed to read DATABASE_URL environment variable"),
        )
        .await?;
    println!("Connected to database");
    initialize_database(&connection).await?;

    // retrieve categories
    println!("Retrieving all categories...");
    let categories = get_categories(&client, BASE_URL).await?;
    println!(
        "{:?}",
        categories
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );

    // retrieve products from all categories concurrently
    let category_retrieval = futures::future::join_all(
        categories
            .into_iter()
            .map(|category| task::spawn(get_all_products(client.clone(), category))),
    )
    .await;

    // transform into the sku's
    let category_products = category_retrieval
        .into_iter()
        .map(|category_results| category_results.expect("Failed to get category"))
        .collect::<Result<Vec<Vec<Product>>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    println!("{:?} products were found", category_products.len());

    // cache the result
    fs::write(
        CACHE_PATH,
        serde_json::to_string_pretty(&category_products)?,
    )?;

    // create the products if not existing before
    let new_products_ids = if !no_insert {
        let mut names: Vec<&str> = Vec::with_capacity(category_products.len());
        let mut barcodes: Vec<&str> = Vec::with_capacity(category_products.len());
        let mut skus: Vec<&str> = Vec::with_capacity(category_products.len());

        category_products.iter().for_each(|p| {
            names.push(&p.name);
            barcodes.push(&p.barcode);
            skus.push(&p.sku);
        });

        sqlx::query(
            r#"INSERT INTO countdown_products (
				name, barcode, sku
			) SELECT * FROM UNNEST($1, $2, $3)
			ON CONFLICT (sku) DO NOTHING
			RETURNING sku, id
		"#,
        )
        .bind(names)
        .bind(barcodes)
        .bind(skus)
        .map(|row| {
            let sku: String = row.get(0);
            let id: i32 = row.get(1);

            (sku, id)
        })
        .fetch_all(&connection)
        .await?
    } else {
        vec![]
    };

    let product_count = new_products_ids.len();
    // Map<sku, countdown_id>
    let mut new_skus = new_products_ids.into_iter().fold(
        HashMap::with_capacity(product_count),
        |mut map, (sku, countdown_id)| {
            map.insert(sku, countdown_id);
            map
        },
    );

    if !no_insert {
        let new_products = category_products
            .iter()
            .filter_map(|p| new_skus.remove(&p.sku))
            .collect::<Vec<_>>();
        sqlx::query(
            r#"INSERT INTO PRODUCTS (
			countdown_id	
		) SELECT * FROM UNNEST($1)"#,
        )
        .bind(new_products)
        .execute(&connection)
        .await?;

        println!("Found {product_count} new products");
    }

    // upload all price data
    {
        let mapped_product_ids = sqlx::query(
            r#"SELECT products.id, countdown_products.sku FROM products
			INNER JOIN countdown_products
			ON products.countdown_id = countdown_products.id
			WHERE countdown_id IS NOT NULL"#,
        )
        .map(|row: PgRow| {
            let id: i32 = row.get(0);
            let sku: String = row.get(1);

            (id, sku)
        })
        .fetch_all(&connection)
        .await?;

        let mut product_ids = Vec::with_capacity(category_products.len());
        let mut cost_in_cents = Vec::with_capacity(category_products.len());
        let mut supermarket = Vec::with_capacity(category_products.len());

        let mut mapped_products = HashMap::with_capacity(mapped_product_ids.len());
        category_products.iter().for_each(|product| {
            mapped_products.insert(&product.sku, product.per_unit_price);
        });

        let mut lost_skus = vec![];
        for (product_id, sku) in mapped_product_ids {
            // retrieve the cost associated with this sku
            let product_cost = mapped_products.remove(&sku);

            // find the product
            match product_cost {
                Some(cost) => {
                    product_ids.push(product_id);
                    cost_in_cents.push(cost);
                    supermarket.push("countdown");
                }
                None => lost_skus.push(sku),
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
            "Failed to find {} skus, items were removed from the store",
            lost_skus.len()
        );

        if !no_insert {
            // now insert the rows
            sqlx::query(
                "INSERT INTO prices (
				product_id,
				cost_in_cents,
				supermarket
			) SELECT * FROM UNNEST($1, $2, $3)",
            )
            .bind(&product_ids)
            .bind(&cost_in_cents)
            .bind(&supermarket)
            .execute(&connection)
            .await?;

            println!("Inserted {} prices", product_ids.len());
        } else {
            println!("Skipped inserting prices into database");
        }
    }

    Ok(())
}
