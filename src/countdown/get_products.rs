use std::{
    collections::{HashSet, VecDeque},
    fmt,
    sync::Arc,
};

use error_stack::{Context, Result, ResultExt};
use reqwest::Client;
use serde::Deserialize;
use tokio::{sync::Mutex, task};
use tracing::Span;

use crate::{countdown::COUNTDOWN_BASE_URL, CONCURRENT_REQUESTS, PAGE_ITERATION_INTERVAL};

use super::{Category, Product};

/// Describes the response returned by the /products endpoint.
#[derive(Deserialize)]
struct ProductsResponse {
    /// All the items on the current page.
    products: ProductsItemsResponse,
}

/// Represents the inner products of a [`ProductsResponse`]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductsItemsResponse {
    /// All the items on the current page.
    items: Vec<ItemResponse>,
    /// The total amount of items across all pages.
    total_items: i32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ItemResponse {
    /// A single product that can be purchased.
    Product {
        /// A lowercase string representation of the product
        name: String,
        /// The GS1 barcode.
        barcode: String,
        /// A unique store identifier ID.
        sku: String,
        /// The price of the product.
        price: ProductPrice,
    },
    /// A promotional item
    PromoTile {},
    /// A carousel item of products, usually contained within a group.
    PromotionalCarousel {},
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductPrice {
    /// The normal price of the product.
    original_price: f32,
    /// The current price of the product.
    ///
    /// Equivalent to `original_price - save_price`.
    sale_price: f32,
    /// The total amount on sale by purchasing this product.
    save_price: f32,
}

/// The size amount of items to query for each page.
const PAGE_SIZE: i32 = 120;

/// The response for a request to browse the store.

#[derive(Debug)]
pub struct GetProductResponse {
    /// The current page's products
    pub products: HashSet<Product>,
    /// The next page number, or None if we have reached the end.
    pub next_page: Option<i64>,
    /// The total amount of pages that may exist.
    ///
    /// Note that the API may not fully return all of these pages, it is an upper limit.
    pub total_pages: i64,
}

/// Retrieves a list of products
///
/// Uses the `/products?target=browse` endpoint.
#[tracing::instrument(
	name = "get products",
	level = "trace",
	skip_all,
	fields(
		%page_number,
		%category,
		product_count_retrieved = tracing::field::Empty
	)
)]
pub async fn get_products(
    client: &Client,
    base_url: &str,
    page_number: i64,
    category: &Category,
) -> Result<GetProductResponse, reqwest::Error> {
    // our Category contains url information
    // but we only want the last part of the url
    let category_url_part = category.url.split('/').last();

    let res: ProductsResponse = client
        .get(format!("{base_url}/products"))
        .query(&[
            ("size", Some(PAGE_SIZE.to_string())),
            ("target", Some(String::from("browse"))),
            ("page", Some(page_number.to_string())),
            (
                "dasFilter",
                category_url_part.map(|url| format!("Department;;{url};false")),
            ),
        ])
        .send()
        .await?
        .json()
        .await?;

    let products = res
        .products
        .items
        .into_iter()
        .filter_map(|item| match item {
            ItemResponse::Product {
                name,
                barcode,
                price,
                sku,
                ..
            } => Some(Product {
                name,
                barcode,
                sku,
                // convert to cents from dollars
                #[allow(clippy::cast_possible_truncation)]
                per_unit_price: (price.sale_price * 100.0).round() as i32,
            }),
            _ => None,
        })
        .collect::<HashSet<Product>>();

    let is_end =
        page_number * i64::from(PAGE_SIZE) > res.products.total_items.into() || products.is_empty();

    Span::current().record("product_count_retrieved", products.len());

    Ok(GetProductResponse {
        products,
        next_page: (!is_end).then_some(page_number + 1),
        #[allow(clippy::cast_possible_truncation)]
        total_pages: (f64::from(res.products.total_items) / f64::from(PAGE_SIZE)).ceil() as i64,
    })
}

type AddTask = Option<Box<dyn FnOnce(&GetProductResponse) -> Vec<PageRequestTask> + Send + Sync>>;

struct PageRequestTask {
    category: Category,
    page: i64,
    add_tasks: AddTask,
}

async fn perform_task(
    client: Client,
    tasks: Arc<Mutex<VecDeque<PageRequestTask>>>,
) -> Result<HashSet<Product>, reqwest::Error> {
    let mut total_products = HashSet::new();

    loop {
        let task = tasks.lock().await.pop_front();
        let Some(task) = task else {
            break;
        };

        let res = get_products(&client, COUNTDOWN_BASE_URL, task.page, &task.category).await?;

        // handle the add_tasks callback if it existed
        if let Some(callback) = task.add_tasks {
            let new_tasks = callback(&res);
            tasks.lock().await.extend(new_tasks);
        }

        total_products.extend(res.products);

        // give the API some time to rest
        // so we don't get rate limited
        tokio::time::sleep(PAGE_ITERATION_INTERVAL).await;
    }

    Ok(total_products)
}

#[derive(Debug)]
pub enum ProductRetrievalError {
    Join,
    ProductRetrieval,
}

impl fmt::Display for ProductRetrievalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductRetrievalError::Join => write!(f, "Failed to join all products after querying"),
            ProductRetrievalError::ProductRetrieval => write!(f, "Failed to retrieve a product"),
        }
    }
}

impl Context for ProductRetrievalError {}

/// Retrieves all the products for a given [`Category`].
///
/// Yields for [`PAGE_ITERATION_INTERVAL`] between requests, to prevent rate-limiting.
///
/// Runs [`CONCURRENT_REQUESTS`] requests at once.
#[tracing::instrument(name = "get_all_products", skip_all, fields(
	num_categories = %categories.len()
))]
pub async fn get_all_products(
    client: reqwest::Client,
    categories: Vec<Category>,
) -> Result<HashSet<Product>, ProductRetrievalError> {
    let tasks: Arc<Mutex<VecDeque<PageRequestTask>>> = Arc::new(Mutex::new(
        categories
            .into_iter()
            .map(|category| PageRequestTask {
                category: category.clone(),
                page: 1,
                add_tasks: Some(Box::new(move |products_response| {
                    (2..=products_response.total_pages)
                        .map(|page| PageRequestTask {
                            category: category.clone(),
                            page,
                            add_tasks: None,
                        })
                        .collect()
                })),
            })
            .collect(),
    ));

    let task_results = futures::future::join_all((0..CONCURRENT_REQUESTS).map(|_| {
        let new_client = client.clone();
        let new_tasks = tasks.clone();
        task::spawn(async move { perform_task(new_client, new_tasks).await })
    }))
    .await
    .into_iter()
    .collect::<std::result::Result<Vec<_>, _>>()
    .change_context(ProductRetrievalError::Join)?;

    let product_results = task_results
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()
        .change_context(ProductRetrievalError::ProductRetrieval)?
        .into_iter()
        .flatten()
        .collect();

    Ok(product_results)
}
