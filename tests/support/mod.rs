use std::{io, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};

/// Query parameters received by the fake Countdown API.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ProductRequest {
    pub page: i64,
    pub size: i32,
    pub target: String,
    #[serde(rename = "dasFilter")]
    pub department: String,
}

struct ApiState {
    requests: mpsc::UnboundedSender<ProductRequest>,
}

/// A per-test HTTP server that records requests and serves pagination fixtures.
pub struct TestApi {
    pub base_url: String,
    requests: mpsc::UnboundedReceiver<ProductRequest>,
    shutdown: Option<oneshot::Sender<()>>,
    server: JoinHandle<io::Result<()>>,
}

impl TestApi {
    pub async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind the fake API to an available port");
        let address = listener.local_addr().expect("Failed to read the API port");
        let (requests_tx, requests) = mpsc::unbounded_channel();
        let state = Arc::new(ApiState {
            requests: requests_tx,
        });
        let router = Router::new()
            .route("/api/v1/products", get(products))
            .with_state(state);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        Self {
            base_url: format!("http://{address}/api/v1"),
            requests,
            shutdown: Some(shutdown_tx),
            server,
        }
    }

    /// Call after the scraper finishes to check for duplicated requests too.
    pub fn drain_requests(&mut self) -> Vec<ProductRequest> {
        let mut requests = Vec::new();
        while let Ok(request) = self.requests.try_recv() {
            requests.push(request);
        }
        requests
    }

    pub async fn shutdown(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        timeout(Duration::from_secs(5), &mut self.server)
            .await
            .expect("The fake API did not shut down")
            .expect("The fake API task panicked")
            .expect("The fake API server failed");
    }
}

impl Drop for TestApi {
    fn drop(&mut self) {
        // Also clean up if an assertion panics before explicit shutdown.
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.server.abort();
    }
}

async fn products(
    State(state): State<Arc<ApiState>>,
    Query(request): Query<ProductRequest>,
) -> Json<Value> {
    // Only three fixture requests: recording them must not add backpressure.
    let _ = state.requests.send(request.clone());
    // One representative product per page is sufficient. totalItems tells the
    // real pagination code to discover three pages at its page size of 120.
    Json(json!({
        "products": {
            "totalItems": 360,
            "items": [{
                "type": "Product",
                "name": format!("Product on page {}", request.page),
                "barcode": format!("barcode-{}", request.page),
                "sku": format!("sku-{}", request.page),
                "price": {
                    "originalPrice": 2.0,
                    "salePrice": 1.5,
                    "savePrice": 0.5
                }
            }]
        }
    }))
}
