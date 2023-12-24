use error_stack::Result;
use reqwest::Client;
use serde_json::json;

/// Sets the fulfillment method to be "pickup", so we can set precise store locations.
#[tracing::instrument(
    name = "set fulfillment method",
    level = "debug",
    skip_all,
    fields(method = "pickup")
)]
pub async fn set_fulfillment_method(client: &Client, base_url: &str) -> Result<(), reqwest::Error> {
    client
        .put(&format!("{base_url}/fulfilment/my/methods/pickup"))
        .json(&json!({}))
        .send()
        .await?;

    Ok(())
}
