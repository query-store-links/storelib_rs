use crate::error::StoreError;
use crate::models::fe3::{ApplicabilityBlob, DigestEntry, PackageInstance, ResolvedFileLocation};
use crate::services::display_catalog::ProgressEmitter;
use crate::utilities::helpers::string_to_package_type;
use log::{debug, trace, warn};

// ---------------------------------------------------------------------------
// Endpoint constants
// ---------------------------------------------------------------------------

const FE3_DELIVERY: &str = "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx";
const FE3_DELIVERY_SECURED: &str =
    "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx/secured";

// ---------------------------------------------------------------------------
// Embedded XML templates
// ---------------------------------------------------------------------------

const GET_COOKIE_XML: &str = include_str!("../xml/get_cookie.xml");
const WUID_REQUEST_XML: &str = include_str!("../xml/wuid_request.xml");
const FE3_FILE_URL_XML: &str = include_str!("../xml/fe3_file_url.xml");

// ---------------------------------------------------------------------------
// Default MSA device token (from original StoreLib C# source)
// ---------------------------------------------------------------------------

const MSA_TOKEN: &str = "<Device>dAA9AEUAdwBBAHcAQQBzAE4AMwBCAEEAQQBVADEAYgB5AHMAZQBtAGIAZQBEAFYAQwArADMAZgBtADcAbwBXAHkASAA3AGIAbgBnAEcAWQBtAEEAQQBMAGoAbQBqAFYAVQB2AFEAYwA0AEsAVwBFAC8AYwBDAEwANQBYAGUANABnAHYAWABkAGkAegBHAGwAZABjADEAZAAvAFcAeQAvAHgASgBQAG4AVwBRAGUAYwBtAHYAbwBjAGkAZwA5AGoAZABwAE4AawBIAG0AYQBzAHAAVABKAEwARAArAFAAYwBBAFgAbQAvAFQAcAA3AEgAagBzAEYANAA0AEgAdABsAC8AMQBtAHUAcgAwAFMAdQBtAG8AMABZAGEAdgBqAFIANwArADQAcABoAC8AcwA4ADEANgBFAFkANQBNAFIAbQBnAFIAQwA2ADMAQwBSAEoAQQBVAHYAZgBzADQAaQB2AHgAYwB5AEwAbAA2AHoAOABlAHgAMABrAFgAOQBPAHcAYQB0ADEAdQBwAFMAOAAxAEgANgA4AEEASABzAEoAegBnAFQAQQBMAG8AbgBBADIAWQBBAEEAQQBpAGcANQBJADMAUQAvAFYASABLAHcANABBAEIAcQA5AFMAcQBhADEAQgA4AGsAVQAxAGEAbwBLAEEAdQA0AHYAbABWAG4AdwBWADMAUQB6AHMATgBtAEQAaQBqAGgANQBkAEcAcgBpADgAQQBlAEUARQBWAEcAbQBXAGgASQBCAE0AUAAyAEQAVwA0ADMAZABWAGkARABUAHoAVQB0AHQARQBMAEgAaABSAGYAcgBhAGIAWgBsAHQAQQBUAEUATABmAHMARQBGAFUAYQBRAFMASgB4ADUAeQBRADgAagBaAEUAZQAyAHgANABCADMAMQB2AEIAMgBqAC8AUgBLAGEAWQAvAHEAeQB0AHoANwBUAHYAdAB3AHQAagBzADYAUQBYAEIAZQA4AHMAZwBJAG8AOQBiADUAQQBCADcAOAAxAHMANgAvAGQAUwBFAHgATgBEAEQAYQBRAHoAQQBYAFAAWABCAFkAdQBYAFEARQBzAE8AegA4AHQAcgBpAGUATQBiAEIAZQBUAFkAOQBiAG8AQgBOAE8AaQBVADcATgBSAEYAOQAzAG8AVgArAFYAQQBiAGgAcAAwAHAAUgBQAFMAZQBmAEcARwBPAHEAdwBTAGcANwA3AHMAaAA5AEoASABNAHAARABNAFMAbgBrAHEAcgAyAGYARgBpAEMAUABrAHcAVgBvAHgANgBuAG4AeABGAEQAbwBXAC8AYQAxAHQAYQBaAHcAegB5AGwATAAxADIAdwB1AGIAbQA1AHUAbQBwAHEAeQBXAGMASwBSAGoAeQBoADIASgBUAEYASgBXADUAZwBYAEUASQA1AHAAOAAwAEcAdQAyAG4AeABMAFIATgB3AGkAdwByADcAVwBNAFIAQQBWAEsARgBXAE0AZQBSAHoAbAA5AFUAcQBnAC8AcABYAC8AdgBlAEwAdwBTAGsAMgBTAFMASABmAGEASwA2AGoAYQBvAFkAdQBuAFIARwByADgAbQBiAEUAbwBIAGwARgA2AEoAQwBhAGEAVABCAFgAQgBjAHYAdQBlAEMASgBvADkAOABoAFIAQQByAEcAdwA0ACsAUABIAGUAVABiAE4AUwBFAFgAWAB6AHYAWgA2AHUAVwA1AEUAQQBmAGQAWgBtAFMAOAA4AFYASgBjAFoAYQBGAEsANwB4AHgAZwAwAHcAbwBuADcAaAAwAHgAQwA2AFoAQgAwAGMAWQBqAEwAcgAvAEcAZQBPAHoAOQBHADQAUQBVAEgAOQBFAGsAeQAwAGQAeQBGAC8AcgBlAFUAMQBJAHkAaQBhAHAAcABoAE8AUAA4AFMAMgB0ADQAQgByAFAAWgBYAFQAdgBDADAA\
UAA3AHoATwArAGYARwBrAHgAVgBtACsAVQBmAFoAYgBRADUANQBzAHcARQA9ACYAcAA9AA==</Device>";

