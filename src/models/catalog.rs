use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a field that the MS Store API returns as either an integer or
/// a quoted string (e.g. `0` and `"0"` both appear in the wild).
fn de_str_or_i64<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrInt {
        Int(i64),
        Str(String),
    }

    Ok(match Option::<StrOrInt>::deserialize(d)? {
        Some(StrOrInt::Int(n)) => Some(n),
        Some(StrOrInt::Str(s)) => s.parse::<i64>().ok(),
        None => None,
    })
}

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct DisplayCatalogModel {
    pub product: Option<Product>,
    pub big_ids: Option<Vec<String>>,
    pub has_more_pages: Option<bool>,
    pub products: Option<Vec<Product>>,
    pub total_result_count: Option<i64>,
}

// ---------------------------------------------------------------------------
// Product
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Product {
    pub last_modified_date: Option<String>,
    pub localized_properties: Option<Vec<ProductLocalizedProperty>>,
    /// Flat title field returned by the `/productFamilies/autosuggest`
    /// endpoint (the per-product `/products/{id}` endpoint puts titles under
    /// `LocalizedProperties[].ProductTitle` instead — see
    /// [`Self::localized_properties`]).
    #[serde(default, rename(serialize = "title", deserialize = "Title"))]
    pub title: Option<String>,
    /// Flat icon URL returned by autosuggest (protocol-relative, e.g.
    /// `//store-images.s-microsoft.com/...`).
    #[serde(default, rename(serialize = "icon", deserialize = "Icon"))]
    pub icon: Option<String>,
    /// Product type string from autosuggest (e.g. `"Application"`,
    /// `"Game"`). For the typed enum see [`ProductKind`].
    #[serde(default, rename(serialize = "type", deserialize = "Type"))]
    pub r#type: Option<String>,
    pub market_properties: Option<Vec<ProductMarketProperty>>,
    #[serde(rename(serialize = "productASchema", deserialize = "ProductASchema"))]
    pub product_a_schema: Option<String>,
    #[serde(rename(serialize = "productBSchema", deserialize = "ProductBSchema"))]
    pub product_b_schema: Option<String>,
    pub properties: Option<ProductProperties>,
    pub alternate_ids: Option<Vec<AlternateId>>,
    pub domain_data_version: Option<serde_json::Value>,
    pub ingestion_source: Option<String>,
    pub is_microsoft_product: Option<bool>,
    pub preferred_sku_id: Option<String>,
    pub product_type: Option<String>,
    pub validation_data: Option<ValidationData>,
    #[serde(rename(serialize = "sandboxId", deserialize = "SandboxId"))]
    pub sandbox_id: Option<String>,
    pub is_sandboxed_product: Option<bool>,
    pub merchandizing_tags: Option<Vec<serde_json::Value>>,
    pub part_d: Option<String>,
    pub product_family: Option<String>,
    pub schema_version: Option<String>,
    pub product_kind: Option<String>,
    pub display_sku_availabilities: Option<Vec<DisplaySkuAvailability>>,
    /// Compliance / content-policy escape hatch. Usually `{}`; schema not
    /// documented by Microsoft, so kept as `Value`.
    pub product_policies: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Product sub-types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ValidationData {
    pub passed_validation: Option<bool>,
    pub revision_id: Option<String>,
    pub validation_result_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ProductProperties {
    pub attributes: Option<Vec<serde_json::Value>>,
    pub can_install_to_sd_card: Option<bool>,
    pub category: Option<String>,
    pub sub_category: Option<String>,
    pub categories: Option<serde_json::Value>,
    pub extensions: Option<serde_json::Value>,
    pub is_accessible: Option<bool>,
    pub is_line_of_business_app: Option<bool>,
    pub is_published_to_legacy_windows_phone_store: Option<bool>,
    pub is_published_to_legacy_windows_store: Option<bool>,
    pub is_settings_app: Option<bool>,
    pub package_family_name: Option<String>,
    pub package_identity_name: Option<String>,
    pub publisher_certificate_name: Option<String>,
    pub publisher_id: Option<String>,
    pub xbox_live_tier: Option<serde_json::Value>,
    #[serde(rename(serialize = "xboxXPA", deserialize = "XboxXPA"))]
    pub xbox_xpa: Option<serde_json::Value>,
    /// Misc Xbox flags. The wire surface uses ALL-CAPS for the key, so
    /// the rename is explicit. Object shape varies and is usually empty
    /// (`{}`) for non-Xbox products — kept as `Value` for forward-compat.
    #[serde(rename(serialize = "xbox", deserialize = "XBOX"))]
    pub xbox: Option<serde_json::Value>,
    pub xbox_console_gen_compatible: Option<serde_json::Value>,
    pub xbox_console_gen_optimized: Option<serde_json::Value>,
    pub xbox_cross_gen_set_id: Option<serde_json::Value>,
    pub xbox_live_gold_required: Option<bool>,
    /// Escape hatch carrying additional client metadata. Most useful field
    /// in practice is `StoreApp` (a string-encoded JSON object with a
    /// `productDeclarations.usesGenerativeAI` flag and secondary
    /// categories). Kept as `Value` because the inner JSON is opaque.
    pub extended_client_metadata: Option<serde_json::Value>,
    pub ownership_type: Option<serde_json::Value>,
    pub pdp_background_color: Option<String>,
    pub has_add_ons: Option<bool>,
    pub revision_id: Option<String>,
    pub product_group_id: Option<String>,
    pub product_group_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ProductMarketProperty {
    pub original_release_date: Option<String>,
    pub original_release_date_friendly_name: Option<String>,
    pub minimum_user_age: Option<i64>,
    pub content_ratings: Option<Vec<ContentRating>>,
    pub related_products: Option<Vec<serde_json::Value>>,
    pub usage_data: Option<Vec<UsageDatum>>,
    pub bundle_config: Option<serde_json::Value>,
    pub markets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct UsageDatum {
    pub average_rating: Option<f64>,
    pub aggregate_time_span: Option<String>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub rating_count: Option<i64>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub purchase_count: Option<i64>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub trial_count: Option<i64>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub rental_count: Option<i64>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub play_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ContentRating {
    pub rating_system: Option<String>,
    pub rating_id: Option<String>,
    pub rating_descriptors: Option<Vec<String>>,
    pub rating_disclaimers: Option<Vec<serde_json::Value>>,
    pub interactive_elements: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ProductLocalizedProperty {
    pub developer_name: Option<String>,
    pub display_platform_properties: Option<serde_json::Value>,
    pub publisher_name: Option<String>,
    pub publisher_website_uri: Option<String>,
    pub support_uri: Option<String>,
    pub support_phone: Option<String>,
    pub publisher_address: Option<String>,
    pub eligibility_properties: Option<serde_json::Value>,
    pub franchises: Option<Vec<serde_json::Value>>,
    pub images: Option<Vec<Image>>,
    pub videos: Option<Vec<serde_json::Value>>,
    /// CMS-managed promo videos (hero trailers, etc.) — distinct from the
    /// generic `videos` array. Carries DASH/HLS URLs + a typed
    /// `PreviewImage`.
    #[serde(rename(serialize = "cmsVideos", deserialize = "CMSVideos"))]
    pub cms_videos: Option<Vec<CmsVideo>>,
    pub product_description: Option<String>,
    pub product_title: Option<String>,
    pub friendly_title: Option<String>,
    pub short_title: Option<String>,
    pub sort_title: Option<String>,
    pub short_description: Option<String>,
    pub search_titles: Option<Vec<SearchTitle>>,
    pub voice_title: Option<String>,
    pub render_group_details: Option<serde_json::Value>,
    pub product_display_ranks: Option<Vec<serde_json::Value>>,
    /// True if the listing has a 3D-interactive model viewer
    /// (used by some AR-enabled apps).
    #[serde(rename(
        serialize = "interactive3DEnabled",
        deserialize = "Interactive3DEnabled"
    ))]
    pub interactive_3d_enabled: Option<bool>,
    /// Opaque config for the 3D model viewer (Microsoft hasn't published
    /// a schema). Keep as `Value` for forward-compat.
    pub interactive_model_config: Option<serde_json::Value>,
    pub language: Option<String>,
    pub markets: Option<Vec<String>>,
}

/// A CMS-managed promo video attached to a product (typically the
/// "hero trailer" shown on the Store listing). Most string fields can
/// be empty strings in the wild.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct CmsVideo {
    pub audio_encoding: Option<String>,
    #[serde(rename(serialize = "cc", deserialize = "CC"))]
    pub cc: Option<serde_json::Value>,
    #[serde(rename(serialize = "cms", deserialize = "CMS"))]
    pub cms: Option<serde_json::Value>,
    pub caption: Option<String>,
    /// MPEG-DASH manifest URL.
    #[serde(rename(serialize = "dash", deserialize = "DASH"))]
    pub dash: Option<String>,
    /// HLS manifest URL.
    #[serde(rename(serialize = "hls", deserialize = "HLS"))]
    pub hls: Option<String>,
    pub file_size_in_bytes: Option<i64>,
    pub height: Option<i64>,
    pub width: Option<i64>,
    pub preview_image: Option<Image>,
    pub sort_order: Option<i64>,
    pub trailer_id: Option<serde_json::Value>,
    pub video_encoding: Option<String>,
    pub video_position_info: Option<String>,
    /// e.g. `"HeroTrailer"`, `"Trailer"`.
    pub video_purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SearchTitle {
    pub search_title_string: Option<String>,
    pub search_title_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Image {
    pub background_color: Option<String>,
    pub caption: Option<String>,
    pub file_size_in_bytes: Option<i64>,
    pub foreground_color: Option<String>,
    pub height: Option<i64>,
    pub image_position_info: Option<String>,
    pub image_purpose: Option<String>,
    /// Explicit rename — serde's PascalCase converter renders `SHA256` as
    /// `Sha256`, which doesn't match the wire (`UnscaledImageSHA256Hash`).
    #[serde(rename(
        serialize = "unscaledImageSHA256Hash",
        deserialize = "UnscaledImageSHA256Hash"
    ))]
    pub unscaled_image_sha256_hash: Option<String>,
    pub uri: Option<String>,
    pub width: Option<i64>,
    /// EIS (Enterprise/Internal Store) listing identifier — explicit
    /// rename to preserve the `EIS` casing across the wire.
    #[serde(
        default,
        rename(
            serialize = "eisListingIdentifier",
            deserialize = "EISListingIdentifier"
        )
    )]
    pub eis_listing_identifier: Option<String>,
    /// Microsoft asset / CDN file id (numeric string), e.g.
    /// `"3067298299926220602"`.
    #[serde(default)]
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct AlternateId {
    pub id_type: Option<String>,
    pub value: Option<String>,
}

