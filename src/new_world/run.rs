use super::NEW_WORLD_BASE_URL;
use error_stack::{Report, ResultExt};
use reqwest::Client;
use serde::Deserialize;

use crate::error::ApplicationError;

#[derive(Deserialize)]
struct Store {
    /// A version 4 uuid of the store to identify it.
    id: String,
    /// The name of the store.
    name: String,
}

#[derive(Deserialize)]
struct GetStoreList {
    stores: Vec<Store>,
}

pub async fn run() -> Result<(), Report<ApplicationError>> {
    let client = Client::new();

    // retrieve all stores
    let stores: GetStoreList = client
        .get(format!("{NEW_WORLD_BASE_URL}/Store/GetStoreList"))
        .send()
        .await
        .change_context(ApplicationError::HttpError)?
        .json()
        .await
        .change_context(ApplicationError::HttpError)?;

    Ok(())
}
