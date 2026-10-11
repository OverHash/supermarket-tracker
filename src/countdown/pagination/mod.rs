use std::{collections::HashSet, future::Future};

use super::{Category, Product, get_products::GetProductResponse};

/// Fetches each category's advertised pages sequentially, without success delays.
/// HTTP is supplied by the caller so pagination can be tested independently.
///
/// Page 1 determines the page count. Errors stop retrieval immediately, and
/// cancelling collection drops the current fetch without detaching any work.
pub(super) async fn collect_products<F, Fut, E>(
    categories: Vec<Category>,
    mut fetch_page: F,
) -> Result<HashSet<Product>, E>
where
    F: FnMut(Category, i64) -> Fut,
    Fut: Future<Output = Result<GetProductResponse, E>>,
{
    let mut products = HashSet::new();

    for category in categories {
        let first = fetch_page(category.clone(), 1).await?;
        let total_pages = first.total_pages;
        products.extend(first.products);

        for page in 2..=total_pages {
            let response = fetch_page(category.clone(), page).await?;
            products.extend(response.products);
        }
    }

    Ok(products)
}

#[cfg(test)]
mod tests;
