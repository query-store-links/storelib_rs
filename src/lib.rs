//! `storelib_rs` – Rust port of StoreLib, a Microsoft Store API client.
//!
//! Supports both native (tokio) and WASM (wasm-bindgen-futures) async
//! runtimes.  Enable the `wasm` feature when targeting `wasm32-unknown-unknown`.

// ---------------------------------------------------------------------------
// WASM initialisation
// ---------------------------------------------------------------------------

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Initialise panic hook for better WASM error messages (no-op on native).
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn wasm_init() {
    // console_error_panic_hook would go here if added as a dependency.
}

// ---------------------------------------------------------------------------
// Public modules
// ---------------------------------------------------------------------------

pub mod error;
pub mod models;
pub mod services;
pub mod utilities;

// ---------------------------------------------------------------------------
// Convenient re-exports at the crate root
// ---------------------------------------------------------------------------

// error
pub use error::StoreError;

// models
pub use models::addon::Addon;
pub use models::catalog::{
    AllowedPlatform, AlternateId, Application, Availability, AvailabilityProperties,
    ClientConditions, Conditions, ContentRating, DisplayCatalogModel, DisplaySkuAvailability,
    FulfillmentData, HardwareProperties, Image, LegalText, OrderManagementData, Package,
    PackageDownloadUri, PiFilter, PlatformDependency, Price, Product, ProductLocalizedProperty,
    ProductMarketProperty, ProductProperties, SearchTitle, Sku, SkuLocalizedProperty,
    SkuMarketProperty, SkuProperties, UsageDatum, ValidationData,
};
pub use models::enums::{
    DCatEndpoint, DeviceFamily, DisplayCatalogResult, IdentifierType, ImagePurpose, PackageType,
    ProductKind,
};
pub use models::fe3::{ApplicabilityBlob, ContentTargetPlatform, PackageInstance};
pub use models::locale::{Lang, Locale, Market};
pub use models::search::{DCatSearch, SearchResult};

// services
pub use services::display_catalog::DisplayCatalogHandler;
pub use services::fe3::FE3Handler;

// utilities
pub use utilities::helpers::{
    create_dcat_uri, endpoint_to_base_url, endpoint_to_search_url, string_to_package_type,
};
