use serde::{Deserialize, Serialize};

/// Which DisplayCatalog endpoint to query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DCatEndpoint {
    Production,
    Int,
    Xbox,
    XboxInt,
    Dev,
    OneP,
    OnePInt,
}

/// Package format type returned by the FE3 service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageType {
    Uap,
    Xap,
    AppX,
    #[default]
    Unknown,
}

/// How to interpret the product identifier passed to a DCat query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IdentifierType {
    ProductId,
    XboxTitleId,
    PackageFamilyName,
    ContentId,
    LegacyWindowsPhoneProductId,
    LegacyWindowsStoreProductId,
    LegacyXboxProductId,
}

impl IdentifierType {
    /// Canonical camelCase wire form (matches the serde representation).
    pub fn as_str(&self) -> &'static str {
        match self {
            IdentifierType::ProductId => "productId",
            IdentifierType::XboxTitleId => "xboxTitleId",
            IdentifierType::PackageFamilyName => "packageFamilyName",
            IdentifierType::ContentId => "contentId",
            IdentifierType::LegacyWindowsPhoneProductId => "legacyWindowsPhoneProductId",
            IdentifierType::LegacyWindowsStoreProductId => "legacyWindowsStoreProductId",
            IdentifierType::LegacyXboxProductId => "legacyXboxProductId",
        }
    }

    /// Parse a string in any common casing — `ProductId`, `productId`,
    /// `product-id`, `PRODUCT_ID`, `product_id` — into the canonical enum.
    /// Non-alphanumeric characters are stripped before comparison.
    pub fn parse_tolerant(raw: &str) -> Option<IdentifierType> {
        let normalized: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        Some(match normalized.as_str() {
            "productid" => IdentifierType::ProductId,
            "xboxtitleid" => IdentifierType::XboxTitleId,
            "packagefamilyname" => IdentifierType::PackageFamilyName,
            "contentid" => IdentifierType::ContentId,
            "legacywindowsphoneproductid" => IdentifierType::LegacyWindowsPhoneProductId,
            "legacywindowsstoreproductid" => IdentifierType::LegacyWindowsStoreProductId,
            "legacyxboxproductid" => IdentifierType::LegacyXboxProductId,
            _ => return None,
        })
    }
}

/// Purpose / role of an image asset attached to a product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImagePurpose {
    Logo,
    Tile,
    Screenshot,
    BoxArt,
    BrandedKeyArt,
    Poster,
    FeaturePromotionalSquareArt,
    ImageGallery,
    SuperHeroArt,
    TitledHeroArt,
}

/// High-level product category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProductKind {
    Game,
    Application,
    Book,
    Movie,
    Physical,
    Software,
}

/// Target device family for search / package filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceFamily {
    Desktop,
    Mobile,
    Xbox,
    ServerCore,
    IotCore,
    HoloLens,
    Andromeda,
    Universal,
    Wcos,
}

impl DeviceFamily {
    /// Returns the `platformDependencyName` string used in search URLs.
    pub fn platform_dependency_name(&self) -> &'static str {
        match self {
            DeviceFamily::Desktop => "Windows.Desktop",
            DeviceFamily::Mobile => "Windows.Mobile",
            DeviceFamily::Xbox => "Windows.Xbox",
            DeviceFamily::ServerCore => "Windows.Server",
            DeviceFamily::IotCore => "Windows.Iot",
            DeviceFamily::HoloLens => "Windows.Holographic",
            DeviceFamily::Andromeda => "Windows.8828080",
            DeviceFamily::Universal => "Windows.Universal",
            DeviceFamily::Wcos => "Windows.Core",
        }
    }
}

/// Result status of a DisplayCatalog query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayCatalogResult {
    NotFound,
    Restricted,
    TimedOut,
    Error,
    Found,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_type_canonical_str_matches_serde() {
        // The canonical string must match what serde emits.
        for it in [
            IdentifierType::ProductId,
            IdentifierType::XboxTitleId,
            IdentifierType::PackageFamilyName,
            IdentifierType::ContentId,
            IdentifierType::LegacyWindowsPhoneProductId,
            IdentifierType::LegacyWindowsStoreProductId,
            IdentifierType::LegacyXboxProductId,
        ] {
            let json = serde_json::to_string(&it).unwrap();
            // serde emits "value" with surrounding quotes
            assert_eq!(json, format!("\"{}\"", it.as_str()));
        }
    }

    #[test]
    fn identifier_type_parse_tolerant_pascal_case() {
        assert_eq!(
            IdentifierType::parse_tolerant("ProductId"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("PackageFamilyName"),
            Some(IdentifierType::PackageFamilyName),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("LegacyWindowsPhoneProductId"),
            Some(IdentifierType::LegacyWindowsPhoneProductId),
        );
    }

    #[test]
    fn identifier_type_parse_tolerant_camel_case() {
        assert_eq!(
            IdentifierType::parse_tolerant("productId"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("xboxTitleId"),
            Some(IdentifierType::XboxTitleId),
        );
    }

    #[test]
    fn identifier_type_parse_tolerant_separators() {
        // Hyphens, underscores, and arbitrary whitespace all dropped.
        assert_eq!(
            IdentifierType::parse_tolerant("product-id"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("product_id"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("product id"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("package.family.name"),
            Some(IdentifierType::PackageFamilyName),
        );
    }

    #[test]
    fn identifier_type_parse_tolerant_screaming_case() {
        assert_eq!(
            IdentifierType::parse_tolerant("PRODUCT_ID"),
            Some(IdentifierType::ProductId),
        );
        assert_eq!(
            IdentifierType::parse_tolerant("XBOX-TITLE-ID"),
            Some(IdentifierType::XboxTitleId),
        );
    }

    #[test]
    fn identifier_type_parse_tolerant_rejects_unknown() {
        assert_eq!(IdentifierType::parse_tolerant(""), None);
        assert_eq!(IdentifierType::parse_tolerant("notARealId"), None);
        assert_eq!(IdentifierType::parse_tolerant("product"), None);
        // Surrounding noise that *doesn't* spell out the variant is still rejected.
        assert_eq!(IdentifierType::parse_tolerant("xxxProductIdxxx"), None);
    }

    #[test]
    fn identifier_type_round_trip_via_tolerant_and_serde() {
        // For every variant, the canonical str round-trips through both
        // tolerant parsing and serde deserialization.
        for it in [
            IdentifierType::ProductId,
            IdentifierType::XboxTitleId,
            IdentifierType::PackageFamilyName,
            IdentifierType::ContentId,
            IdentifierType::LegacyWindowsPhoneProductId,
            IdentifierType::LegacyWindowsStoreProductId,
            IdentifierType::LegacyXboxProductId,
        ] {
            let s = it.as_str();
            assert_eq!(IdentifierType::parse_tolerant(s), Some(it.clone()));
            let json = format!("\"{s}\"");
            let parsed: IdentifierType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, it);
        }
    }
}