// ---------------------------------------------------------------------------
// FE3 handler
// ---------------------------------------------------------------------------
//
// Construct via [`FE3Handler::new`] for stored-callback progress
// (`fe3.linkReceived` per resolved URL via [`Self::get_file_urls`]); reach
// for the static `_with_progress` associated functions when the caller has
// its own enriched closure (e.g. `DisplayCatalogHandler::get_packages_for_product`
// attaches the owning moniker to each link event).

pub struct FE3Handler {
    pub(crate) client: reqwest::Client,
    /// Subscribe with `fe3.progress.set(Box::new(|e| ...))`.
    pub progress: ProgressEmitter,
}

impl FE3Handler {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            progress: ProgressEmitter::new(),
        }
    }

    /// Resolve direct download URLs for `(update_ids, revision_ids)`,
    /// firing one `fe3.linkReceived` event per resolved URL through
    /// [`Self::progress`]. `size` in the returned tuples is always `None`
    /// (FE3 only carries per-URL sizes in `SyncUpdates`).
    pub async fn get_file_urls(
        &self,
        update_ids: &[String],
        revision_ids: &[String],
        msa_token: Option<&str>,
    ) -> Result<Vec<(String, Option<i64>)>, StoreError> {
        let emitter = &self.progress;
        Self::get_file_urls_with_progress(
            update_ids,
            revision_ids,
            msa_token,
            &self.client,
            |idx, total, update_id, url, size| {
                emitter.emit_counter(
                    "fe3.linkReceived",
                    format!(
                        "uri={url} | size={} | updateId={update_id}",
                        size.map(|s| s.to_string()).unwrap_or_else(|| "?".into()),
                    ),
                    (idx + 1) as u32,
                    total as u32,
                );
            },
        )
        .await
    }

    // -----------------------------------------------------------------------
    // Cookie
    // -----------------------------------------------------------------------

    /// POST the GetCookie SOAP envelope to FE3 and return the `EncryptedData`
    /// value extracted from the response XML.
    pub async fn get_cookie(client: &reqwest::Client) -> Result<String, StoreError> {
        debug!("FE3: POST {FE3_DELIVERY} (GetCookie)");
        let response = client
            .post(FE3_DELIVERY)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(GET_COOKIE_XML)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("FE3 GetCookie response: HTTP {status}");

        let body = response.text().await.map_err(StoreError::Http)?;
        trace!("FE3 GetCookie body:\n{body}");

        let doc = roxmltree::Document::parse(&body).map_err(|e| StoreError::Xml(e.to_string()))?;

        let cookie = doc
            .descendants()
            .find(|n| n.tag_name().name() == "EncryptedData")
            .and_then(|n| n.text())
            .ok_or_else(|| {
                StoreError::Xml("EncryptedData node not found in cookie response".into())
            })?;

        debug!("FE3: cookie obtained ({} bytes)", cookie.len());
        Ok(cookie.to_owned())
    }

    // -----------------------------------------------------------------------
    // SyncUpdates
    // -----------------------------------------------------------------------

    /// POST a `SyncUpdates` request using a pre-obtained `cookie`. Returns the
    /// HTML-decoded SOAP response body.
    ///
    /// Most callers should use [`Self::sync_updates`], which fetches a cookie
    /// internally. This variant is for callers that need to emit a progress
    /// event between the two HTTP requests.
    pub async fn sync_updates_with_cookie(
        cookie: &str,
        wu_category_id: &str,
        msa_token: Option<&str>,
        client: &reqwest::Client,
    ) -> Result<String, StoreError> {
        let token = msa_token.unwrap_or(MSA_TOKEN);
        let body = WUID_REQUEST_XML
            .replace("{0}", cookie)
            .replace("{1}", wu_category_id)
            .replace("{2}", token);

        debug!("FE3: POST {FE3_DELIVERY} (SyncUpdates, WuCategoryId={wu_category_id})");
        let response = client
            .post(FE3_DELIVERY)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("FE3 SyncUpdates response: HTTP {status}");

        let raw = response.text().await.map_err(StoreError::Http)?;
        let decoded = html_decode(&raw);
        trace!("FE3 SyncUpdates body:\n{decoded}");
        Ok(decoded)
    }

    /// Fetch a FE3 cookie, then POST a `SyncUpdates` request for the given
    /// `wu_category_id`.  Returns the HTML-decoded SOAP response body.
    pub async fn sync_updates(
        wu_category_id: &str,
        msa_token: Option<&str>,
        client: &reqwest::Client,
    ) -> Result<String, StoreError> {
        let cookie = Self::get_cookie(client).await?;
        Self::sync_updates_with_cookie(&cookie, wu_category_id, msa_token, client).await
    }

    // -----------------------------------------------------------------------
    // Process update IDs
    // -----------------------------------------------------------------------

    /// Parse the raw `SyncUpdates` XML and extract `(update_ids, revision_ids)`.
    ///
    /// Only nodes whose XML fragment contains a `SecuredFragment` child are
    /// included, matching the logic from the original C# code.
    pub fn process_update_ids(xml: &str) -> Result<(Vec<String>, Vec<String>), StoreError> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| StoreError::Xml(e.to_string()))?;

        let mut update_ids = Vec::new();
        let mut revision_ids = Vec::new();

        for node in doc.descendants() {
            if node.tag_name().name() != "SecuredFragment" {
                continue;
            }

            // SecuredFragment -> parent (Properties) -> parent (Xml/Update element)
            // -> first_child (UpdateIdentity)
            let identity = node
                .parent()
                .and_then(|p| p.parent())
                .and_then(|gp| gp.first_element_child());

            if let Some(identity) = identity {
                if let (Some(uid), Some(rev)) = (
                    identity.attribute("UpdateID"),
                    identity.attribute("RevisionNumber"),
                ) {
                    debug!("FE3: update ID={uid} revision={rev}");
                    update_ids.push(uid.to_owned());
                    revision_ids.push(rev.to_owned());
                }
            } else {
                warn!("FE3: SecuredFragment node has unexpected parent structure; skipping");
            }
        }

        debug!("FE3: process_update_ids found {} ID(s)", update_ids.len());
        Ok((update_ids, revision_ids))
    }

    // -----------------------------------------------------------------------
    // Package instances
    // -----------------------------------------------------------------------

    /// Parse `AppxMetadata` nodes from the `SyncUpdates` XML and build
    /// [`PackageInstance`] values (without resolved download URLs).
    ///
    /// Walks three structures and stitches them together by moniker:
    /// - `<AppxMetadata PackageMoniker="...">` — package identity + type +
    ///   `<ApplicabilityBlob>` JSON.
    /// - The rich `<Update>` block whose `<Files>/<File>` has
    ///   `InstallerSpecificIdentifier == PackageMoniker` — primary-binary
    ///   hash/size/timestamp plus the parent `<ExtendedProperties>` and
    ///   `<HandlerSpecificData>/<AppxPackageInstallData>` siblings.
    /// - Child digest tags (`AdditionalDigest`, `PiecesHashDigest`,
    ///   `BlockMapDigest`) under the primary `<File>`.
    pub async fn get_package_instances(xml: &str) -> Result<Vec<PackageInstance>, StoreError> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| StoreError::Xml(e.to_string()))?;

        // First pass: index every <File> by its InstallerSpecificIdentifier
        // (which equals PackageMoniker). Blockmap entries (Abm_*.cab) have
        // no InstallerSpecificIdentifier so they don't enter the index; the
        // primary binary wins by construction.
        let mut primary_file_by_moniker: std::collections::HashMap<String, roxmltree::Node> =
            std::collections::HashMap::new();
        for node in doc.descendants() {
            if node.tag_name().name() != "File" {
                continue;
            }
            if let Some(moniker) = node.attribute("InstallerSpecificIdentifier") {
                primary_file_by_moniker
                    .entry(moniker.to_owned())
                    .or_insert(node);
            }
        }

        let mut instances = Vec::new();
        for node in doc.descendants() {
            if node.tag_name().name() != "AppxMetadata" {
                continue;
            }

            // Must have at least 3 attributes (PackageMoniker, PackageType, ...)
            let attrs: Vec<_> = node.attributes().collect();
            if attrs.len() < 3 {
                continue;
            }

            let moniker = match node.attribute("PackageMoniker") {
                Some(v) => v.to_owned(),
                None => continue,
            };
            let pkg_type_str = node.attribute("PackageType").unwrap_or("");
            let pkg_type = string_to_package_type(pkg_type_str);
            let is_appx_bundle = node
                .attribute("IsAppxBundle")
                .and_then(|v| v.parse::<bool>().ok());

            debug!("FE3: package instance moniker={moniker} type={pkg_type_str}");

            // First child text node carries the ApplicabilityBlob JSON.
            let blob: Option<ApplicabilityBlob> =
                node.first_child().and_then(|c| c.text()).and_then(|t| {
                    trace!("FE3: ApplicabilityBlob JSON: {t}");
                    serde_json::from_str(t).ok()
                });

            // ----- Per-binary <File> attributes + child digests ---------
            let primary_file = primary_file_by_moniker.get(&moniker).copied();
            let file_str = |name: &str| {
                primary_file
                    .and_then(|f| f.attribute(name))
                    .map(String::from)
            };
            let file_i64 = |name: &str| {
                primary_file
                    .and_then(|f| f.attribute(name))
                    .and_then(|s| s.parse::<i64>().ok())
            };

            let file_name = primary_file.and_then(|f| f.attribute("FileName"));
            let digest = file_str("Digest");
            let digest_algorithm = file_str("DigestAlgorithm");
            let file_size_attr = file_i64("Size");
            let modified = file_str("Modified");
            let patching_type = file_str("PatchingType");

            let mut additional_digests: Vec<DigestEntry> = Vec::new();
            let mut pieces_hash_digest: Option<DigestEntry> = None;
            let mut block_map_digest: Option<DigestEntry> = None;
            if let Some(file) = primary_file {
                for child in file.children() {
                    if !child.is_element() {
                        continue;
                    }
                    let alg = child.attribute("Algorithm").unwrap_or("").to_string();
                    let val = child.text().unwrap_or("").trim().to_string();
                    if val.is_empty() {
                        continue;
                    }
                    let entry = DigestEntry {
                        algorithm: alg,
                        value: val,
                    };
                    match child.tag_name().name() {
                        "AdditionalDigest" => additional_digests.push(entry),
                        "PiecesHashDigest" => pieces_hash_digest = Some(entry),
                        "BlockMapDigest" => block_map_digest = Some(entry),
                        _ => {}
                    }
                }
            }

            // ----- Walk up to the owning <Update>/<Xml> to pick up
            //       ExtendedProperties + AppxPackageInstallData siblings.
            // File -> Files -> Xml -> Update
            let update_xml = primary_file
                .and_then(|f| f.parent())
                .and_then(|files| files.parent());

            let ext_props = update_xml.and_then(|xml_node| {
                xml_node
                    .children()
                    .find(|c| c.tag_name().name() == "ExtendedProperties")
            });

            let ext_str = |name: &str| ext_props.and_then(|n| n.attribute(name)).map(String::from);
            let ext_bool = |name: &str| {
                ext_props
                    .and_then(|n| n.attribute(name))
                    .and_then(|v| v.parse::<bool>().ok())
            };
            let ext_i64 = |name: &str| {
                ext_props
                    .and_then(|n| n.attribute(name))
                    .and_then(|s| s.parse::<i64>().ok())
            };

            let handler = ext_str("Handler");
            let is_appx_framework = ext_bool("IsAppxFramework");
            let max_download_size = ext_i64("MaxDownloadSize");
            let min_download_size = ext_i64("MinDownloadSize");
            let package_content_id = ext_str("PackageContentId");
            let package_identity_name = ext_str("PackageIdentityName");
            let creation_date = ext_str("CreationDate");
            let content_type = ext_str("ContentType");
            let mandatory_version = ext_str("MandatoryVersion");
            let mandatory_date = ext_str("MandatoryDate");
            let default_properties_language = ext_str("DefaultPropertiesLanguage");
            let from_store_service = ext_bool("FromStoreService");
            let legacy_mobile_product_id = ext_str("LegacyMobileProductId");

            let main_package = update_xml
                .and_then(|xml_node| {
                    xml_node
                        .descendants()
                        .find(|c| c.tag_name().name() == "AppxPackageInstallData")
                })
                .and_then(|n| n.attribute("MainPackage"))
                .and_then(|v| v.parse::<bool>().ok());

            // file_size: prefer <File Size>, fall back to
            // <ExtendedProperties MaxDownloadSize>. (DCat is the final
            // fallback, applied by display_catalog.)
            let file_size = file_size_attr.or(max_download_size);

            let readable_file_name = PackageInstance::build_readable_file_name(&moniker, file_name);

            instances.push(PackageInstance {
                package_moniker: moniker,
                package_uri: None,
                package_type: pkg_type,
                applicability_blob: blob,
                update_id: String::new(),
                file_size,
                file_name: file_name.map(String::from),
                readable_file_name,

                is_appx_bundle,

                digest,
                digest_algorithm,
                modified,
                patching_type,
                additional_digests,
                pieces_hash_digest,
                block_map_digest,

                handler,
                is_appx_framework,
                max_download_size,
                min_download_size,
                package_content_id,
                package_identity_name,
                creation_date,
                content_type,
                mandatory_version,
                mandatory_date,
                default_properties_language,
                from_store_service,
                legacy_mobile_product_id,

                main_package,

                all_file_locations: Vec::new(),
            });
        }

        Ok(instances)
    }

    // -----------------------------------------------------------------------
    // File URLs
    // -----------------------------------------------------------------------

    /// For each `(update_id, revision_id)` pair, POST a
    /// `GetExtendedUpdateInfo2` SOAP request to FE3 and fire `on_url` once
    /// per successfully-resolved (non-blockmap) URL **as soon as the SOAP
    /// response is parsed**, before the next request goes out. Returns
    /// `Vec<(url, size_or_none)>` keyed parallel to the inputs.
    ///
    /// Use this static when you need a custom per-link closure (e.g. the
    /// DCat path attaches the owning package's moniker). For the common
    /// case where `fe3.linkReceived` events should flow through a
    /// pre-installed handler-level callback, use the instance method
    /// [`Self::get_file_urls`] instead.
    ///
    /// Callback args: `(request_idx, request_total, update_id, url, size)`.
    /// `size` is always `None` — FE3 does not return per-URL sizes in
    /// `GetExtendedUpdateInfo2` responses (the size lives on `<File Size>`
    /// in SyncUpdates). Kept in the signature for source compatibility.
    pub async fn get_file_urls_with_progress<F>(
        update_ids: &[String],
        revision_ids: &[String],
        msa_token: Option<&str>,
        client: &reqwest::Client,
        on_url: F,
    ) -> Result<Vec<(String, Option<i64>)>, StoreError>
    where
        F: Fn(usize, usize, &str, &str, Option<i64>),
    {
        let per_update = Self::get_file_locations_with_progress(
            update_ids,
            revision_ids,
            msa_token,
            client,
            |idx, total, update_id, loc| {
                on_url(idx, total, update_id, &loc.url, None);
            },
        )
        .await?;

        // Match the legacy single-URL-per-update contract: pick the first
        // location for each update_id (the primary binary; blockmaps are
        // filtered upstream by digest matching).
        Ok(per_update
            .into_iter()
            .map(|locs| {
                locs.into_iter()
                    .next()
                    .map(|l| (l.url, None))
                    .unwrap_or_default()
            })
            .filter(|(u, _)| !u.is_empty())
            .collect())
    }

    /// Rich variant of [`Self::get_file_urls_with_progress`] that returns
    /// *every* `<FileLocation>` for each update — primary binary, blockmap,
    /// and signed alternative — along with the per-URL `<FileDigest>` so
    /// callers can match a URL back to its `<File>` entry. Fires `on_loc`
    /// once per resolved `<FileLocation>` as soon as the SOAP response is
    /// parsed.
    ///
    /// Returns a `Vec` indexed parallel to `update_ids`: each inner `Vec`
    /// is the file locations FE3 returned for that update, in document
    /// order. Blockmaps are NOT filtered here — that's a job for the
    /// caller (match `digest` against `PackageInstance::digest` to pick
    /// the binary, or against `PackageInstance::block_map_digest` to pick
    /// the blockmap).
    ///
    /// Callback args: `(request_idx, request_total, update_id, &location)`.
    pub async fn get_file_locations_with_progress<F>(
        update_ids: &[String],
        revision_ids: &[String],
        msa_token: Option<&str>,
        client: &reqwest::Client,
        on_loc: F,
    ) -> Result<Vec<Vec<ResolvedFileLocation>>, StoreError>
    where
        F: Fn(usize, usize, &str, &ResolvedFileLocation),
    {
        let token = msa_token.unwrap_or(MSA_TOKEN);
        let total = update_ids.len();
        let mut all_per_update: Vec<Vec<ResolvedFileLocation>> = Vec::with_capacity(total);

        for (i, update_id) in update_ids.iter().enumerate() {
            let revision_id = match revision_ids.get(i) {
                Some(r) => r.as_str(),
                None => {
                    all_per_update.push(Vec::new());
                    continue;
                }
            };

            let body = FE3_FILE_URL_XML
                .replace("{0}", update_id)
                .replace("{1}", revision_id)
                .replace("{2}", token);

            debug!("FE3: POST {FE3_DELIVERY_SECURED} (GetExtendedUpdateInfo2, UpdateID={update_id} RevisionID={revision_id})");
            let response = client
                .post(FE3_DELIVERY_SECURED)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(body)
                .send()
                .await
                .map_err(StoreError::Http)?;

            let status = response.status();
            debug!("FE3 GetExtendedUpdateInfo2 response: HTTP {status}");

            let raw = response.text().await.map_err(StoreError::Http)?;
            debug!("FE3 GetExtendedUpdateInfo2 body:\n{raw}");

            let doc = match roxmltree::Document::parse(&raw) {
                Ok(d) => d,
                Err(e) => return Err(StoreError::Xml(e.to_string())),
            };

            let mut locs_for_update: Vec<ResolvedFileLocation> = Vec::new();
            for file_loc in doc.descendants() {
                if file_loc.tag_name().name() != "FileLocation" {
                    continue;
                }
                let mut url_opt: Option<String> = None;
                let mut digest_opt: Option<String> = None;
                for child in file_loc.children() {
                    match child.tag_name().name() {
                        "Url" => {
                            if let Some(text) = child.text() {
                                debug!("FE3: URL resolved: {text}");
                                url_opt = Some(text.to_owned());
                            }
                        }
                        "FileDigest" => {
                            digest_opt = child.text().map(|t| t.trim().to_owned());
                            trace!("FE3: FileDigest={digest_opt:?}");
                        }
                        _ => {}
                    }
                }
                if let Some(url) = url_opt {
                    let loc = ResolvedFileLocation {
                        url,
                        digest: digest_opt,
                    };
                    on_loc(i, total, update_id, &loc);
                    locs_for_update.push(loc);
                }
            }
            all_per_update.push(locs_for_update);
        }

        Ok(all_per_update)
    }
}

