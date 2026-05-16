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

/// Build a DisplayCatalog batch URL that fetches multiple products in a
/// single request via the `bigIds=A,B,C` parameter. Returns a URL like
/// `…/products?bigIds=9WZDN…,9NBLG…&market=US&languages=en&catalogsource=apps&fieldsTemplate=Details`.
///
/// `ids` should contain Microsoft Store Product IDs only (the `bigIds`
/// parameter does not accept alternate identifiers).
pub fn create_dcat_batch_uri(
    endpoint: &DCatEndpoint,
    ids: &[&str],
    locale: &Locale,
) -> String {
    // base ends with "/products/"; drop the slash so `?bigIds=...` attaches cleanly.
    let base = endpoint_to_base_url(endpoint).trim_end_matches('/');
    let trail = locale.dcat_trail();
    let joined = ids.join(",");
    format!("{base}?bigIds={joined}&{trail}&fieldsTemplate=Details")
}

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::enums::{DCatEndpoint, IdentifierType};
    use crate::models::locale::{Lang, Locale, Market};

    // -- string_to_package_type -----------------------------------------------

    #[test]
    fn package_type_uap() {
        assert!(matches!(string_to_package_type("UAP"), PackageType::Uap));
    }

    #[test]
    fn package_type_xap() {
        assert!(matches!(string_to_package_type("XAP"), PackageType::Xap));
    }

    #[test]
    fn package_type_appx() {
        assert!(matches!(string_to_package_type("AppX"), PackageType::AppX));
    }

    #[test]
    fn package_type_unknown() {
        assert!(matches!(string_to_package_type(""), PackageType::Unknown));
        assert!(matches!(
            string_to_package_type("msix"),
            PackageType::Unknown
        ));
    }

    // -- endpoint_to_base_url -------------------------------------------------

    #[test]
    fn base_url_production() {
        assert_eq!(
            endpoint_to_base_url(&DCatEndpoint::Production),
            "https://displaycatalog.mp.microsoft.com/v7.0/products/"
        );
    }

    #[test]
    fn base_url_xbox() {
        assert_eq!(
            endpoint_to_base_url(&DCatEndpoint::Xbox),
            "https://xbox-displaycatalog.mp.microsoft.com/v7.0/products/"
        );
    }

    #[test]
    fn all_endpoints_return_non_empty_url() {
        let endpoints = [
            DCatEndpoint::Production,
            DCatEndpoint::Int,
            DCatEndpoint::Xbox,
            DCatEndpoint::XboxInt,
            DCatEndpoint::Dev,
            DCatEndpoint::OneP,
            DCatEndpoint::OnePInt,
        ];
        for ep in &endpoints {
            let url = endpoint_to_base_url(ep);
            assert!(url.starts_with("https://"), "bad URL for {ep:?}: {url}");
        }
    }

    // -- create_dcat_uri ------------------------------------------------------

    fn prod_locale() -> Locale {
        Locale::production()
    }

    #[test]
    fn uri_product_id() {
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "9WZDNCRFJ3TJ",
            &IdentifierType::ProductId,
            &prod_locale(),
        );
        assert!(
            uri.starts_with("https://displaycatalog.mp.microsoft.com/v7.0/products/9WZDNCRFJ3TJ?")
        );
        assert!(uri.contains("market=US"));
        assert!(uri.contains("fieldsTemplate=Details"));
        assert!(!uri.contains("lookup"));
    }

    #[test]
    fn uri_package_family_name() {
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "4DF9E0F8.Netflix_mcm4njqhnhss8",
            &IdentifierType::PackageFamilyName,
            &prod_locale(),
        );
        assert!(uri.contains("lookup?alternateId=PackageFamilyName"));
        assert!(uri.contains("Value=4DF9E0F8.Netflix_mcm4njqhnhss8"));
        assert!(uri.contains("fieldsTemplate=Details"));
    }

    #[test]
    fn uri_xbox_title_id() {
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "123456",
            &IdentifierType::XboxTitleId,
            &prod_locale(),
        );
        assert!(uri.contains("alternateId=XboxTitleID"));
        assert!(uri.contains("Value=123456"));
    }

    #[test]
    fn uri_content_id() {
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "some-content-id",
            &IdentifierType::ContentId,
            &prod_locale(),
        );
        assert!(uri.contains("alternateId=CONTENTID"));
    }

    #[test]
    fn uri_legacy_phone() {
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "old-phone-id",
            &IdentifierType::LegacyWindowsPhoneProductId,
            &prod_locale(),
        );
        assert!(uri.contains("alternateId=LegacyWindowsPhoneProductID"));
    }

    // -- create_dcat_batch_uri ------------------------------------------------

    #[test]
    fn batch_uri_joins_ids_with_commas() {
        let uri = create_dcat_batch_uri(
            &DCatEndpoint::Production,
            &["9WZDNCRFJ3TJ", "9NBLGGH4R315"],
            &prod_locale(),
        );
        assert!(
            uri.starts_with("https://displaycatalog.mp.microsoft.com/v7.0/products?"),
            "got: {uri}",
        );
        assert!(uri.contains("bigIds=9WZDNCRFJ3TJ,9NBLGGH4R315"), "got: {uri}");
        assert!(uri.contains("market=US"));
        assert!(uri.contains("fieldsTemplate=Details"));
        // No trailing slash before the query string.
        assert!(!uri.contains("products/?"), "got: {uri}");
    }

    #[test]
    fn batch_uri_single_id_has_no_comma() {
        let uri = create_dcat_batch_uri(
            &DCatEndpoint::Production,
            &["9WZDNCRFJ3TJ"],
            &prod_locale(),
        );
        assert!(uri.contains("bigIds=9WZDNCRFJ3TJ&"));
        assert!(!uri.contains(",&"));
    }

    #[test]
    fn batch_uri_carries_locale_overrides() {
        let locale = Locale::new(Market::De, Lang::De, false);
        let uri = create_dcat_batch_uri(
            &DCatEndpoint::Xbox,
            &["A", "B", "C"],
            &locale,
        );
        assert!(uri.starts_with("https://xbox-displaycatalog.mp.microsoft.com/v7.0/products?"));
        assert!(uri.contains("bigIds=A,B,C"));
        assert!(uri.contains("market=DE"));
        assert!(uri.contains("languages=de"));
    }

    #[test]
    fn uri_locale_trail_embedded() {
        let locale = Locale::new(Market::De, Lang::De, false);
        let uri = create_dcat_uri(
            &DCatEndpoint::Production,
            "9WZDNCRFJ3TJ",
            &IdentifierType::ProductId,
            &locale,
        );
        assert!(uri.contains("market=DE"));
        assert!(uri.contains("languages=de"));
    }
}
