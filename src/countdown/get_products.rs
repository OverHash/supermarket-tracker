use std::{collections::HashSet, error::Error, fmt};

use error_stack::{Report, ResultExt};
use reqwest::Client;
use serde::Deserialize;
use tracing::Span;

use super::{Category, Product, scheduler};

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
struct ProductPrice {
    /// The normal price of the product.
    #[serde(rename = "originalPrice")]
    original: f32,
    /// The current price of the product.
    ///
    /// Equivalent to `original_price - save_price`.
    #[serde(rename = "salePrice")]
    sale: f32,
    /// The total amount on sale by purchasing this product.
    #[serde(rename = "savePrice")]
    save: f32,
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
    let category_url_part = category.url.split('/').next_back();

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
                per_unit_price: (price.sale * 100.0).round() as i32,
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

#[derive(Debug)]
pub enum ProductRetrievalError {
    ProductRetrieval,
}

impl fmt::Display for ProductRetrievalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductRetrievalError::ProductRetrieval => write!(f, "Failed to retrieve a product"),
        }
    }
}

impl Error for ProductRetrievalError {}

/// Retrieves all the products for a given [`Category`].
///
/// Uses the supplied API base URL, like [`get_products`].
///
/// Each concurrency slot waits for [`crate::PAGE_ITERATION_INTERVAL`] after a
/// response before starting another request, to prevent rate-limiting.
///
/// Runs up to [`crate::CONCURRENT_REQUESTS`] requests at once.
///
/// # Errors
///
/// Returns an error if an HTTP request fails.
#[tracing::instrument(name = "get_all_products", skip_all, fields(
	num_categories = %categories.len()
))]
pub async fn get_all_products(
    client: &reqwest::Client,
    base_url: &str,
    categories: Vec<Category>,
) -> Result<HashSet<Product>, Report<ProductRetrievalError>> {
    scheduler::collect_products(categories, |category, page| async move {
        get_products(client, base_url, page, &category).await
    })
    .await
    .change_context(ProductRetrievalError::ProductRetrieval)
}
