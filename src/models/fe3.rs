use crate::models::enums::PackageType;
use serde::{Deserialize, Serialize};

/// A resolved package instance with download URI and update metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PackageInstance {
    pub package_moniker: String,
    pub package_uri: Option<String>,
    pub package_type: PackageType,
    pub applicability_blob: Option<ApplicabilityBlob>,
    pub update_id: String,
}

/// Applicability metadata embedded in the FE3 SyncUpdates response.
///
/// Note: the original JSON uses dot-separated keys such as
/// `"blob.version"`, `"content.isMain"` etc.  Those are preserved here
/// via explicit `#[serde(rename = "...")]` attributes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicabilityBlob {
    #[serde(rename = "blob.version")]
    pub blob_version: Option<i64>,

    #[serde(rename = "content.isMain")]
    pub content_is_main: Option<bool>,

    #[serde(rename = "content.packageId")]
    pub content_package_id: Option<String>,

    #[serde(rename = "content.productId")]
    pub content_product_id: Option<String>,

    #[serde(rename = "content.targetPlatforms")]
    pub content_target_platforms: Option<Vec<ContentTargetPlatform>>,

    #[serde(rename = "content.type")]
    pub content_type: Option<i32>,
}

/// Per-platform targeting information inside an `ApplicabilityBlob`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentTargetPlatform {
    #[serde(rename = "platform.maxVersionTested")]
    pub platform_max_version_tested: Option<i64>,

    #[serde(rename = "platform.minVersion")]
    pub platform_min_version: Option<i64>,

    #[serde(rename = "platform.target")]
    pub platform_target: Option<i32>,
}
