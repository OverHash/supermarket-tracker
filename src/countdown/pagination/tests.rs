use std::{
    future::{Future, Ready, ready},
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
};

use tokio::sync::{mpsc, oneshot};

use super::{Category, GetProductResponse, Product, collect_products};

#[derive(Debug, Eq, PartialEq)]
enum TestError {
    FetchFailed,
}

struct PendingRequest {
    category: Category,
    page: i64,
    response: oneshot::Sender<Result<GetProductResponse, TestError>>,
}

impl PendingRequest {
    fn respond(self, total_pages: i64) {
        self.response
            .send(Ok(response(&self.category.name, self.page, total_pages)))
            .expect("Collection dropped a fetch before its response was supplied");
    }

    fn fail(self) {
        self.response
            .send(Err(TestError::FetchFailed))
            .expect("Collection dropped a fetch before its error was supplied");
    }
}

fn category(name: &str) -> Category {
    Category {
        name: name.to_owned(),
        url: format!("/shop/browse/{name}"),
    }
}

fn response(category: &str, page: i64, total_pages: i64) -> GetProductResponse {
    GetProductResponse {
        products: [Product {
            name: format!("{category} page {page}"),
            barcode: format!("{category}-{page}"),
            sku: format!("{category}-{page}"),
            per_unit_price: 150,
        }]
        .into(),
        next_page: (page < total_pages).then_some(page + 1),
        total_pages,
    }
}

/// Records a fetch when polled, then holds its response until the test supplies
/// one. No network, runtime, spawned tasks, or wall-clock time is involved.
async fn fetch_page(
    requests: mpsc::UnboundedSender<PendingRequest>,
    category: Category,
    page: i64,
) -> Result<GetProductResponse, TestError> {
    let (response, receiver) = oneshot::channel();
    requests
        .send(PendingRequest {
            category,
            page,
            response,
        })
        .expect("The test stopped observing fetches");
    receiver.await.expect("The test dropped a response sender")
}

/// Tests explicitly poll collection after supplying responses. Nothing depends
/// on background task scheduling, so a no-op waker suffices.
fn drive<F: Future>(collection: Pin<&mut F>) -> Poll<F::Output> {
    collection.poll(&mut Context::from_waker(Waker::noop()))
}

/// Page 1 announces three pages. Pages 2 and 3 must each start immediately after
/// the preceding response, but never while that response is still pending.
/// This protects page discovery and sequential fetching without a timer.
#[test]
fn one_category_discovers_and_retrieves_every_page_once() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let collection = collect_products(vec![category("fruit")], |category, page| {
        fetch_page(requests_tx.clone(), category, page)
    });
    let mut collection = pin!(collection);

    for expected_page in 1..=3 {
        assert!(drive(collection.as_mut()).is_pending());
        let request = requests.try_recv().expect("The next page should start");
        assert_eq!(request.page, expected_page);
        assert!(requests.try_recv().is_err(), "Only one fetch may start");
        assert!(drive(collection.as_mut()).is_pending());
        assert!(
            requests.try_recv().is_err(),
            "Wait for the current response"
        );
        request.respond(3);
    }

    let Poll::Ready(Ok(products)) = drive(collection.as_mut()) else {
        panic!("Collection should finish after the last page responds");
    };
    let mut skus: Vec<_> = products
        .iter()
        .map(|product| product.sku.as_str())
        .collect();
    skus.sort_unstable();
    assert_eq!(skus, ["fruit-1", "fruit-2", "fruit-3"]);
    assert!(
        requests.try_recv().is_err(),
        "No page should be fetched twice"
    );
}

#[test]
fn categories_are_completed_sequentially() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let collection = collect_products(
        vec![category("fruit"), category("dairy")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    );
    let mut collection = pin!(collection);

    for (expected_category, expected_page, total_pages) in
        [("fruit", 1, 2), ("fruit", 2, 2), ("dairy", 1, 1)]
    {
        assert!(drive(collection.as_mut()).is_pending());
        let request = requests.try_recv().unwrap();
        assert_eq!(request.category.name, expected_category);
        assert_eq!(request.page, expected_page);
        assert!(requests.try_recv().is_err(), "Only one fetch may start");
        request.respond(total_pages);
    }

    let Poll::Ready(Ok(products)) = drive(collection.as_mut()) else {
        panic!("Collection should finish after both categories respond");
    };
    assert_eq!(products.len(), 3);
    assert!(requests.try_recv().is_err());
}

