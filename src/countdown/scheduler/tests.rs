use std::{
    future::Future,
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
    time::Duration,
};

use tokio::{
    sync::{mpsc, oneshot},
    time::advance,
};

use super::{Category, GetProductResponse, PAGE_ITERATION_INTERVAL, Product, collect_products};

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
            .send(Ok(GetProductResponse {
                products: [Product {
                    name: format!("{} page {}", self.category.name, self.page),
                    barcode: format!("{}-{}", self.category.name, self.page),
                    sku: format!("{}-{}", self.category.name, self.page),
                    per_unit_price: 150,
                }]
                .into(),
                next_page: (self.page < total_pages).then_some(self.page + 1),
                total_pages,
            }))
            .expect("The scheduler dropped a fetch before its response was supplied");
    }

    fn fail(self) {
        self.response
            .send(Err(TestError::FetchFailed))
            .expect("The scheduler dropped a fetch before its error was supplied");
    }
}

fn category(name: &str) -> Category {
    Category {
        name: name.to_owned(),
        url: format!("/shop/browse/{name}"),
    }
}

/// Records a fetch when it is actually polled, then holds its response until
/// the test sends one. No network, spawned tasks, or wall-clock time involved.
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

/// Tests explicitly drive the scheduler after supplying responses or advancing
/// time. A no-op waker suffices: nothing is left to background task scheduling.
fn drive<F: Future>(scheduler: Pin<&mut F>) -> Poll<F::Output> {
    scheduler.poll(&mut Context::from_waker(Waker::noop()))
}

/// Checks that a temporarily empty queue does not prevent newly discovered
/// pages from being fetched concurrently.
///
/// Start with only page 1, then supply its response announcing three pages.
/// After advancing past the cooldown, pages 2 and 3 must both have started,
/// WITHOUT completing either response. A scheduler with just one surviving
/// worker starts page 2 but not page 3; `try_recv` then fails immediately.
#[tokio::test(start_paused = true)]
async fn workers_remain_available_for_pages_discovered_by_one_category() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let scheduler = collect_products(vec![category("fruit")], |category, page| {
        fetch_page(requests_tx.clone(), category, page)
    });
    let mut scheduler = pin!(scheduler);

    assert!(drive(scheduler.as_mut()).is_pending());
    let initial = requests.try_recv().expect("Page 1 should have started");
    assert_eq!(initial.page, 1);
    assert!(requests.try_recv().is_err());

    initial.respond(3);
    assert!(drive(scheduler.as_mut()).is_pending());
    advance(PAGE_ITERATION_INTERVAL).await;
    assert!(drive(scheduler.as_mut()).is_pending());

    let first = requests
        .try_recv()
        .expect("One discovered page should have started");
    // Keep the first response pending. The other page must start independently,
    // not only after this fetch finishes. This is the regression assertion.
    let second = requests.try_recv().expect(
        "The other discovered page must start while the first response is held; \
         a temporarily empty queue must not retire idle workers",
    );
    let mut pages = [first.page, second.page];
    pages.sort_unstable();
    assert_eq!(pages, [2, 3]);
    assert!(requests.try_recv().is_err());

    first.respond(3);
    second.respond(3);
    let Poll::Ready(Ok(products)) = drive(scheduler.as_mut()) else {
        panic!("The scrape should finish once all discovered pages have responded");
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

#[tokio::test(start_paused = true)]
async fn no_categories_finishes_without_fetching() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let scheduler = collect_products(Vec::new(), |category, page| {
        fetch_page(requests_tx.clone(), category, page)
    });
    let mut scheduler = pin!(scheduler);
    let Poll::Ready(Ok(products)) = drive(scheduler.as_mut()) else {
        panic!("An empty category list should finish immediately");
    };
    assert!(products.is_empty());
    assert!(requests.try_recv().is_err());
}

