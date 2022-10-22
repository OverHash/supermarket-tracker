use reqwest::Client;
use serde::Deserialize;

use super::get_categories::Category;

#[derive(Deserialize)]
struct ProductsResponse {
    products: ProductsItemsResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductsItemsResponse {
    items: Vec<ItemResponse>,
    total_items: i32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ItemResponse {
    Product {
        /// A lowercase string representation of the product
        name: String,
        barcode: String,
        variety: Option<String>,
        brand: String,
        slug: String,
        sku: String,
        unit: String,
        price: ProductPrice,
    },
    PromoTile {},
    PromotionalCarousel {},
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductPrice {
    original_price: f32,
    sale_price: f32,
    save_price: f32,
}

#[derive(Debug)]
pub struct Product {
    /// Then name of the product.
    name: String,
    /// The barcode of the product.
    barcode: String,
    /// The sku of the product.
    pub sku: String,
    /// The current price of the product, in cents.
    per_unit_price: i32,
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