// ---------------------------------------------------------------------------
// Sku / DisplaySkuAvailability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct DisplaySkuAvailability {
    pub sku: Option<Sku>,
    pub availabilities: Option<Vec<Availability>>,
    /// Parallel array to `availabilities` describing prior price /
    /// availability snapshots (the "historical best" the Store shows for
    /// crossed-out pricing). Same shape as `Availability` — the
    /// HBA-only extra field is `product_a_schema`; the
    /// `Availability`-only extra is `remediation_required`. Both are
    /// optional, so one struct serves both arrays.
    pub historical_best_availabilities: Option<Vec<Availability>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Sku {
    pub last_modified_date: Option<String>,
    pub localized_properties: Option<Vec<SkuLocalizedProperty>>,
    pub market_properties: Option<Vec<SkuMarketProperty>>,
    pub properties: Option<SkuProperties>,
    #[serde(rename(serialize = "skuASchema", deserialize = "SkuASchema"))]
    pub sku_a_schema: Option<String>,
    #[serde(rename(serialize = "skuBSchema", deserialize = "SkuBSchema"))]
    pub sku_b_schema: Option<String>,
    pub sku_id: Option<String>,
    pub sku_type: Option<String>,
    pub recurrence_policy: Option<serde_json::Value>,
    pub subscription_policy_id: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SkuProperties {
    pub early_adopter_enrollment_url: Option<serde_json::Value>,
    pub fulfillment_data: Option<FulfillmentData>,
    pub fulfillment_type: Option<String>,
    pub fulfillment_plugin_id: Option<String>,
    /// Explicit rename — `IAPs` is all-caps on the wire and serde's
    /// PascalCase converter would emit `IaPs` otherwise.
    #[serde(rename(serialize = "hasThirdPartyIAPs", deserialize = "HasThirdPartyIAPs"))]
    pub has_third_party_ia_ps: Option<bool>,
    pub last_update_date: Option<String>,
    pub hardware_properties: Option<HardwareProperties>,
    pub hardware_requirements: Option<Vec<serde_json::Value>>,
    pub hardware_warning_list: Option<Vec<serde_json::Value>>,
    pub installation_terms: Option<String>,
    pub packages: Option<Vec<Package>>,
    pub version_string: Option<String>,
    /// Explicit rename — `B2B` is all-caps on the wire.
    #[serde(rename(
        serialize = "visibleToB2BServiceIds",
        deserialize = "VisibleToB2BServiceIds"
    ))]
    pub visible_to_b2b_service_ids: Option<Vec<serde_json::Value>>,
    #[serde(rename(serialize = "xboxXPA", deserialize = "XboxXPA"))]
    pub xbox_xpa: Option<bool>,
    pub bundled_skus: Option<Vec<serde_json::Value>>,
    pub is_repurchasable: Option<bool>,
    pub sku_display_rank: Option<i64>,
    pub display_physical_store_inventory: Option<serde_json::Value>,
    pub additional_identifiers: Option<Vec<serde_json::Value>>,
    pub is_trial: Option<bool>,
    pub is_pre_order: Option<bool>,
    pub is_bundle: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct FulfillmentData {
    pub product_id: Option<String>,
    #[serde(rename(serialize = "wuBundleId", deserialize = "WuBundleId"))]
    pub wu_bundle_id: Option<String>,
    #[serde(rename(serialize = "wuCategoryId", deserialize = "WuCategoryId"))]
    pub wu_category_id: Option<String>,
    pub package_family_name: Option<String>,
    pub sku_id: Option<String>,
    pub content: Option<serde_json::Value>,
    /// Per-package feature flags. Often `null`; schema not documented.
    pub package_features: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct HardwareProperties {
    pub minimum_hardware: Option<Vec<serde_json::Value>>,
    pub recommended_hardware: Option<Vec<String>>,
    pub minimum_processor: Option<String>,
    pub recommended_processor: Option<String>,
    pub minimum_graphics: Option<String>,
    pub recommended_graphics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Package {
    pub applications: Option<Vec<Application>>,
    pub architectures: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
    pub device_capabilities: Option<Vec<serde_json::Value>>,
    pub experience_ids: Option<Vec<serde_json::Value>>,
    pub framework_dependencies: Option<Vec<FrameworkDependency>>,
    pub hardware_dependencies: Option<Vec<serde_json::Value>>,
    pub hardware_requirements: Option<Vec<serde_json::Value>>,
    pub hash: Option<String>,
    pub hash_algorithm: Option<String>,
    pub is_streaming_app: Option<bool>,
    pub languages: Option<Vec<String>>,
    pub max_download_size_in_bytes: Option<i64>,
    #[serde(default, deserialize_with = "de_str_or_i64")]
    pub max_install_size_in_bytes: Option<i64>,
    pub package_format: Option<String>,
    pub package_family_name: Option<String>,
    pub main_package_family_name_for_dlc: Option<serde_json::Value>,
    pub package_full_name: Option<String>,
    pub package_id: Option<String>,
    pub content_id: Option<String>,
    pub key_id: Option<String>,
    pub package_rank: Option<i64>,
    pub package_uri: Option<String>,
    pub platform_dependencies: Option<Vec<PlatformDependency>>,
    pub platform_dependency_xml_blob: Option<String>,
    pub resource_id: Option<String>,
    pub version: Option<String>,
    pub package_download_uris: Option<Vec<PackageDownloadUri>>,
    pub driver_dependencies: Option<Vec<serde_json::Value>>,
    pub fulfillment_data: Option<FulfillmentData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct PackageDownloadUri {
    pub uri: Option<String>,
    pub rank: Option<i64>,
}

/// One entry in `Package.framework_dependencies` — a runtime / library
/// package the primary binary needs. Versions are returned as integers
/// or strings depending on the endpoint, so we keep them as
/// [`serde_json::Value`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct FrameworkDependency {
    pub max_tested: Option<serde_json::Value>,
    pub min_version: Option<serde_json::Value>,
    /// PFN base of the dependency, e.g. `"Microsoft.VCLibs.140.00.UWPDesktop"`.
    pub package_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct PlatformDependency {
    // The API returns these as either integers or quoted strings (e.g. "0").
    pub max_tested: Option<serde_json::Value>,
    pub min_version: Option<serde_json::Value>,
    pub platform_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Application {
    pub application_id: Option<String>,
    pub declaration_order: Option<i64>,
    pub extensions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SkuMarketProperty {
    pub first_available_date: Option<String>,
    pub supported_languages: Option<Vec<String>>,
    pub package_ids: Option<serde_json::Value>,
    pub markets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SkuLocalizedProperty {
    pub contributors: Option<Vec<serde_json::Value>>,
    pub features: Option<Vec<serde_json::Value>>,
    pub minimum_notes: Option<String>,
    pub recommended_notes: Option<String>,
    pub release_notes: Option<String>,
    pub display_platform_properties: Option<serde_json::Value>,
    pub sku_description: Option<String>,
    pub sku_title: Option<String>,
    pub sku_button_title: Option<String>,
    pub delivery_date_overlay: Option<serde_json::Value>,
    pub sku_display_rank: Option<Vec<serde_json::Value>>,
    pub text_resources: Option<serde_json::Value>,
    pub images: Option<Vec<serde_json::Value>>,
    pub legal_text: Option<LegalText>,
    pub language: Option<String>,
    pub markets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct LegalText {
    pub additional_license_terms: Option<String>,
    pub copyright: Option<String>,
    pub copyright_uri: Option<String>,
    pub privacy_policy: Option<String>,
    pub privacy_policy_uri: Option<String>,
    pub tou: Option<String>,
    pub tou_uri: Option<String>,
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Availability {
    pub actions: Option<Vec<String>>,
    #[serde(rename(serialize = "availabilityASchema", deserialize = "AvailabilityASchema"))]
    pub availability_a_schema: Option<String>,
    #[serde(rename(serialize = "availabilityBSchema", deserialize = "AvailabilityBSchema"))]
    pub availability_b_schema: Option<String>,
    pub availability_id: Option<String>,
    pub conditions: Option<Conditions>,
    pub last_modified_date: Option<String>,
    pub markets: Option<Vec<String>>,
    pub order_management_data: Option<OrderManagementData>,
    pub properties: Option<AvailabilityProperties>,
    pub sku_id: Option<String>,
    pub display_rank: Option<i64>,
    pub remediation_required: Option<bool>,
    /// Subscription / entitlement-key data. Present (non-null) for paid
    /// content; `None` for free apps like Netflix.
    pub licensing_data: Option<LicensingData>,
    /// Schema version string — present only on entries inside
    /// `historical_best_availabilities` (e.g. `"Product;3"`). `None` on
    /// regular `availabilities`.
    #[serde(rename(serialize = "productASchema", deserialize = "ProductASchema"))]
    pub product_a_schema: Option<String>,
}

/// Subscription / entitlement-key data attached to an [`Availability`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct LicensingData {
    pub satisfying_entitlement_keys: Option<Vec<SatisfyingEntitlementKey>>,
}

/// One entry in [`LicensingData::satisfying_entitlement_keys`] — the keys
/// that satisfy this availability's licensing requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct SatisfyingEntitlementKey {
    pub entitlement_keys: Option<Vec<String>>,
    pub licensing_key_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct AvailabilityProperties {
    pub original_release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct OrderManagementData {
    pub granted_entitlement_keys: Option<Vec<serde_json::Value>>,
    #[serde(rename(serialize = "piFilter", deserialize = "PIFilter"))]
    pub pi_filter: Option<PiFilter>,
    pub price: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Price {
    pub currency_code: Option<String>,
    #[serde(rename(serialize = "isPIRequired", deserialize = "IsPIRequired"))]
    pub is_pi_required: Option<bool>,
    pub list_price: Option<f64>,
    #[serde(rename(serialize = "msrp", deserialize = "MSRP"))]
    pub msrp: Option<f64>,
    pub tax_type: Option<String>,
    pub wholesale_currency_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct PiFilter {
    pub exclusion_properties: Option<Vec<serde_json::Value>>,
    pub inclusion_properties: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct Conditions {
    pub client_conditions: Option<ClientConditions>,
    pub end_date: Option<String>,
    pub resource_set_ids: Option<Vec<String>>,
    pub start_date: Option<String>,
    /// Geographic / regulatory eligibility gates, e.g.
    /// `["CannotSeenByChinaClient"]`. Present on historical-best entries.
    pub eligibility_predicate_ids: Option<Vec<String>>,
    /// DCat schema version this availability targets (e.g. `6`). Present
    /// on historical-best entries.
    pub supported_catalog_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct ClientConditions {
    pub allowed_platforms: Option<Vec<AllowedPlatform>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
pub struct AllowedPlatform {
    // The API returns these as either integers or quoted strings (e.g. "0").
    pub max_version: Option<serde_json::Value>,
    pub min_version: Option<serde_json::Value>,
    pub platform_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- de_str_or_i64 via UsageDatum -----------------------------------------

    #[test]
    fn usage_datum_integer_counts() {
        let json = r#"{"AverageRating":4.5,"AggregateTimeSpan":"AllTime","RatingCount":100,"PurchaseCount":50,"TrialCount":10,"RentalCount":0,"PlayCount":5}"#;
        let d: UsageDatum = serde_json::from_str(json).unwrap();
        assert_eq!(d.rating_count, Some(100));
        assert_eq!(d.purchase_count, Some(50));
        assert_eq!(d.trial_count, Some(10));
        assert_eq!(d.rental_count, Some(0));
        assert_eq!(d.play_count, Some(5));
    }

    #[test]
    fn usage_datum_string_counts() {
        // MS Store returns some counts as quoted strings.
        let json = r#"{"RatingCount":"42","PurchaseCount":"0","TrialCount":"7","RentalCount":"0","PlayCount":"99"}"#;
        let d: UsageDatum = serde_json::from_str(json).unwrap();
        assert_eq!(d.rating_count, Some(42));
        assert_eq!(d.purchase_count, Some(0));
        assert_eq!(d.trial_count, Some(7));
        assert_eq!(d.rental_count, Some(0));
        assert_eq!(d.play_count, Some(99));
    }

    #[test]
    fn usage_datum_null_counts() {
        let json = r#"{"RatingCount":null,"PurchaseCount":null}"#;
        let d: UsageDatum = serde_json::from_str(json).unwrap();
        assert_eq!(d.rating_count, None);
        assert_eq!(d.purchase_count, None);
    }

    #[test]
    fn usage_datum_missing_counts_default_to_none() {
        let json = r#"{}"#;
        let d: UsageDatum = serde_json::from_str(json).unwrap();
        assert_eq!(d.rating_count, None);
        assert_eq!(d.purchase_count, None);
    }

    // -- DisplayCatalogModel round-trip ---------------------------------------

    #[test]
    fn product_serializes_as_camel_case_while_deserializing_pascal_case() {
        // PascalCase in (matches what MS Store sends), camelCase out (what JS expects).
        let json = r#"{"Product":{"LocalizedProperties":[{"ProductTitle":"Hi"}],"ProductASchema":"v1","SandboxId":"sb"},"Products":[]}"#;
        let m: DisplayCatalogModel = serde_json::from_str(json).unwrap();
        let out = serde_json::to_string(&m).unwrap();
        assert!(out.contains("\"product\":"), "got: {out}");
        assert!(out.contains("\"products\":"), "got: {out}");
        assert!(out.contains("\"localizedProperties\":"), "got: {out}");
        assert!(out.contains("\"productTitle\":"), "got: {out}");
        assert!(out.contains("\"productASchema\":"), "got: {out}");
        assert!(out.contains("\"sandboxId\":"), "got: {out}");
        assert!(!out.contains("\"Product\""), "found PascalCase in: {out}");
    }

    #[test]
    fn price_acronyms_emit_camel_case() {
        let json = r#"{"CurrencyCode":"USD","IsPIRequired":true,"MSRP":9.99,"ListPrice":4.99}"#;
        let p: Price = serde_json::from_str(json).unwrap();
        let out = serde_json::to_string(&p).unwrap();
        assert!(out.contains("\"isPIRequired\":true"), "got: {out}");
        assert!(out.contains("\"msrp\":9.99"), "got: {out}");
        assert!(out.contains("\"currencyCode\":"), "got: {out}");
        assert!(out.contains("\"listPrice\":"), "got: {out}");
    }

    #[test]
    fn display_catalog_model_deserializes_minimal_json() {
        let json = r#"{"Product":null,"BigIds":null,"HasMorePages":false,"Products":[],"TotalResultCount":0}"#;
        let m: DisplayCatalogModel = serde_json::from_str(json).unwrap();
        assert!(!m.has_more_pages.unwrap_or(true));
        assert_eq!(m.total_result_count, Some(0));
    }

    #[test]
    fn fulfillment_data_round_trip_wu_keys() {
        // The MS Store wire form uses WuCategoryId / WuBundleId; JS sees camelCase.
        let json = r#"{"ProductId":"9NBLGGH4R315","WuBundleId":"abc-bundle","WuCategoryId":"def-cat","PackageFamilyName":"X.Y_1234"}"#;
        let fd: FulfillmentData = serde_json::from_str(json).unwrap();
        assert_eq!(fd.wu_category_id.as_deref(), Some("def-cat"));
        assert_eq!(fd.wu_bundle_id.as_deref(), Some("abc-bundle"));

        let out = serde_json::to_string(&fd).unwrap();
        assert!(out.contains("\"wuCategoryId\":\"def-cat\""), "got: {out}");
        assert!(out.contains("\"wuBundleId\":\"abc-bundle\""), "got: {out}");
        assert!(out.contains("\"productId\":\"9NBLGGH4R315\""), "got: {out}");
        assert!(
            out.contains("\"packageFamilyName\":\"X.Y_1234\""),
            "got: {out}"
        );
        // Old PascalCase keys must not leak through.
        assert!(!out.contains("\"WuCategoryId\""), "got: {out}");
    }

    #[test]
    fn sku_round_trip_a_b_schemas() {
        let json = r#"{"SkuASchema":"A1","SkuBSchema":"B1","SkuId":"0001"}"#;
        let sku: Sku = serde_json::from_str(json).unwrap();
        assert_eq!(sku.sku_a_schema.as_deref(), Some("A1"));
        assert_eq!(sku.sku_b_schema.as_deref(), Some("B1"));

        let out = serde_json::to_string(&sku).unwrap();
        assert!(out.contains("\"skuASchema\":\"A1\""), "got: {out}");
        assert!(out.contains("\"skuBSchema\":\"B1\""), "got: {out}");
        assert!(out.contains("\"skuId\":\"0001\""), "got: {out}");
    }

    #[test]
    fn package_max_download_size_round_trip() {
        // Package.max_download_size_in_bytes powers PackageInstance.packageSize
        // when FE3 doesn't report a size — important enough to lock in.
        let json = r#"{"PackageFullName":"X.Y_1.0.0.0_x64__abcd","MaxDownloadSizeInBytes":123456789,"MaxInstallSizeInBytes":"234567890"}"#;
        let pkg: Package = serde_json::from_str(json).unwrap();
        assert_eq!(
            pkg.package_full_name.as_deref(),
            Some("X.Y_1.0.0.0_x64__abcd")
        );
        assert_eq!(pkg.max_download_size_in_bytes, Some(123_456_789));
        // de_str_or_i64 converts "234567890" → 234567890
        assert_eq!(pkg.max_install_size_in_bytes, Some(234_567_890));

        let out = serde_json::to_string(&pkg).unwrap();
        assert!(
            out.contains("\"maxDownloadSizeInBytes\":123456789"),
            "got: {out}",
        );
        assert!(out.contains("\"packageFullName\":"), "got: {out}");
    }

    #[test]
    fn order_management_data_pi_filter_round_trip() {
        let json = r#"{"PIFilter":{"InclusionProperties":[],"ExclusionProperties":[]}}"#;
        let omd: OrderManagementData = serde_json::from_str(json).unwrap();
        assert!(omd.pi_filter.is_some());

        let out = serde_json::to_string(&omd).unwrap();
        assert!(out.contains("\"piFilter\":"), "got: {out}");
        assert!(!out.contains("\"PIFilter\""), "got: {out}");
    }

    #[test]
    fn product_with_nested_sku_serializes_consistent_case() {
        // End-to-end: PascalCase in, camelCase out, multiple nesting levels.
        let json = r#"{
            "LastModifiedDate":"2024-01-01",
            "ProductASchema":"sa-1",
            "DisplaySkuAvailabilities":[{
                "Sku":{
                    "SkuId":"0001",
                    "Properties":{
                        "FulfillmentData":{"WuCategoryId":"cat-123"},
                        "Packages":[{"PackageFullName":"X.Y","MaxDownloadSizeInBytes":42}]
                    }
                }
            }]
        }"#;
        let p: Product = serde_json::from_str(json).unwrap();
        let dsa = p.display_sku_availabilities.as_deref().unwrap();
        let sku = dsa[0].sku.as_ref().unwrap();
        let props = sku.properties.as_ref().unwrap();
        assert_eq!(
            props
                .fulfillment_data
                .as_ref()
                .unwrap()
                .wu_category_id
                .as_deref(),
            Some("cat-123"),
        );

        let out = serde_json::to_string(&p).unwrap();
        // Every nested level must be camelCase.
        assert!(out.contains("\"displaySkuAvailabilities\":"), "got: {out}");
        assert!(out.contains("\"sku\":"), "got: {out}");
        assert!(out.contains("\"properties\":"), "got: {out}");
        assert!(out.contains("\"fulfillmentData\":"), "got: {out}");
        assert!(out.contains("\"wuCategoryId\":\"cat-123\""), "got: {out}");
        assert!(out.contains("\"productASchema\":"), "got: {out}");
        assert!(out.contains("\"maxDownloadSizeInBytes\":42"), "got: {out}");
        // No leftover PascalCase keys at any level.
        for bad in [
            "\"DisplaySkuAvailabilities\"",
            "\"Sku\"",
            "\"WuCategoryId\"",
            "\"ProductASchema\"",
        ] {
            assert!(!out.contains(bad), "leaked {bad} in: {out}");
        }
    }
}
