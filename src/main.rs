const SITE_URL: &str = "https://www.countdown.co.nz";
const BASE_URL: &str = "https://www.countdown.co.nz/api/v1/";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "accept-language",
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
        );

        reqwest::Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .default_headers(default_headers)
            .build()
            .expect("Failed to create http client")
    };

    let res = client.get(SITE_URL).send().await?.text().await?;
    let start_api_key = res
        .find("apikey=")
        .expect("Failed to find start of API key")
        + "apikey=".len();
    let end_api_key = res[start_api_key..]
        .find('"')
        .map(|n| n + start_api_key)
        .expect("Failed to find end of API key");
    let full_api_key = &res[start_api_key..end_api_key];
    println!("API Key = {full_api_key:?}");

    std::fs::write("./output.html", res).expect("Failed to write output");

    println!("Hello, {BASE_URL}");
    Ok(())
}
