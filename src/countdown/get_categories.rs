use std::fmt::Display;

use reqwest::Client;
use serde::Deserialize;

/// Describes a category which can be searched
#[derive(Deserialize, Clone)]
pub struct Category {
    #[serde(rename = "label")]
    pub name: String,
    /// The url path to browse the category
    pub url: String,
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Category({0}@{1})", self.name, self.url))?;

        Ok(())
    }
}

/// Describes the response from the API when requesting the initial shell
#[derive(Deserialize)]
struct ShellResponse {
    browse: Vec<Category>,
}

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
