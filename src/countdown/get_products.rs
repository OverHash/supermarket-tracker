use reqwest::Client;
use serde::Deserialize;

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
        /// The variety of the product, if it has one.
        variety: Option<String>,
        /// The brand type of the product.
        brand: String,
        /// A URL slug representing the product.
        slug: String,
        /// A unique store identifier ID.
        sku: String,
        /// The type of unit when purchasing the product.
        unit: String,
        /// The price of the product.
        price: ProductPrice,
    },
    /// A promotional item
    PromoTile {},
    /// A carousel item of products, usually contained within a group.
    PromotionalCarousel {},
}

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

/// The size to query each page.
const PAGE_SIZE: i32 = 120;

/// The response for a request to browse the store.

#[derive(Debug)]
pub struct GetProductResponse {
    /// The current page's products
    pub products: Vec<Product>,
    /// The next page number, or None if we have reached the end.
    pub next_page: Option<i32>,
}

/// Retrieves a list of products
///
/// Uses the /products?target=browse endpoint.
pub async fn get_products(
    client: &Client,
    base_url: &str,
    page_number: i32,
    category: Option<&Category>,
) -> Result<GetProductResponse, reqwest::Error> {
    let res: ProductsResponse = client
        .get(format!("{base_url}/products"))
        .query(&[
            ("size", Some(PAGE_SIZE.to_string())),
            ("target", Some(String::from("browse"))),
            ("page", Some(page_number.to_string())),
            (
                "dasFilter",
                category.map(|c| format!("Department;;{};false", c.url)),
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
                per_unit_price: (price.sale_price * 100.0) as i32,
            }),
            _ => None,
        })
        .collect::<Vec<Product>>();

    let is_end = PAGE_SIZE * page_number > res.products.total_items || products.is_empty();

    Ok(GetProductResponse {
        products,
        next_page: (!is_end).then_some(page_number + 1),
    })
}
