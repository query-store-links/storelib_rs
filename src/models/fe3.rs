use crate::models::enums::PackageType;
use serde::{Deserialize, Serialize};

/// A resolved package instance with download URI and update metadata.
///
/// Serializes as camelCase, so JS consumers see:
/// ```text
/// {
///   packageMoniker: string,
///   packageUri: string | null,
///   packageType: "uap" | "xap" | "appX" | "unknown",
///   applicabilityBlob: object | null,
///   updateId: string,
///   packageSize: number | null,   // bytes; FE3 first, DCat fallback
/// }
/// ```
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInstance {
    pub package_moniker: String,
    pub package_uri: Option<String>,
    pub package_type: PackageType,
    pub applicability_blob: Option<ApplicabilityBlob>,
    pub update_id: String,
    /// Download size in bytes. Prefer the FE3-reported size; fall back to
    /// DisplayCatalog's `MaxDownloadSizeInBytes`. `None` for framework
    /// packages (e.g. VCLibs) that aren't listed in the catalog SKU but
    /// are still returned by FE3.
    #[serde(rename = "packageSize")]
    pub file_size: Option<i64>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::enums::PackageType;

    fn sample_instance() -> PackageInstance {
        PackageInstance {
            package_moniker: "Microsoft.Test_1.0_x64".into(),
            package_uri: Some("https://download.example/pkg.appx".into()),
            package_type: PackageType::AppX,
            applicability_blob: None,
            update_id: "11111111-2222-3333-4444-555555555555".into(),
            file_size: Some(12345),
        }
    }

    #[test]
    fn package_instance_serializes_camel_case() {
        let json = serde_json::to_string(&sample_instance()).unwrap();
        assert!(json.contains("\"packageMoniker\":"), "got: {json}");
        assert!(json.contains("\"packageUri\":"), "got: {json}");
        assert!(json.contains("\"packageType\":"), "got: {json}");
        assert!(json.contains("\"applicabilityBlob\":"), "got: {json}");
        assert!(json.contains("\"updateId\":"), "got: {json}");
        // file_size is renamed to packageSize on the wire.
        assert!(json.contains("\"packageSize\":12345"), "got: {json}");
        // No snake_case or PascalCase leaks.
        assert!(!json.contains("file_size"), "got: {json}");
        assert!(!json.contains("package_uri"), "got: {json}");
        assert!(!json.contains("PackageUri"), "got: {json}");
    }

    #[test]
    fn package_instance_package_size_null_when_unknown() {
        let mut p = sample_instance();
        p.file_size = None;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"packageSize\":null"), "got: {json}");
    }

    #[test]
    fn package_instance_package_type_emits_camel_case_enum() {
        let mut p = sample_instance();
        p.package_type = PackageType::AppX;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"packageType\":\"appX\""), "got: {json}");

        p.package_type = PackageType::Unknown;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"packageType\":\"unknown\""), "got: {json}");
    }

    #[test]
    fn applicability_blob_round_trip_preserves_dotted_keys() {
        // FE3 wire format uses dot-separated keys; we deliberately pass them
        // through unchanged so consumers can correlate with the SOAP payload.
        let json = r#"{"blob.version":1,"content.isMain":true,"content.packageId":"abc"}"#;
        let blob: ApplicabilityBlob = serde_json::from_str(json).unwrap();
        assert_eq!(blob.blob_version, Some(1));
        assert_eq!(blob.content_is_main, Some(true));
        assert_eq!(blob.content_package_id.as_deref(), Some("abc"));

        let out = serde_json::to_string(&blob).unwrap();
        assert!(out.contains("\"blob.version\":1"), "got: {out}");
        assert!(out.contains("\"content.isMain\":true"), "got: {out}");
    }
}
