use countdown::get_categories;

use crate::countdown::get_products;

const SITE_URL: &str = "https://www.countdown.co.nz";
const BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";

mod countdown;

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

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
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
        categories.iter().map(|c| c.to_string()).collect::<Vec<_>>()
    );

    // retrieve products
    let products = get_products(&client, BASE_URL).await?;

    Ok(())
}
