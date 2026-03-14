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
#[serde(rename_all = "PascalCase")]
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
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub last_modified_date: Option<String>,
    pub localized_properties: Option<Vec<ProductLocalizedProperty>>,
    pub market_properties: Option<Vec<ProductMarketProperty>>,
    #[serde(rename = "ProductASchema")]
    pub product_a_schema: Option<String>,
    #[serde(rename = "ProductBSchema")]
    pub product_b_schema: Option<String>,
    pub properties: Option<ProductProperties>,
    pub alternate_ids: Option<Vec<AlternateId>>,
    pub domain_data_version: Option<serde_json::Value>,
    pub ingestion_source: Option<String>,
    pub is_microsoft_product: Option<bool>,
    pub preferred_sku_id: Option<String>,
    pub product_type: Option<String>,
    pub validation_data: Option<ValidationData>,
    #[serde(rename = "SandboxId")]
    pub sandbox_id: Option<String>,
    pub is_sandboxed_product: Option<bool>,
    pub merchandizing_tags: Option<Vec<serde_json::Value>>,
    pub part_d: Option<String>,
    pub product_family: Option<String>,
    pub schema_version: Option<String>,
    pub product_kind: Option<String>,
    pub display_sku_availabilities: Option<Vec<DisplaySkuAvailability>>,
}

