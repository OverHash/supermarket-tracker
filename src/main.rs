use std::env;

use countdown::{get_categories, Category, Product};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use tokio::task;

use crate::countdown::get_products;
use crate::initialize_database::initialize_database;

const BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";

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
        println!("Getting page {current} for category {category}");

        let res = get_products(&client, BASE_URL, current, Some(&category)).await?;
        current_page = res.next_page;
        products.extend(res.products);

        // give the API some time to rest
        // so we don't get rate limited
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(products)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ignore any error attempting to load .env file
    dotenv().ok();

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
    initialize_database(&connection).await?;

    // retrieve categories
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
    // todo

    // create the products if not existing before
    {
        let mut names = Vec::with_capacity(category_products.len());
        let mut barcodes = Vec::with_capacity(category_products.len());
        let mut skus = Vec::with_capacity(category_products.len());

        category_products.into_iter().for_each(|p| {
            names.push(p.name);
            barcodes.push(p.barcode);
            skus.push(p.sku);
        });

        let inserted_names = sqlx::query(
            r#"INSERT INTO countdown_products (
				name, barcode, sku
			) SELECT * FROM UNNEST($1, $2, $3)
			WHERE NOT EXISTS(
				SELECT name FROM countdown_products WHERE name = $1
			)
			RETURNING name
		"#,
        )
        .bind(&names)
        .bind(&barcodes)
        .bind(&skus)
        .map(|row| {
            let name: String = row.get(0);
            name
        })
        .fetch_all(&connection)
        .await?;

        println!("Inserted {} names", inserted_names.len());
    }

    Ok(())
}
