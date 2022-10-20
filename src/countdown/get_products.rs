use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ProductsResponse {
    products: ProductsItemsResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductsItemsResponse {
    items: Vec<SingleProductResponse>,
    total_items: i32,
}

#[derive(Deserialize)]
struct SingleProductResponse {
    /// Always "Product"
    #[serde(rename = "type")]
    product_type: String,
    /// A lowercase string representation of the product
    name: String,
    barcode: String,
    variety: Option<String>,
    brand: String,
    slug: String,
    sku: String,
    unit: String,
    price: ProductPrice,
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
    /// Then name of the product
    name: String,
    /// The barcode of the product
    barcode: String,
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
) -> Result<GetProductResponse, reqwest::Error> {
    let res: ProductsResponse = client
        .get(format!("{base_url}/products"))
        .query(&[
            ("size", PAGE_SIZE.to_string()),
            ("target", String::from("browse")),
            ("page", String::from("1")),
        ])
        .send()
        .await?
        .json()
        .await?;

    let products = res
        .products
        .items
        .into_iter()
        .map(|p| Product {
            name: p.name,
            barcode: p.barcode,
            per_unit_price: (p.price.sale_price * 100.0) as i32,
        })
        .collect::<Vec<Product>>();

    let is_not_end = PAGE_SIZE * page_number > res.products.total_items && !products.is_empty();

    Ok(GetProductResponse {
        products,
        next_page: (!is_not_end).then_some(page_number + 1),
    })
}
