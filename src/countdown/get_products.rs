use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ProductsResponse {
    products: ProductsItemsResponse,
}

#[derive(Deserialize)]
struct ProductsItemsResponse {
    items: Vec<SingleProductResponse>,
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

pub struct Product {}

/// Retrieves a list of products
///
/// Uses the /products?dasFilter=Department%3B%3Bfruit-veg%3Bfalse&target=browse endpoint.
pub async fn get_products(client: &Client, base_url: &str) -> Result<Vec<Product>, reqwest::Error> {
    let res: ProductsResponse = client
        .get(format!("{base_url}/products"))
        .query(&[("size", "120"), ("target", "browse"), ("page", "1")])
        .send()
        .await?
        .json()
        .await?;

    Ok(vec![])
}
