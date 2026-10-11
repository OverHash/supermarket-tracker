use std::{
    collections::{HashSet, VecDeque},
    future::Future,
};

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::time::sleep;

use crate::{CONCURRENT_REQUESTS, PAGE_ITERATION_INTERVAL};

use super::{Category, Product, get_products::GetProductResponse};

struct PageRequest {
    category: Category,
    page: i64,
}

/// Owns pending pages, active fetches, and per-slot cooldowns. HTTP is supplied
/// by the caller; no mutex or independently spawned worker task is needed.
///
/// An empty pending queue is not completion while a fetch can discover pages.
/// Errors and cancellation drop all owned fetch futures together.
pub(super) async fn collect_products<F, Fut, E>(
    categories: Vec<Category>,
    mut fetch_page: F,
) -> Result<HashSet<Product>, E>
where
    F: FnMut(Category, i64) -> Fut,
    Fut: Future<Output = Result<GetProductResponse, E>>,
{
    let mut pending: VecDeque<_> = categories
        .into_iter()
        .map(|category| PageRequest { category, page: 1 })
        .collect();
    let mut active = FuturesUnordered::new();
    let mut cooldowns = FuturesUnordered::new();
    let mut available_slots = CONCURRENT_REQUESTS;
    let mut products = HashSet::new();

    loop {
        while available_slots > 0 {
            let Some(request) = pending.pop_front() else {
                break;
            };
            let fetch = fetch_page(request.category.clone(), request.page);
            active.push(async move { (request, fetch.await) });
            available_slots -= 1;
        }

        if pending.is_empty() && active.is_empty() {
            // Cooldowns only govern the next request. No need to wait for them
            // when there is no more work and nothing can discover more pages.
            return Ok(products);
        }

        tokio::select! {
            Some((request, response)) = active.next(), if !active.is_empty() => {
                let response = response?;
                if request.page == 1 {
                    // Add all the future pages for this category
                    pending.extend((2..=response.total_pages).map(|page| PageRequest {
                        category: request.category.clone(),
                        page,
                    }));
                }

                // Store response products
                products.extend(response.products);

                // Keep the completed slot occupied during its cooldown, but
                // let an unused slot start newly discovered work immediately.
                cooldowns.push(sleep(PAGE_ITERATION_INTERVAL));
            }
            Some(()) = cooldowns.next(), if !cooldowns.is_empty() => {
                available_slots += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests;
