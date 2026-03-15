use crate::error::StoreError;
use crate::models::catalog::DisplayCatalogModel;
use crate::models::enums::{DCatEndpoint, DeviceFamily, DisplayCatalogResult, IdentifierType};
use crate::models::fe3::PackageInstance;
use crate::models::locale::Locale;
use crate::models::search::DCatSearch;
use crate::services::fe3::FE3Handler;
use crate::utilities::helpers::{create_dcat_uri, endpoint_to_search_url};
use log::{debug, error, info, warn};

/// High-level client for the Microsoft DisplayCatalog API.
///
/// Wraps product lookups, package resolution via FE3, and search queries.
pub struct DisplayCatalogHandler {
    pub product_listing: Option<DisplayCatalogModel>,
    pub error: Option<String>,
    pub selected_endpoint: DCatEndpoint,
    pub result: Option<DisplayCatalogResult>,
    pub device_family: DeviceFamily,
    pub search_result: Option<DCatSearch>,
    pub id: Option<String>,
    pub selected_locale: Locale,
    pub is_found: bool,
    client: reqwest::Client,
}

impl DisplayCatalogHandler {
    /// Create a new handler pointing at the given endpoint with the given locale.
    pub fn new(endpoint: DCatEndpoint, locale: Locale) -> Self {
        let client = Self::build_client();
        DisplayCatalogHandler {
            product_listing: None,
            error: None,
            selected_endpoint: endpoint,
            result: None,
            device_family: DeviceFamily::Desktop,
            search_result: None,
            id: None,
            selected_locale: locale,
            is_found: false,
            client,
        }
    }

    /// Convenience constructor for the production endpoint with the default
    /// US/en locale.
    pub fn production() -> Self {
        Self::new(DCatEndpoint::Production, Locale::production())
    }

    fn build_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("StoreLib")
            .build()
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // Product query
    // -----------------------------------------------------------------------

    /// Query DisplayCatalog for a product by its `id` and `id_type`.
    ///
    /// On success `self.product_listing` and `self.is_found` are updated.
    /// An optional `auth_token` may be provided for flighted/sandbox queries.
    pub async fn query_dcat(
        &mut self,
        id: &str,
        id_type: IdentifierType,
        auth_token: Option<&str>,
    ) -> Result<(), StoreError> {
        self.id = Some(id.to_owned());
        self.result = None;
        self.is_found = false;

        let url = create_dcat_uri(&self.selected_endpoint, id, &id_type, &self.selected_locale);
        debug!("DCat query: GET {url}");

        let mut req = self.client.get(&url);
        if let Some(token) = auth_token {
            if !token.is_empty() {
                debug!("DCat query: attaching Authentication header");
                req = req.header("Authentication", token);
            }
        }

        let response = req.send().await.map_err(|e| {
            if e.is_timeout() {
                warn!("DCat query timed out for id={id}");
                StoreError::TimedOut
            } else {
                StoreError::Http(e)
            }
        })?;

        let status = response.status();
        debug!("DCat response: HTTP {status}");

        if status.is_success() {
            let body = response.text().await.map_err(StoreError::Http)?;
            debug!("DCat response body: {} bytes", body.len());
            let model: DisplayCatalogModel = serde_json::from_str(&body).map_err(|e| {
                error!("DCat JSON parse error: {e}");
                log_json_context(&body, e.column());
                StoreError::Json(e)
            })?;
            let title = model
                .products
                .as_deref()
                .and_then(|v| v.first())
                .or(model.product.as_ref())
                .and_then(|p| p.localized_properties.as_deref())
                .and_then(|v| v.first())
                .and_then(|lp| lp.product_title.as_deref())
                .unwrap_or("<no title>");
            info!("DCat found: \"{title}\" (id={id})");
            self.product_listing = Some(model);
            self.result = Some(DisplayCatalogResult::Found);
            self.is_found = true;
            Ok(())
        } else if status == reqwest::StatusCode::NOT_FOUND {
            warn!("DCat: product not found (id={id})");
            self.result = Some(DisplayCatalogResult::NotFound);
            Err(StoreError::NotFound)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(StoreError::Other(format!(
                "Failed to query DisplayCatalog endpoint {:?}, status {}, body: {}",
                self.selected_endpoint, status, body
            )))
        }
    }

    // -----------------------------------------------------------------------
    // Package resolution via FE3
    // -----------------------------------------------------------------------

