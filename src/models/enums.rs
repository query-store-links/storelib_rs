/// Which DisplayCatalog endpoint to query.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum PackageType {
    Uap,
    Xap,
    AppX,
    Unknown,
}

/// How to interpret the product identifier passed to a DCat query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierType {
    ProductId,
    XboxTitleId,
    PackageFamilyName,
    ContentId,
    LegacyWindowsPhoneProductId,
    LegacyWindowsStoreProductId,
    LegacyXboxProductId,
}

/// Purpose / role of an image asset attached to a product.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductKind {
    Game,
    Application,
    Book,
    Movie,
    Physical,
    Software,
}

/// Target device family for search / package filtering.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayCatalogResult {
    NotFound,
    Restricted,
    TimedOut,
    Error,
    Found,
}
