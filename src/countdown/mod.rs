mod category;
mod get_categories;
mod get_products;
mod product;

pub use category::Category;
pub use get_categories::get_categories;
pub use get_products::{get_all_products, get_products};
pub use product::Product;

pub const COUNTDOWN_BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