// ---------------------------------------------------------------------------
// Product sub-types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationData {
    pub passed_validation: Option<bool>,
    pub revision_id: Option<String>,
    pub validation_result_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
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
    #[serde(rename = "XboxXPA")]
    pub xbox_xpa: Option<serde_json::Value>,
    pub ownership_type: Option<serde_json::Value>,
    pub pdp_background_color: Option<String>,
    pub has_add_ons: Option<bool>,
    pub revision_id: Option<String>,
    pub product_group_id: Option<String>,
    pub product_group_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
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
#[serde(rename_all = "PascalCase")]
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
#[serde(rename_all = "PascalCase")]
pub struct ContentRating {
    pub rating_system: Option<String>,
    pub rating_id: Option<String>,
    pub rating_descriptors: Option<Vec<String>>,
    pub rating_disclaimers: Option<Vec<serde_json::Value>>,
    pub interactive_elements: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ProductLocalizedProperty {
    pub developer_name: Option<String>,
    pub display_platform_properties: Option<serde_json::Value>,
    pub publisher_name: Option<String>,
    pub publisher_website_uri: Option<String>,
    pub support_uri: Option<String>,
    pub eligibility_properties: Option<serde_json::Value>,
    pub franchises: Option<Vec<serde_json::Value>>,
    pub images: Option<Vec<Image>>,
    pub videos: Option<Vec<serde_json::Value>>,
    pub product_description: Option<String>,
    pub product_title: Option<String>,
    pub short_title: Option<String>,
    pub sort_title: Option<String>,
    pub short_description: Option<String>,
    pub search_titles: Option<Vec<SearchTitle>>,
    pub voice_title: Option<String>,
    pub render_group_details: Option<serde_json::Value>,
    pub product_display_ranks: Option<Vec<serde_json::Value>>,
    pub language: Option<String>,
    pub markets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SearchTitle {
    pub search_title_string: Option<String>,
    pub search_title_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    pub background_color: Option<String>,
    pub caption: Option<String>,
    pub file_size_in_bytes: Option<i64>,
    pub foreground_color: Option<String>,
    pub height: Option<i64>,
    pub image_position_info: Option<String>,
    pub image_purpose: Option<String>,
    pub unscaled_image_sha256_hash: Option<String>,
    pub uri: Option<String>,
    pub width: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct AlternateId {
    pub id_type: Option<String>,
    pub value: Option<String>,
}

// ---------------------------------------------------------------------------
// Sku / DisplaySkuAvailability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DisplaySkuAvailability {
    pub sku: Option<Sku>,
    pub availabilities: Option<Vec<Availability>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Sku {
    pub last_modified_date: Option<String>,
    pub localized_properties: Option<Vec<SkuLocalizedProperty>>,
    pub market_properties: Option<Vec<SkuMarketProperty>>,
    pub properties: Option<SkuProperties>,
    #[serde(rename = "SkuASchema")]
    pub sku_a_schema: Option<String>,
    #[serde(rename = "SkuBSchema")]
    pub sku_b_schema: Option<String>,
    pub sku_id: Option<String>,
    pub sku_type: Option<String>,
    pub recurrence_policy: Option<serde_json::Value>,
    pub subscription_policy_id: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SkuProperties {
    pub early_adopter_enrollment_url: Option<serde_json::Value>,
    pub fulfillment_data: Option<FulfillmentData>,
    pub fulfillment_type: Option<String>,
    pub has_third_party_ia_ps: Option<bool>,
    pub last_update_date: Option<String>,
    pub hardware_properties: Option<HardwareProperties>,
    pub hardware_requirements: Option<Vec<serde_json::Value>>,
    pub hardware_warning_list: Option<Vec<serde_json::Value>>,
    pub installation_terms: Option<String>,
    pub packages: Option<Vec<Package>>,
    pub version_string: Option<String>,
    pub visible_to_b2b_service_ids: Option<Vec<serde_json::Value>>,
    #[serde(rename = "XboxXPA")]
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
#[serde(rename_all = "PascalCase")]
pub struct FulfillmentData {
    pub product_id: Option<String>,
    #[serde(rename = "WuBundleId")]
    pub wu_bundle_id: Option<String>,
    #[serde(rename = "WuCategoryId")]
    pub wu_category_id: Option<String>,
    pub package_family_name: Option<String>,
    pub sku_id: Option<String>,
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct HardwareProperties {
    pub minimum_hardware: Option<Vec<serde_json::Value>>,
    pub recommended_hardware: Option<Vec<String>>,
    pub minimum_processor: Option<String>,
    pub recommended_processor: Option<String>,
    pub minimum_graphics: Option<String>,
    pub recommended_graphics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub applications: Option<Vec<Application>>,
    pub architectures: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
    pub device_capabilities: Option<Vec<serde_json::Value>>,
    pub experience_ids: Option<Vec<serde_json::Value>>,
    pub framework_dependencies: Option<Vec<serde_json::Value>>,
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
#[serde(rename_all = "PascalCase")]
pub struct PackageDownloadUri {
    pub uri: Option<String>,
    pub rank: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PlatformDependency {
    // The API returns these as either integers or quoted strings (e.g. "0").
    pub max_tested: Option<serde_json::Value>,
    pub min_version: Option<serde_json::Value>,
    pub platform_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Application {
    pub application_id: Option<String>,
    pub declaration_order: Option<i64>,
    pub extensions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SkuMarketProperty {
    pub first_available_date: Option<String>,
    pub supported_languages: Option<Vec<String>>,
    pub package_ids: Option<serde_json::Value>,
    pub markets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
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
#[serde(rename_all = "PascalCase")]
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
#[serde(rename_all = "PascalCase")]
pub struct Availability {
    pub actions: Option<Vec<String>>,
    #[serde(rename = "AvailabilityASchema")]
    pub availability_a_schema: Option<String>,
    #[serde(rename = "AvailabilityBSchema")]
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct AvailabilityProperties {
    pub original_release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct OrderManagementData {
    pub granted_entitlement_keys: Option<Vec<serde_json::Value>>,
    #[serde(rename = "PIFilter")]
    pub pi_filter: Option<PiFilter>,
    pub price: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Price {
    pub currency_code: Option<String>,
    #[serde(rename = "IsPIRequired")]
    pub is_pi_required: Option<bool>,
    pub list_price: Option<f64>,
    #[serde(rename = "MSRP")]
    pub msrp: Option<f64>,
    pub tax_type: Option<String>,
    pub wholesale_currency_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PiFilter {
    pub exclusion_properties: Option<Vec<serde_json::Value>>,
    pub inclusion_properties: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Conditions {
    pub client_conditions: Option<ClientConditions>,
    pub end_date: Option<String>,
    pub resource_set_ids: Option<Vec<String>>,
    pub start_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ClientConditions {
    pub allowed_platforms: Option<Vec<AllowedPlatform>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct AllowedPlatform {
    // The API returns these as either integers or quoted strings (e.g. "0").
    pub max_version: Option<serde_json::Value>,
    pub min_version: Option<serde_json::Value>,
    pub platform_name: Option<String>,
}
