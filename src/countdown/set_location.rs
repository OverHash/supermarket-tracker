use error_stack::Result;
use reqwest::Client;
use serde_json::json;

/// Sets the location of the countdown store to a specified ID.
#[tracing::instrument(
	name = "set store location",
	level = "debug",
	skip_all,
	fields(
		%store_id,
	)
)]
pub async fn set_location(
    client: &Client,
    base_url: &str,
    store_id: i32,
) -> Result<(), reqwest::Error> {
    client
        .put(&format!("{base_url}/fulfilment/my/pickup-addresses"))
        .json(&json!({
            "addressId": store_id
        }))
        .send()
        .await?;

    Ok(())
}
