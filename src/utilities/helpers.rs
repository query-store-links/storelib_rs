use crate::models::enums::{DCatEndpoint, IdentifierType, PackageType};
use crate::models::locale::Locale;

// ---------------------------------------------------------------------------
// Package type helpers
// ---------------------------------------------------------------------------

/// Convert the raw string from an `AppxMetadata` XML attribute to a
/// [`PackageType`] variant.
pub fn string_to_package_type(raw: &str) -> PackageType {
    match raw {
        "XAP" => PackageType::Xap,
        "AppX" => PackageType::AppX,
        "UAP" => PackageType::Uap,
        _ => PackageType::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Endpoint URL tables
// ---------------------------------------------------------------------------

/// Returns the base URL for a DisplayCatalog product endpoint.
pub fn endpoint_to_base_url(endpoint: &DCatEndpoint) -> &'static str {
    match endpoint {
        DCatEndpoint::Production => "https://displaycatalog.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::Int => "https://displaycatalog-int.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::Xbox => "https://xbox-displaycatalog.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::XboxInt => "https://xbox-displaycatalog-int.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::Dev => "https://displaycatalog-dev.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::OneP => "https://displaycatalog1p.mp.microsoft.com/v7.0/products/",
        DCatEndpoint::OnePInt => "https://displaycatalog1p-int.mp.microsoft.com/v7.0/products/",
    }
}

/// Returns the base URL for a DisplayCatalog autosuggest search endpoint.
pub fn endpoint_to_search_url(endpoint: &DCatEndpoint) -> &'static str {
    match endpoint {
        DCatEndpoint::Int =>
            "https://displaycatalog-int.mp.microsoft.com/v7.0/productFamilies/autosuggest?market=US&languages=en-US&query=",
        _ =>
            "https://displaycatalog.mp.microsoft.com/v7.0/productFamilies/autosuggest?market=US&languages=en-US&query=",
    }
}

// ---------------------------------------------------------------------------
// URI construction
// ---------------------------------------------------------------------------

/// Build a full DisplayCatalog request URL from its components.
///
/// Mirrors `UriHelpers.CreateAlternateDCatUri` from the C# original.
pub fn create_dcat_uri(
    endpoint: &DCatEndpoint,
    id: &str,
    id_type: &IdentifierType,
    locale: &Locale,
) -> String {
    let base = endpoint_to_base_url(endpoint);
    let trail = locale.dcat_trail();

    match id_type {
        IdentifierType::ProductId =>
            format!("{}{id}?{trail}&fieldsTemplate=Details", base),

        IdentifierType::XboxTitleId =>
            format!("{}lookup?alternateId=XboxTitleID&Value={id}&{trail}&fieldsTemplate=Details", base),

        IdentifierType::PackageFamilyName =>
            format!("{}lookup?alternateId=PackageFamilyName&Value={id}&{trail}&fieldsTemplate=Details", base),

        IdentifierType::ContentId =>
            format!("{}lookup?alternateId=CONTENTID&Value={id}&{trail}&fieldsTemplate=Details", base),

        IdentifierType::LegacyWindowsPhoneProductId =>
            format!("{}lookup?alternateId=LegacyWindowsPhoneProductID&Value={id}&{trail}&fieldsTemplate=Details", base),

        IdentifierType::LegacyWindowsStoreProductId =>
            format!("{}lookup?alternateId=LegacyWindowsStoreProductID&Value={id}&{trail}&fieldsTemplate=Details", base),

        IdentifierType::LegacyXboxProductId =>
            format!("{}lookup?alternateId=LegacyXboxProductID&Value={id}&{trail}&fieldsTemplate=Details", base),
    }
}
