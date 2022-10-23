#[derive(Debug)]
pub struct Product {
    /// Then name of the product.
    pub name: String,
    /// The barcode of the product.
    pub barcode: String,
    /// The sku of the product.
    pub sku: String,
    /// The current price of the product, in cents.
    pub per_unit_price: i32,
}
