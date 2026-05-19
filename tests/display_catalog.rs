//! Integration tests for the DisplayCatalog service.
//!
//! These tests make live HTTP requests to the Microsoft Store API and are
//! therefore marked `#[ignore]` by default.  Run them explicitly with:
//!
//!   cargo test --test display_catalog -- --ignored
//!
//! They require network access and will fail if the MS Store API is unreachable.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::str::FromStr;
use storelib_rs::cancellation::CancellationToken;
use storelib_rs::models::enums::{DCatEndpoint, DeviceFamily, IdentifierType};
use storelib_rs::models::locale::{Lang, LanguageTag, Locale, Market};
use storelib_rs::services::display_catalog::{ClientConfig, DisplayCatalogHandler, ProgressEvent};

// Netflix product ID — stable well-known app in the US store.
const NETFLIX_PRODUCT_ID: &str = "9WZDNCRFJ3TJ";
const NETFLIX_PFN: &str = "4DF9E0F8.Netflix_mcm4njqhnhss8";
// Two other stable apps for batch testing — Hulu and Disney+.
const HULU_PRODUCT_ID: &str = "9WZDNCRFJ3R8";
const DISNEY_PRODUCT_ID: &str = "9NXQXXLFST89";

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

    // Autosuggest returns a flat `Title` field; the per-product endpoint
    // puts titles under `LocalizedProperties[].ProductTitle`. Accept either.
    let any_netflix = result
        .results
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .flat_map(|g| g.products.as_deref().unwrap_or(&[]))
        .any(|p| {
            let flat = p.title.as_deref();
            let nested = p
                .localized_properties
                .as_deref()
                .and_then(|v| v.first())
                .and_then(|lp| lp.product_title.as_deref());
            flat.into_iter()
                .chain(nested)
                .any(|t| t.to_lowercase().contains("netflix"))
        });

    assert!(
        any_netflix,
        "At least one result should have 'Netflix' in the title"
    );
}

// ---------------------------------------------------------------------------
// Typed accessors
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn accessors_walk_live_netflix_listing() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let title = handler.title().expect("title should be set");
    assert!(
        title.to_lowercase().contains("netflix"),
        "title via accessor was {title:?}"
    );
    assert!(
        handler.publisher_name().is_some(),
        "publisher_name should be set"
    );
    // Netflix is free so price metadata may or may not be present; accessors
    // must at minimum not panic and return sensible values.
    let _ = handler.price();
    let _ = handler.availabilities();
    // packages() should be a slice (possibly empty for some product types).
    let _ = handler.packages();
    assert!(
        handler.wu_category_id().is_some(),
        "wu_category_id should be set for a downloadable app"
    );
}

// ---------------------------------------------------------------------------
// Batch query (bigIds)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn batch_query_returns_multiple_products() {
    let mut handler = make_handler();
    let ids = [NETFLIX_PRODUCT_ID, HULU_PRODUCT_ID, DISNEY_PRODUCT_ID];
    handler
        .query_dcat_batch(&ids, None)
        .await
        .expect("batch query should succeed");

    let products = handler.products();
    assert!(
        products.len() >= 2,
        "expected at least 2 products in batch response, got {}",
        products.len(),
    );
    // Every returned product should have a title.
    for p in products {
        let title = p
            .localized_properties
            .as_deref()
            .and_then(|v| v.first())
            .and_then(|lp| lp.product_title.as_deref());
        assert!(title.is_some(), "product missing ProductTitle: {p:?}");
    }
}

#[tokio::test]
#[ignore]
async fn batch_query_rejects_empty_ids() {
    let mut handler = make_handler();
    let err = handler
        .query_dcat_batch(&[], None)
        .await
        .expect_err("empty ids should error");
    assert!(matches!(err, storelib_rs::StoreError::Other(_)));
}

