use core::fmt;

use error_stack::{Context, IntoReport, Result, ResultExt};
use reqwest::Client;
use serde::Deserialize;

use super::category::Category;

/// API response for a specific navigation item
#[derive(Deserialize)]
struct NavigationItem {
    items: Vec<Category>,
}

/// API response for nav menus
#[derive(Deserialize)]
struct ShellNav {
    #[serde(rename = "label")]
    name: String,
    #[serde(rename = "navigationItems")]
    navigation_items: Option<Vec<NavigationItem>>,
}

/// Describes the response from the API when requesting the initial shell
#[derive(Deserialize)]
struct ShellResponse {
    #[serde(rename = "mainNavs")]
    main_navs: Vec<ShellNav>,
}

#[derive(Debug)]
pub enum GetCategoriesError {
    HttpError,
    DecodeError,
}

impl fmt::Display for GetCategoriesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetCategoriesError::HttpError => write!(f, "Failed to perform an HTTP request."),
            GetCategoriesError::DecodeError => write!(
                f,
                "Failed to decode the JSON response to an appropriate type"
            ),
        }
    }
}

impl Context for GetCategoriesError {}

/// Retrieves all the categories in the store.
pub async fn get_categories(
    client: &Client,
    base_url: &str,
) -> Result<Vec<Category>, GetCategoriesError> {
    let res: ShellResponse = client
        .get(format!("{base_url}/shell"))
        .send()
        .await
        .into_report()
        .change_context(GetCategoriesError::HttpError)?
        .json()
        .await
        .into_report()
        .change_context(GetCategoriesError::DecodeError)?;

    // read res.mainNavs[1]
    let browse_page = res
        .main_navs
        .into_iter()
        .find(|nav| &nav.name == "Browse")
        .ok_or(GetCategoriesError::DecodeError)
        .into_report()
        .attach_printable("Failed to find Browse menu")?;

    // read res.mainNavs[1].navigationItems[0]
    let nav_items = browse_page
        .navigation_items
        .ok_or(GetCategoriesError::DecodeError)
        .into_report()
        .attach_printable("Failed to read navigation items")?
        .into_iter()
        .map(|nav_item| nav_item.items)
        .flatten()
        .collect();

    Ok(nav_items)
}
