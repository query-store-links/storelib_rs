//! Integration tests for the DisplayCatalog service.
//!
//! These tests make live HTTP requests to the Microsoft Store API and are
//! therefore marked `#[ignore]` by default.  Run them explicitly with:
//!
//!   cargo test --test display_catalog -- --ignored
//!
//! They require network access and will fail if the MS Store API is unreachable.

use storelib_rs::models::enums::{DeviceFamily, IdentifierType};
use storelib_rs::services::display_catalog::DisplayCatalogHandler;

// Netflix product ID — stable well-known app in the US store.
const NETFLIX_PRODUCT_ID: &str = "9WZDNCRFJ3TJ";
const NETFLIX_PFN: &str = "4DF9E0F8.Netflix_mcm4njqhnhss8";

fn make_handler() -> DisplayCatalogHandler {
    DisplayCatalogHandler::production()
}

// ---------------------------------------------------------------------------
// Query by ProductId
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn query_netflix_by_product_id_is_found() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    assert!(
        handler.is_found,
        "Netflix should be found in the US catalog"
    );
}

#[tokio::test]
#[ignore]
async fn query_netflix_title_contains_netflix() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let listing = handler
        .product_listing
        .expect("product_listing should be set");
    let product = listing.product.expect("Product should be present");
    let props = product
        .localized_properties
        .expect("LocalizedProperties should be present");
    let title = props[0]
        .product_title
        .as_deref()
        .expect("ProductTitle should be set");
    assert!(
        title.to_lowercase().contains("netflix"),
        "Title '{title}' should contain 'Netflix'"
    );
}

// ---------------------------------------------------------------------------
// Query by PackageFamilyName
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn query_netflix_by_package_family_name() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PFN, IdentifierType::PackageFamilyName, None)
        .await
        .expect("query_dcat by PFN should succeed");

    assert!(handler.is_found, "Netflix should be found by PFN");
}

// ---------------------------------------------------------------------------
// Not-found product
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn query_nonexistent_product_is_not_found() {
    let mut handler = make_handler();
    let result = handler
        .query_dcat("0000AAAANOTREAL", IdentifierType::ProductId, None)
        .await;

    match result {
        Ok(_) => assert!(!handler.is_found, "Nonexistent product should not be found"),
        Err(storelib_rs::StoreError::NotFound) => {}
        Err(e) => panic!("Unexpected error querying nonexistent product: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn search_netflix_returns_results() {
    let mut handler = make_handler();
    let result = handler
        .search_dcat("netflix", DeviceFamily::Desktop)
        .await
        .expect("search_dcat should succeed");

    let count = result.total_result_count.unwrap_or(0);
    assert!(count > 0, "Search for 'netflix' should return results");
}

#[tokio::test]
#[ignore]
async fn search_results_contain_relevant_title() {
    let mut handler = make_handler();
    let result = handler
        .search_dcat("netflix", DeviceFamily::Desktop)
        .await
        .expect("search_dcat should succeed");

    // Results are grouped by product family; check any product title.
    let any_netflix = result
        .results
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .flat_map(|g| g.products.as_deref().unwrap_or(&[]))
        .flat_map(|p| p.localized_properties.as_deref().unwrap_or(&[]))
        .filter_map(|lp| lp.product_title.as_deref())
        .any(|t| t.to_lowercase().contains("netflix"));

    assert!(
        any_netflix,
        "At least one result should have 'Netflix' in the title"
    );
}