// ---------------------------------------------------------------------------
// FE3 package resolution (end-to-end)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn get_packages_for_netflix_returns_resolved_packages() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let packages = handler
        .get_packages_for_product(None)
        .await
        .expect("get_packages_for_product should succeed");

    assert!(
        !packages.is_empty(),
        "Netflix should resolve at least one package"
    );

    // Every resolved entry must have a moniker; at least one should carry a
    // package URL (frameworks may not).
    for pkg in &packages {
        assert!(!pkg.package_moniker.is_empty(), "empty moniker: {pkg:?}");
    }
    assert!(
        packages.iter().any(|p| p.package_uri.is_some()),
        "at least one package should have a download URI"
    );
    // At least the main package should have a size, sourced from DCat or FE3.
    assert!(
        packages.iter().any(|p| p.file_size.is_some()),
        "at least one package should have a non-null packageSize"
    );
    // FE3's <File FileName="..."> must be surfaced; at least one should
    // resolve into a readable filename ending with a known package extension.
    let known_exts = [
        ".appx",
        ".appxbundle",
        ".msix",
        ".msixbundle",
        ".eappx",
        ".eappxbundle",
        ".emsix",
        ".emsixbundle",
        ".xap",
    ];
    assert!(
        packages.iter().any(|p| p.file_name.is_some()),
        "at least one package should have a non-null fileName"
    );
    for p in &packages {
        assert!(
            !p.readable_file_name.is_empty(),
            "readable_file_name should always be set",
        );
        assert!(
            p.readable_file_name.starts_with(&p.package_moniker),
            "readable_file_name {:?} should start with the moniker {:?}",
            p.readable_file_name,
            p.package_moniker,
        );
        assert!(
            known_exts
                .iter()
                .any(|ext| p.readable_file_name.ends_with(ext)),
            "readable_file_name {:?} should end with a known package extension",
            p.readable_file_name,
        );
    }
}

// ---------------------------------------------------------------------------
// Localized query (non-US locale)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn query_with_german_locale_returns_localized_market_property() {
    let locale = Locale::new(Market::De, Lang::De, /* include_neutral */ true);
    let mut handler = DisplayCatalogHandler::new(DCatEndpoint::Production, locale);

    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat with DE locale should succeed");

    assert!(
        handler.is_found,
        "Netflix should be available in the DE market"
    );

    // The localized property should report a German `language` field, or fall
    // back to `en` if Netflix isn't translated for DE. Either is acceptable —
    // we just want to verify the request flowed through with the right locale
    // and parsed without error.
    let lang = handler
        .localized()
        .and_then(|lp| lp.language.as_deref())
        .map(str::to_lowercase);
    assert!(
        lang.is_some(),
        "localized property should carry a language tag"
    );
}

#[tokio::test]
#[ignore]
async fn locale_from_tag_drives_real_query() {
    // "en-GB" → market GB, language en, neutral=false.
    let tag = LanguageTag::from_str("en-GB").unwrap();
    let locale = Locale::from_tag(tag, false).unwrap();
    let mut handler = DisplayCatalogHandler::new(DCatEndpoint::Production, locale);

    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat with GB locale should succeed");

    assert!(handler.is_found);
    assert!(handler.title().is_some());
}

// ---------------------------------------------------------------------------
// Progress callback (live ordering)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn progress_callback_fires_expected_query_stages() {
    let mut handler = make_handler();
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let log_cb = log.clone();
    handler.progress.set(Box::new(move |e: ProgressEvent| {
        log_cb.lock().unwrap().push(e.stage);
    }));

    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let stages = log.lock().unwrap().clone();
    // The exact sequence on success is request → response → parse → done.
    assert!(
        stages.contains(&"dcat.request"),
        "expected dcat.request, got {stages:?}"
    );
    assert!(
        stages.contains(&"dcat.response"),
        "expected dcat.response, got {stages:?}"
    );
    assert!(
        stages.contains(&"dcat.parse"),
        "expected dcat.parse, got {stages:?}"
    );
    assert!(
        stages.contains(&"dcat.done"),
        "expected dcat.done, got {stages:?}"
    );
    // request must come before response, response before parse, parse before done.
    let pos = |s: &str| stages.iter().position(|x| *x == s);
    assert!(pos("dcat.request") < pos("dcat.response"));
    assert!(pos("dcat.response") < pos("dcat.parse"));
    assert!(pos("dcat.parse") < pos("dcat.done"));
}

#[tokio::test]
#[ignore]
async fn progress_callback_fires_fe3_stages_during_package_resolution() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let log_cb = log.clone();
    handler.progress.set(Box::new(move |e: ProgressEvent| {
        log_cb.lock().unwrap().push(e.stage);
    }));

    handler
        .get_packages_for_product(None)
        .await
        .expect("get_packages_for_product should succeed");

    let stages = log.lock().unwrap().clone();
    for expected in &[
        "fe3.start",
        "fe3.getCookie",
        "fe3.syncUpdates",
        "fe3.parseUpdateIds",
        "fe3.parsePackages",
        "fe3.resolveUrls",
        "fe3.done",
    ] {
        assert!(
            stages.contains(expected),
            "missing stage {expected}; got {stages:?}"
        );
    }
    let pos = |s: &str| stages.iter().position(|x| *x == s).unwrap();
    assert!(pos("fe3.start") < pos("fe3.getCookie"));
    assert!(pos("fe3.getCookie") < pos("fe3.syncUpdates"));
    assert!(pos("fe3.syncUpdates") < pos("fe3.parseUpdateIds"));
    assert!(pos("fe3.parseUpdateIds") < pos("fe3.parsePackages"));
    assert!(pos("fe3.parsePackages") < pos("fe3.resolveUrls"));
    assert!(pos("fe3.resolveUrls") < pos("fe3.done"));
}