// ---------------------------------------------------------------------------
// HTML entity decoder
// ---------------------------------------------------------------------------

/// Minimal HTML entity decoder covering the entities used in SOAP responses.
pub(crate) fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_decode_all_entities() {
        assert_eq!(html_decode("a &amp; b"), "a & b");
        assert_eq!(html_decode("&lt;tag&gt;"), "<tag>");
        assert_eq!(html_decode("say &quot;hello&quot;"), "say \"hello\"");
        assert_eq!(html_decode("it&apos;s"), "it's");
    }

    #[test]
    fn html_decode_no_entities() {
        assert_eq!(html_decode("plain text"), "plain text");
    }

    #[test]
    fn html_decode_chained_entities() {
        // Replacements are sequential: &amp;lt; -> &lt; -> <
        assert_eq!(html_decode("&amp;lt;"), "<");
    }

    #[test]
    fn process_update_ids_empty_xml() {
        let xml = r#"<?xml version="1.0"?><root></root>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert!(ids.is_empty());
        assert!(revs.is_empty());
    }

    #[test]
    fn process_update_ids_parses_secured_fragment() {
        // Minimal SyncUpdates-style XML with one SecuredFragment node.
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <SyncUpdatesResult>
      <NewUpdates>
        <UpdateInfo>
          <ID>1</ID>
          <Xml>
            <UpdateIdentity UpdateID="abc-123" RevisionNumber="200"/>
            <Properties>
              <SecuredFragment/>
            </Properties>
          </Xml>
        </UpdateInfo>
      </NewUpdates>
    </SyncUpdatesResult>
  </s:Body>
</s:Envelope>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert_eq!(ids, vec!["abc-123"]);
        assert_eq!(revs, vec!["200"]);
    }

    #[test]
    fn process_update_ids_skips_nodes_without_secured_fragment() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <SyncUpdatesResult>
      <NewUpdates>
        <UpdateInfo>
          <ID>1</ID>
          <Xml>
            <UpdateIdentity UpdateID="no-fragment" RevisionNumber="1"/>
            <Properties/>
          </Xml>
        </UpdateInfo>
      </NewUpdates>
    </SyncUpdatesResult>
  </s:Body>
</s:Envelope>"#;
        let (ids, _) = FE3Handler::process_update_ids(xml).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn process_update_ids_multiple() {
        let xml = r#"<?xml version="1.0"?>
<root>
  <Update>
    <UpdateIdentity UpdateID="id-1" RevisionNumber="10"/>
    <Properties>
      <SecuredFragment/>
    </Properties>
  </Update>
  <Update>
    <UpdateIdentity UpdateID="id-2" RevisionNumber="20"/>
    <Properties>
      <SecuredFragment/>
    </Properties>
  </Update>
</root>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "id-1");
        assert_eq!(revs[0], "10");
        assert_eq!(ids[1], "id-2");
        assert_eq!(revs[1], "20");
    }

    #[test]
    fn process_update_ids_invalid_xml_returns_error() {
        let result = FE3Handler::process_update_ids("not xml at all <<<");
        assert!(result.is_err());
    }
}
