use reqwest::Client;
use serde::Deserialize;

use super::category::Category;

/// Describes the response from the API when requesting the initial shell
#[derive(Deserialize)]
struct ShellResponse {
    browse: Vec<Category>,
}

/// Retrieves all the categories in the store.
pub async fn get_categories(
    client: &Client,
    base_url: &str,
) -> Result<Vec<Category>, reqwest::Error> {
    let res: ShellResponse = client
        .get(format!("{base_url}/shell"))
        .send()
        .await?
        .json()
        .await?;

    Ok(res.browse)
}
