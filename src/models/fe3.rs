use crate::models::enums::PackageType;
use serde::{Deserialize, Serialize};

/// A named hash returned by FE3 (`AdditionalDigest`, `PiecesHashDigest`,
/// `BlockMapDigest`). `algorithm` is the wire string (e.g. `"SHA256"`).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestEntry {
    pub algorithm: String,
    pub value: String,
}

/// One `<FileLocation>` from a `GetExtendedUpdateInfo2` response.
///
/// FE3 typically returns several `FileLocation`s per update — the binary's
/// download URL plus the blockmap's, and on some responses both an
/// unsigned CDN edge URL and a signed `tlu.dl.delivery.mp.microsoft.com`
/// URL with auth query params. `digest` ties the location back to a
/// specific `<File>` entry (the matching `Digest` attribute) so callers
/// can distinguish "the binary" from "the blockmap" without URL heuristics.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFileLocation {
    pub url: String,
    /// Per-URL hash from `<FileDigest>` — base64-encoded, matches the
    /// `Digest` attribute on the `<File>` this URL serves.
    pub digest: Option<String>,
}

/// A resolved package instance with download URI and update metadata.
///
/// Almost every field beyond the first eight is sourced from the FE3
/// `SyncUpdates` response — see the parser in `services::fe3` for the
/// exact XML path. Fields are `Option`/`Vec` so absence is just `None`/
/// empty; nothing here will panic at construction time.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInstance {
    pub package_moniker: String,
    /// Primary download URL (the first non-blockmap URL returned by FE3).
    /// See [`Self::all_file_locations`] for *all* URLs FE3 returned,
    /// including blockmaps and signed/secured alternatives.
    pub package_uri: Option<String>,
    pub package_type: PackageType,
    pub applicability_blob: Option<ApplicabilityBlob>,
    pub update_id: String,
    /// Download size in bytes. Sourced from `<File Size="">` (SyncUpdates)
    /// for the primary binary, falling back to `<ExtendedProperties
    /// MaxDownloadSize="">`, then to DisplayCatalog's
    /// `MaxDownloadSizeInBytes`. `None` only when none of the three carry
    /// a value.
    #[serde(rename = "packageSize")]
    pub file_size: Option<i64>,
    /// FE3's raw `<File FileName="...">` value — typically a GUID followed
    /// by the real extension (`.appx`, `.appxbundle`, `.msixbundle`,
    /// `.eappx`, `.xap`). `None` if the SyncUpdates response had no matching
    /// `<File>` element (rare).
    pub file_name: Option<String>,
    /// Human-readable filename suitable for saving the package to disk:
    /// `<package_moniker><real extension>`, where the extension is taken
    /// from [`Self::file_name`] (whitelisted to known package formats) and
    /// defaults to `.appx` when the raw name is missing or unrecognised.
    ///
    /// Example: `4DF9E0F8.Netflix_8.156.0.0_neutral_~_mcm4njqhnhss8.appxbundle`.
    ///
    /// This is *not* sanitised for any particular filesystem — callers that
    /// write to disk should sanitise per-OS (`:` `*` `?` etc. on Windows).
    pub readable_file_name: String,

    // ---------------------------------------------------------------
    // <AppxMetadata IsAppxBundle="...">
    // ---------------------------------------------------------------
    /// `true` if the primary file is a bundle (`.appxbundle` / `.msixbundle`).
    pub is_appx_bundle: Option<bool>,

    // ---------------------------------------------------------------
    // <File> attributes (primary binary — the one whose
    //   InstallerSpecificIdentifier matches the moniker)
    // ---------------------------------------------------------------
    /// Per-binary hash — base64-encoded. Algorithm in
    /// [`Self::digest_algorithm`] (`"SHA1"` for every package observed so
    /// far). Use this to verify the download after fetching.
    pub digest: Option<String>,
    pub digest_algorithm: Option<String>,
    /// Last-modified timestamp from `<File Modified="">`, ISO 8601.
    pub modified: Option<String>,
    /// Present on companion files (blockmaps/CABs); absent on the primary
    /// binary. e.g. `"DynamicMetadata"`.
    pub patching_type: Option<String>,
    /// Extra `<AdditionalDigest Algorithm="...">value</AdditionalDigest>`
    /// children of the `<File>` element.
    pub additional_digests: Vec<DigestEntry>,
    /// `<PiecesHashDigest>` — used for delta/range downloads.
    pub pieces_hash_digest: Option<DigestEntry>,
    /// `<BlockMapDigest>` — hash of the package's `.appxblockmap.xml`.
    pub block_map_digest: Option<DigestEntry>,

    // ---------------------------------------------------------------
    // <ExtendedProperties> attributes (the rich variant — only some
    // updates carry these; lightweight packages have them all `None`)
    // ---------------------------------------------------------------
    /// Update handler URI, e.g.
    /// `"http://schemas.microsoft.com/msus/2002/12/UpdateHandlers/AppxPackage"`.
    pub handler: Option<String>,
    /// Authoritative framework flag from `<ExtendedProperties
    /// IsAppxFramework="">`. Prefer this over moniker-prefix heuristics.
    pub is_appx_framework: Option<bool>,
    pub max_download_size: Option<i64>,
    pub min_download_size: Option<i64>,
    /// Store-side content identifier for this specific package.
    pub package_content_id: Option<String>,
    /// PFN base, e.g. `"4DF9E0F8.NETFLIX"`.
    pub package_identity_name: Option<String>,
    pub creation_date: Option<String>,
    pub content_type: Option<String>,
    pub mandatory_version: Option<String>,
    pub mandatory_date: Option<String>,
    pub default_properties_language: Option<String>,
    pub from_store_service: Option<bool>,
    pub legacy_mobile_product_id: Option<String>,

    // ---------------------------------------------------------------
    // <AppxPackageInstallData>
    // ---------------------------------------------------------------
    /// `true` for the primary package in a bundle; `false` for satellites
    /// (resource/language/scale split packages).
    pub main_package: Option<bool>,

    // ---------------------------------------------------------------
    // <FileLocation> entries (GetExtendedUpdateInfo2)
    // ---------------------------------------------------------------
    /// Every URL FE3 returned for this update, including blockmap URLs
    /// and signed alternatives. Useful when the primary URL is rate-limited
    /// and a fallback is needed. Empty until URLs have been resolved.
    pub all_file_locations: Vec<ResolvedFileLocation>,
}