// ---------------------------------------------------------------------------
// Cancellation against real endpoints
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn precancelled_token_short_circuits_query() {
    let token = CancellationToken::new();
    token.cancel(); // already cancelled before the call

    let mut handler = make_handler();
    let err = handler
        .query_dcat_with_cancel(
            NETFLIX_PRODUCT_ID,
            IdentifierType::ProductId,
            None,
            Some(&token),
        )
        .await
        .expect_err("pre-cancelled token should short-circuit");
    assert!(matches!(err, storelib_rs::StoreError::Cancelled));
}

#[tokio::test]
#[ignore]
async fn mid_flight_cancel_aborts_package_resolution() {
    // get_packages_for_product runs three sequential SOAP POSTs; cancelling
    // 200ms in lands during one of the FE3 calls, which should drop cleanly.
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let token = CancellationToken::new();
    let canceller = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        canceller.cancel();
    });

    let started = std::time::Instant::now();
    let err = handler
        .get_packages_for_product_with_cancel(None, Some(&token))
        .await
        .expect_err("cancellation mid-flight should error");
    let elapsed = started.elapsed();

    assert!(
        matches!(err, storelib_rs::StoreError::Cancelled),
        "expected Cancelled, got {err:?}",
    );
    // FE3 normally takes >1s end-to-end; cancel at 200ms should resolve
    // comfortably under 5s.
    assert!(
        elapsed < Duration::from_secs(5),
        "cancel took too long: {elapsed:?}",
    );
}

// ---------------------------------------------------------------------------
// Search variations
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn paged_search_accepts_skip_count_without_erroring() {
    // The autosuggest endpoint silently ignores `skipItems` and always
    // returns the same first ~10 entries. We can still verify that
    // search_dcat_paged accepts a non-zero `skip_count` without erroring
    // and that both calls deserialize cleanly.
    let mut handler = make_handler();
    let first = handler
        .search_dcat_paged("game", DeviceFamily::Desktop, 0)
        .await
        .expect("first page search should succeed");
    let _skipped = handler
        .search_dcat_paged("game", DeviceFamily::Desktop, 100)
        .await
        .expect("search_dcat_paged with skip=100 should still succeed");
    assert!(
        first.total_result_count.unwrap_or(0) > 0,
        "search for 'game' should return results"
    );
}

#[tokio::test]
#[ignore]
async fn search_with_xbox_device_family_returns_results() {
    let mut handler = make_handler();
    let result = handler
        .search_dcat("halo", DeviceFamily::Xbox)
        .await
        .expect("Xbox-family search should succeed");
    let total = result.total_result_count.unwrap_or(0);
    assert!(total > 0, "search for Halo on Xbox should return results");
}

// ---------------------------------------------------------------------------
// Typed accessor fan-out
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn images_with_purpose_returns_at_least_one_logo() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let logos = handler.images_with_purpose("Logo");
    assert!(
        !logos.is_empty(),
        "Netflix should publish at least one Logo image"
    );
    // Logo URLs are typically protocol-relative (`//store-images...`).
    assert!(
        logos.iter().any(|img| img.uri.is_some()),
        "logo should carry a URI"
    );
}

#[tokio::test]
#[ignore]
async fn wu_category_id_matches_explicit_fulfillment_walk() {
    let mut handler = make_handler();
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("query_dcat should succeed");

    let via_accessor = handler
        .wu_category_id()
        .expect("accessor should return Some");
    let via_walk = handler
        .product()
        .and_then(|p| p.display_sku_availabilities.as_deref())
        .and_then(|v| v.first())
        .and_then(|dsa| dsa.sku.as_ref())
        .and_then(|sku| sku.properties.as_ref())
        .and_then(|props| props.fulfillment_data.as_ref())
        .and_then(|fd| fd.wu_category_id.as_deref())
        .expect("manual walk should also yield wu_category_id");
    assert_eq!(via_accessor, via_walk);
}

// ---------------------------------------------------------------------------
// Custom ClientConfig
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn custom_client_config_user_agent_does_not_break_query() {
    let cfg = ClientConfig {
        user_agent: "storelib_rs-test/1.0".into(),
        max_retries: 1,
        ..Default::default()
    };
    let mut handler =
        DisplayCatalogHandler::with_config(DCatEndpoint::Production, Locale::production(), cfg);
    handler
        .query_dcat(NETFLIX_PRODUCT_ID, IdentifierType::ProductId, None)
        .await
        .expect("custom-UA query should succeed");
    assert!(handler.is_found);
}