#[test]
fn no_categories_finishes_without_fetching() {
    let collection = collect_products(
        Vec::new(),
        |_, _| -> Ready<Result<GetProductResponse, TestError>> {
            panic!("An empty category list must not fetch anything");
        },
    );
    let Poll::Ready(Ok(products)) = drive(pin!(collection)) else {
        panic!("An empty category list should finish immediately");
    };
    assert!(products.is_empty());
}

#[test]
fn completion_waits_for_the_current_fetch() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let collection = collect_products(vec![category("fruit")], |category, page| {
        fetch_page(requests_tx.clone(), category, page)
    });
    let mut collection = pin!(collection);

    assert!(drive(collection.as_mut()).is_pending());
    let request = requests.try_recv().unwrap();
    assert!(drive(collection.as_mut()).is_pending());
    assert!(requests.try_recv().is_err());

    request.respond(1);
    let Poll::Ready(Ok(products)) = drive(collection.as_mut()) else {
        panic!("Collection should finish once its only fetch responds");
    };
    assert_eq!(products.len(), 1);
}

#[test]
fn duplicate_skus_across_pages_and_categories_are_collected_once() {
    let mut requested = Vec::new();
    let collection = collect_products(
        vec![category("fruit"), category("dairy")],
        |category, page| {
            requested.push((category.name, page));
            ready(Ok::<_, TestError>(response("shared", 1, 2)))
        },
    );
    let Poll::Ready(Ok(products)) = drive(pin!(collection)) else {
        panic!("Ready responses should finish collection immediately");
    };
    assert_eq!(products.len(), 1);
    assert_eq!(products.iter().next().unwrap().sku, "shared-1");
    assert_eq!(
        requested,
        [
            ("fruit".to_owned(), 1),
            ("fruit".to_owned(), 2),
            ("dairy".to_owned(), 1),
            ("dairy".to_owned(), 2)
        ]
    );
}

#[test]
fn an_empty_first_page_does_not_skip_other_advertised_pages() {
    let mut requested = Vec::new();
    let collection = collect_products(vec![category("fruit")], |category, page| {
        requested.push(page);
        let mut response = response(&category.name, page, 2);
        if page == 1 {
            response.products.clear();
            response.next_page = None;
        }
        ready(Ok::<_, TestError>(response))
    });
    let Poll::Ready(Ok(products)) = drive(pin!(collection)) else {
        panic!("Ready responses should finish collection immediately");
    };
    assert_eq!(products.len(), 1);
    assert_eq!(requested, [1, 2]);
}

/// Cancellation drops the current operation; later pages/categories never start.
#[test]
fn dropping_collection_cancels_the_current_fetch() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let mut collection = Box::pin(collect_products(
        vec![category("fruit"), category("dairy")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    ));

    assert!(drive(collection.as_mut()).is_pending());
    let initial = requests.try_recv().unwrap();
    initial.respond(3);
    assert!(drive(collection.as_mut()).is_pending());
    let current = requests.try_recv().unwrap();
    assert_eq!(current.page, 2);
    drop(collection);

    assert!(current.response.is_closed());
    assert!(
        requests.try_recv().is_err(),
        "No work may start after cancellation"
    );
}

/// A later-page failure must stop remaining pages and subsequent categories.
#[test]
fn a_fetch_error_stops_remaining_pages_and_categories() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let collection = collect_products(
        vec![category("fruit"), category("dairy")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    );
    let mut collection = pin!(collection);

    assert!(drive(collection.as_mut()).is_pending());
    let initial = requests.try_recv().unwrap();
    initial.respond(3);
    assert!(drive(collection.as_mut()).is_pending());
    let current = requests.try_recv().unwrap();
    assert_eq!(current.page, 2);
    current.fail();

    assert_eq!(
        drive(collection.as_mut()),
        Poll::Ready(Err(TestError::FetchFailed))
    );
    assert!(
        requests.try_recv().is_err(),
        "No work may start after failure"
    );
}
