mod support;

use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use supermarket_tracker::countdown::{Category, Product, get_all_products};
use tokio::time::timeout;

use support::{ProductRequest, TestApi};

// Bounds failures in real HTTP I/O; scheduler tests use controlled futures and
// paused time instead of waiting for missing requests to time out.
const SCRAPE_DEADLINE: Duration = Duration::from_secs(10);

fn client() -> Client {
    Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build the test HTTP client")
}

fn category() -> Category {
    Category {
        name: "Fruit and vegetables".to_owned(),
        url: "/shop/browse/fruit-and-vegetables".to_owned(),
    }
}

fn assert_requests(mut requests: Vec<ProductRequest>) {
    requests.sort_unstable_by_key(|request| request.page);
    let expected: Vec<_> = (1..=3)
        .map(|page| ProductRequest {
            page,
            size: 120,
            target: "browse".to_owned(),
            department: "Department;;fruit-and-vegetables;false".to_owned(),
        })
        .collect();
    assert_eq!(requests, expected, "Every page must be requested once");
}

fn assert_products(products: &HashSet<Product>) {
    let mut skus: Vec<_> = products
        .iter()
        .map(|product| product.sku.as_str())
        .collect();
    skus.sort_unstable();
    assert_eq!(skus, ["sku-1", "sku-2", "sku-3"]);
    for product in products {
        assert_eq!(product.per_unit_price, 150);
        let page = product.sku.strip_prefix("sku-").unwrap();
        assert_eq!(product.name, format!("Product on page {page}"));
        assert_eq!(product.barcode, format!("barcode-{page}"));
    }
}

#[tokio::test]
async fn one_category_discovers_and_retrieves_every_page_once() {
    let mut api = TestApi::spawn().await;
    let client = client();
    let base_url = api.base_url.clone();

    let products = timeout(
        SCRAPE_DEADLINE,
        get_all_products(&client, &base_url, vec![category()]),
    )
    .await
    .expect("The scraper did not finish")
    .expect("Product retrieval failed");

    let requests = api.drain_requests();
    api.shutdown().await;
    assert_requests(requests);
    assert_products(&products);
}