/// An empty queue cannot end the scrape while page 1 is still in flight.
#[tokio::test(start_paused = true)]
async fn completion_waits_for_in_flight_fetches() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let scheduler = collect_products(vec![category("fruit")], |category, page| {
        fetch_page(requests_tx.clone(), category, page)
    });
    let mut scheduler = pin!(scheduler);

    assert!(drive(scheduler.as_mut()).is_pending());
    let initial = requests.try_recv().unwrap();
    assert!(requests.try_recv().is_err());
    assert!(drive(scheduler.as_mut()).is_pending());

    initial.respond(1);
    let Poll::Ready(Ok(products)) = drive(scheduler.as_mut()) else {
        panic!("With no more pages, the completed fetch should finish the scrape");
    };
    assert_eq!(products.len(), 1);
}

/// Two held responses occupy both slots. Completing one does not let a third
/// fetch start until that slot's full cooldown has elapsed.
#[tokio::test(start_paused = true)]
async fn concurrency_is_bounded_and_completed_slots_have_a_cooldown() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let scheduler = collect_products(
        vec![category("fruit"), category("dairy"), category("bakery")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    );
    let mut scheduler = pin!(scheduler);

    assert!(drive(scheduler.as_mut()).is_pending());
    let first = requests.try_recv().unwrap();
    let second = requests.try_recv().unwrap();
    assert!(requests.try_recv().is_err(), "Only two fetches may start");

    first.respond(1);
    assert!(drive(scheduler.as_mut()).is_pending());
    assert!(
        requests.try_recv().is_err(),
        "The completed slot is cooling down"
    );

    let tick = Duration::from_millis(1);
    advance(
        PAGE_ITERATION_INTERVAL
            .checked_sub(tick)
            .expect("The cooldown must be at least one timer tick"),
    )
    .await;
    assert!(drive(scheduler.as_mut()).is_pending());
    assert!(
        requests.try_recv().is_err(),
        "The cooldown must not end early"
    );

    advance(tick).await;
    assert!(drive(scheduler.as_mut()).is_pending());
    let third = requests
        .try_recv()
        .expect("The cooled slot should start the next fetch");
    assert_eq!(third.category.name, "bakery");
    assert!(requests.try_recv().is_err());

    second.respond(1);
    third.respond(1);
    let Poll::Ready(Ok(products)) = drive(scheduler.as_mut()) else {
        panic!("The scrape should finish once all categories have responded");
    };
    assert_eq!(products.len(), 3);
}

/// Cancelling the scrape drops its active operations instead of detaching them.
#[tokio::test(start_paused = true)]
async fn dropping_the_scheduler_cancels_active_fetches() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let mut scheduler = Box::pin(collect_products(
        vec![category("fruit"), category("dairy"), category("bakery")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    ));

    assert!(drive(scheduler.as_mut()).is_pending());
    let first = requests.try_recv().unwrap();
    let second = requests.try_recv().unwrap();
    drop(scheduler);

    assert!(first.response.is_closed());
    assert!(second.response.is_closed());
    assert!(
        requests.try_recv().is_err(),
        "Pending work must not start after cancellation"
    );
}

/// A fetch error returns immediately and cancels peers, even if their responses
/// are still pending. Queued work must not start after failure.
#[tokio::test(start_paused = true)]
async fn a_fetch_error_cancels_other_fetches() {
    let (requests_tx, mut requests) = mpsc::unbounded_channel();
    let scheduler = collect_products(
        vec![category("fruit"), category("dairy"), category("bakery")],
        |category, page| fetch_page(requests_tx.clone(), category, page),
    );
    let mut scheduler = pin!(scheduler);

    assert!(drive(scheduler.as_mut()).is_pending());
    let first = requests.try_recv().unwrap();
    let second = requests.try_recv().unwrap();
    first.fail();

    let Poll::Ready(Err(error)) = drive(scheduler.as_mut()) else {
        panic!("A fetch error must finish the scrape without waiting for its peers");
    };
    assert_eq!(error, TestError::FetchFailed);
    assert!(second.response.is_closed());
    assert!(
        requests.try_recv().is_err(),
        "Pending work must not start after failure"
    );
}
