use serde::Serialize;

/// Represents a product that can be purchased from Countdown.
///
/// We define two products to be equal if their `sku` values are equal.
/// Similarly, we define the hash of a product to be solely from its sku,
/// and not from any other field.
#[derive(Debug, Serialize)]
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

impl Eq for Product {}
impl PartialEq for Product {
    fn eq(&self, other: &Self) -> bool {
        self.sku.eq(&other.sku)
    }
}

impl std::hash::Hash for Product {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sku.hash(state);
    }
}
