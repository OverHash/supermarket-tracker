use std::collections::HashSet;

use countdown::{get_categories, Category, Product};
use tokio::task;

use crate::countdown::get_products;

const SITE_URL: &str = "https://www.countdown.co.nz";
const BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";

mod countdown;

/// Retrieves the API key to use for future requests by scraping the HTML response.
async fn get_api_key(client: &reqwest::Client) -> Result<String, reqwest::Error> {
    let html_res = client.get(SITE_URL).send().await?.text().await?;

    let start_api_key = html_res
        .find("apikey=")
        .expect("Failed to find start of API key")
        + "apikey=".len();
    let end_api_key = html_res[start_api_key..]
        .find('"')
        .map(|n| n + start_api_key)
        .expect("Failed to find end of API key");

    Ok(String::from(&html_res[start_api_key..end_api_key]))
}

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

    let api_key = get_api_key(&client).await?;
    println!("Retrieved API key: {api_key:?}");

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
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|product| product.sku)
        .collect::<HashSet<_>>();
    println!("{:?} unique products were found", category_products.len());

    Ok(())
}