    /// Resolve the direct download URLs for the currently-loaded product.
    ///
    /// Requires `query_dcat` to have been called successfully first.
    pub async fn get_packages_for_product(
        &self,
        msa_token: Option<&str>,
    ) -> Result<Vec<PackageInstance>, StoreError> {
        let listing = self.product_listing.as_ref().ok_or_else(|| {
            StoreError::Other("Cannot get packages: product data is null.".into())
        })?;

        // Prefer Products list, fall back to single Product field.
        let product = listing
            .products
            .as_deref()
            .and_then(|v| v.first())
            .or(listing.product.as_ref())
            .ok_or_else(|| {
                StoreError::Other("Cannot get packages: product data is null.".into())
            })?;

        let wu_category_id = product
            .display_sku_availabilities
            .as_deref()
            .and_then(|v| v.first())
            .and_then(|dsa| dsa.sku.as_ref())
            .and_then(|sku| sku.properties.as_ref())
            .and_then(|props| props.fulfillment_data.as_ref())
            .and_then(|fd| fd.wu_category_id.as_deref())
            .ok_or_else(|| {
                StoreError::Other(
                    "Cannot get packages: FulfillmentData (WuCategoryId) is missing.".into(),
                )
            })?;

        debug!("FE3: WuCategoryId={wu_category_id}");

        let xml = FE3Handler::sync_updates(wu_category_id, msa_token, &self.client).await?;

        let (update_ids, revision_ids) = FE3Handler::process_update_ids(&xml)?;
        debug!("FE3: {} update ID(s) parsed", update_ids.len());

        let mut instances = FE3Handler::get_package_instances(&xml).await?;
        debug!("FE3: {} package instance(s) found", instances.len());

        let urls =
            FE3Handler::get_file_urls(&update_ids, &revision_ids, msa_token, &self.client).await?;
        debug!("FE3: {} download URL(s) resolved", urls.len());

        // Build a moniker → file-size lookup from the DCat catalog packages.
        // PackageFullName in DCat equals PackageMoniker in FE3.
        let dcat_size_map: std::collections::HashMap<&str, i64> = product
            .display_sku_availabilities
            .as_deref()
            .iter()
            .flat_map(|v| v.iter())
            .flat_map(|dsa| {
                dsa.sku
                    .as_ref()
                    .and_then(|s| s.properties.as_ref())
                    .and_then(|p| p.packages.as_deref())
                    .unwrap_or(&[])
            })
            .filter_map(|pkg| {
                let name = pkg.package_full_name.as_deref()?;
                let size = pkg.max_download_size_in_bytes?;
                Some((name, size))
            })
            .collect();

        for (i, instance) in instances.iter_mut().enumerate() {
            instance.package_uri = urls.get(i).cloned();
            instance.update_id = update_ids.get(i).cloned().unwrap_or_default();
            instance.file_size = dcat_size_map.get(instance.package_moniker.as_str()).copied();
        }

        info!("Resolved {} package(s)", instances.len());
        Ok(instances)
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// Search DisplayCatalog for the given query string.
    pub async fn search_dcat(
        &mut self,
        query: &str,
        device_family: DeviceFamily,
    ) -> Result<DCatSearch, StoreError> {
        self.search_dcat_paged(query, device_family, 0).await
    }

    /// Search DisplayCatalog for the given query string, skipping `skip_count`
    /// results (each page holds up to 100 results).
    pub async fn search_dcat_paged(
        &mut self,
        query: &str,
        device_family: DeviceFamily,
        skip_count: u32,
    ) -> Result<DCatSearch, StoreError> {
        let base = endpoint_to_search_url(&self.selected_endpoint);
        let dep = device_family.platform_dependency_name();

        let mut url = format!(
            "{}{}&productFamilyNames=apps,games&platformDependencyName={}",
            base, query, dep
        );
        if skip_count > 0 {
            url.push_str(&format!("&skipItems={}", skip_count));
        }

        debug!("DCat search: GET {url}");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("DCat search response: HTTP {status}");

        if status.is_success() {
            let body = response.text().await.map_err(StoreError::Http)?;
            debug!("DCat search response body: {} bytes", body.len());
            self.result = Some(DisplayCatalogResult::Found);
            let result: DCatSearch = serde_json::from_str(&body).map_err(|e| {
                error!("DCat search JSON parse error: {e}");
                log_json_context(&body, e.column());
                StoreError::Json(e)
            })?;
            info!(
                "DCat search: {} result(s) for \"{query}\"",
                result.total_result_count.unwrap_or(0)
            );
            Ok(result)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(StoreError::Other(format!(
                "Failed to search DisplayCatalog for {:?}, status {}, body: {}",
                device_family, status, body
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Log a 200-byte window of JSON centred on `col` (1-based byte offset on
/// line 1, as reported by `serde_json::Error::column()`).
///
/// Emits two log lines:
///   ERROR  … the ~40 chars immediately around the offending token
///   DEBUG  … a wider 200-char window for surrounding context
fn log_json_context(body: &str, col: usize) {
    // serde_json column() is 1-based; clamp to valid range.
    let pos = col.saturating_sub(1).min(body.len());

    // Narrow window: show the token itself and a few chars on each side.
    const NARROW: usize = 40;
    let narrow_start = pos.saturating_sub(NARROW);
    let narrow_end = (pos + NARROW).min(body.len());
    let narrow = &body[narrow_start..narrow_end];
    let arrow_offset = pos - narrow_start;
    error!(
        "JSON context (col {}): …{}…\n{:>width$}",
        col,
        narrow,
        "^",
        width = arrow_offset + 1
    );

    // Wide window: enough context to see the enclosing field name.
    const WIDE: usize = 200;
    let wide_start = pos.saturating_sub(WIDE);
    let wide_end = (pos + WIDE).min(body.len());
    debug!(
        "JSON wide context (cols {}–{}):\n{}",
        wide_start,
        wide_end,
        &body[wide_start..wide_end]
    );
}
