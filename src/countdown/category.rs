use std::fmt::Display;

use serde::Deserialize;

/// Describes a category which can be searched
#[derive(Deserialize, Clone)]
pub struct Category {
    /// The name of the category
    #[serde(rename = "label")]
    pub name: String,
    /// The url path to browse the category
    pub url: String,
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Category({0}@{1:?})", self.name, self.url))?;

        Ok(())
    }
}
