mod category;
mod get_categories;
mod get_products;
mod product;
mod run;

pub use category::Category;
pub use get_categories::get_categories;
pub use get_products::{get_all_products, get_products};
pub use product::Product;
pub use run::run;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";
pub const COUNTDOWN_BASE_URL: &str = "https://www.countdown.co.nz/api/v1";