impl PackageInstance {
    /// Return the canonical extension (`.appx`, `.appxbundle`, `.msixbundle`,
    /// `.eappx`, `.eappxbundle`, `.emsix`, `.emsixbundle`, `.msix`, `.xap`)
    /// for an FE3 `FileName` value, or `None` when the input has no
    /// extension or one that isn't in the package whitelist.
    pub fn package_extension(file_name: &str) -> Option<&'static str> {
        let dot = file_name.rfind('.')?;
        match &file_name[dot..].to_ascii_lowercase()[..] {
            ".appx" => Some(".appx"),
            ".appxbundle" => Some(".appxbundle"),
            ".msix" => Some(".msix"),
            ".msixbundle" => Some(".msixbundle"),
            ".eappx" => Some(".eappx"),
            ".eappxbundle" => Some(".eappxbundle"),
            ".emsix" => Some(".emsix"),
            ".emsixbundle" => Some(".emsixbundle"),
            ".xap" => Some(".xap"),
            _ => None,
        }
    }

    /// Build the canonical `readable_file_name` from a moniker + raw FE3
    /// FileName. Falls back to `.appx` when `file_name` is missing or has
    /// an unrecognised extension.
    pub fn build_readable_file_name(moniker: &str, file_name: Option<&str>) -> String {
        let ext = file_name
            .and_then(Self::package_extension)
            .unwrap_or(".appx");
        format!("{moniker}{ext}")
    }
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
        let moniker = "Microsoft.Test_1.0_x64";
        let file_name = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.appx";
        PackageInstance {
            package_moniker: moniker.into(),
            package_uri: Some("https://download.example/pkg.appx".into()),
            package_type: PackageType::AppX,
            update_id: "11111111-2222-3333-4444-555555555555".into(),
            file_size: Some(12345),
            file_name: Some(file_name.into()),
            readable_file_name: PackageInstance::build_readable_file_name(moniker, Some(file_name)),
            ..Default::default()
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
        assert!(
            json.contains("\"fileName\":\"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.appx\""),
            "got: {json}",
        );
        assert!(
            json.contains("\"readableFileName\":\"Microsoft.Test_1.0_x64.appx\""),
            "got: {json}",
        );
        // No snake_case or PascalCase leaks.
        assert!(!json.contains("file_size"), "got: {json}");
        assert!(!json.contains("file_name"), "got: {json}");
        assert!(!json.contains("readable_file_name"), "got: {json}");
        assert!(!json.contains("package_uri"), "got: {json}");
        assert!(!json.contains("PackageUri"), "got: {json}");
    }

    #[test]
    fn package_extension_whitelist() {
        assert_eq!(
            PackageInstance::package_extension("abc.appx"),
            Some(".appx")
        );
        assert_eq!(
            PackageInstance::package_extension("abc.APPXBUNDLE"),
            Some(".appxbundle"),
        );
        assert_eq!(
            PackageInstance::package_extension("abc.Msix"),
            Some(".msix"),
        );
        assert_eq!(
            PackageInstance::package_extension("abc.eappxbundle"),
            Some(".eappxbundle"),
        );
        assert_eq!(PackageInstance::package_extension("abc.xap"), Some(".xap"));
        assert_eq!(PackageInstance::package_extension("abc.weird"), None);
        assert_eq!(PackageInstance::package_extension("noext"), None);
    }

    #[test]
    fn build_readable_file_name_defaults_to_appx() {
        assert_eq!(
            PackageInstance::build_readable_file_name("Foo.Bar_1.0_x64", Some("guid.appxbundle"),),
            "Foo.Bar_1.0_x64.appxbundle",
        );
        assert_eq!(
            PackageInstance::build_readable_file_name("Foo.Bar_1.0_x64", None),
            "Foo.Bar_1.0_x64.appx",
        );
        assert_eq!(
            PackageInstance::build_readable_file_name("Foo.Bar_1.0_x64", Some("guid.weird")),
            "Foo.Bar_1.0_x64.appx",
        );
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
