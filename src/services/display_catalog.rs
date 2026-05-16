use crate::error::StoreError;
use crate::models::catalog::DisplayCatalogModel;
use crate::models::enums::{DCatEndpoint, DeviceFamily, DisplayCatalogResult, IdentifierType};
use crate::models::fe3::PackageInstance;
use crate::models::locale::Locale;
use crate::models::search::DCatSearch;
use crate::services::fe3::FE3Handler;
use crate::utilities::helpers::{create_dcat_uri, endpoint_to_search_url};
use log::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Progress reporting
// ---------------------------------------------------------------------------

/// A real-time progress update emitted during long-running operations
/// (`query_dcat`, `get_packages_for_product`, `search_dcat`).
///
/// `stage` is a stable identifier (e.g. `"fe3.syncUpdates"`); `message`
/// is human-readable detail. `current`/`total` are populated for stages
/// that report `"N of M"` counters.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub stage: &'static str,
    pub message: String,
    pub current: Option<u32>,
    pub total: Option<u32>,
}

/// Callback type for [`DisplayCatalogHandler::set_progress_callback`].
///
/// On native targets the callback must be `Send + Sync` so the handler
/// stays multi-thread-friendly. On WASM the bound is relaxed to plain
/// `'static` because `js_sys::Function` is `!Send`.
#[cfg(not(target_arch = "wasm32"))]
pub type ProgressCallback = Box<dyn Fn(ProgressEvent) + Send + Sync + 'static>;
#[cfg(target_arch = "wasm32")]
pub type ProgressCallback = Box<dyn Fn(ProgressEvent) + 'static>;

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
    progress: Option<ProgressCallback>,
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
            progress: None,
        }
    }

    /// Install a callback that fires at every phase boundary inside
    /// `query_dcat`, `get_packages_for_product`, and `search_dcat`. Replaces
    /// any previously-installed callback. Pass [`None`] via
    /// [`Self::clear_progress_callback`] to detach.
    pub fn set_progress_callback(&mut self, cb: ProgressCallback) {
        self.progress = Some(cb);
    }

    /// Detach the progress callback (if any).
    pub fn clear_progress_callback(&mut self) {
        self.progress = None;
    }

    pub(crate) fn emit(&self, stage: &'static str, message: impl Into<String>) {
        if let Some(cb) = &self.progress {
            cb(ProgressEvent {
                stage,
                message: message.into(),
                current: None,
                total: None,
            });
        }
    }

    pub(crate) fn emit_counter(
        &self,
        stage: &'static str,
        message: impl Into<String>,
        current: u32,
        total: u32,
    ) {
        if let Some(cb) = &self.progress {
            cb(ProgressEvent {
                stage,
                message: message.into(),
                current: Some(current),
                total: Some(total),
            });
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
        self.emit("dcat.request", format!("GET id={id}"));

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
        self.emit("dcat.response", format!("HTTP {status}"));

        if status.is_success() {
            let body = response.text().await.map_err(StoreError::Http)?;
            debug!("DCat response body: {} bytes", body.len());
            self.emit("dcat.parse", format!("{} bytes", body.len()));
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
            self.emit("dcat.done", format!("\"{title}\""));
            self.product_listing = Some(model);
            self.result = Some(DisplayCatalogResult::Found);
            self.is_found = true;
            Ok(())
        } else if status == reqwest::StatusCode::NOT_FOUND {
            warn!("DCat: product not found (id={id})");
            self.emit("dcat.notFound", format!("id={id}"));
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
        self.emit("fe3.start", format!("WuCategoryId={wu_category_id}"));

        self.emit("fe3.getCookie", "POST GetCookie");
        let cookie = FE3Handler::get_cookie(&self.client).await?;

        self.emit("fe3.syncUpdates", format!("cookie {} bytes", cookie.len()));
        let xml =
            FE3Handler::sync_updates_with_cookie(&cookie, wu_category_id, msa_token, &self.client)
                .await?;

        self.emit("fe3.parseUpdateIds", format!("{} bytes XML", xml.len()));
        let (update_ids, revision_ids) = FE3Handler::process_update_ids(&xml)?;
        debug!("FE3: {} update ID(s) parsed", update_ids.len());
        self.emit_counter(
            "fe3.parseUpdateIds.done",
            "update IDs parsed",
            update_ids.len() as u32,
            update_ids.len() as u32,
        );

        self.emit("fe3.parsePackages", "parsing package instances");
        let mut instances = FE3Handler::get_package_instances(&xml).await?;
        debug!("FE3: {} package instance(s) found", instances.len());
        self.emit_counter(
            "fe3.parsePackages.done",
            "package instances parsed",
            instances.len() as u32,
            instances.len() as u32,
        );

        self.emit(
            "fe3.resolveUrls",
            format!("resolving {} URLs", update_ids.len()),
        );
        let urls =
            FE3Handler::get_file_urls(&update_ids, &revision_ids, msa_token, &self.client).await?;
        debug!("FE3: {} download URL(s) resolved", urls.len());
        self.emit_counter(
            "fe3.resolveUrls.done",
            "URLs resolved",
            urls.len() as u32,
            urls.len() as u32,
        );

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

        info!("DCat size map ({} entries):", dcat_size_map.len());
        for (name, size) in &dcat_size_map {
            info!("  DCat package: {name} = {size} bytes");
        }
        info!("FE3 package monikers ({} entries):", instances.len());
        for inst in &instances {
            info!("  FE3 moniker: {}", inst.package_moniker);
        }

        for (i, instance) in instances.iter_mut().enumerate() {
            instance.update_id = update_ids.get(i).cloned().unwrap_or_default();
            if let Some((url, fe3_size)) = urls.get(i) {
                instance.package_uri = Some(url.clone());
                instance.file_size = *fe3_size;
            }
            // Fall back to DCat size if FE3 didn't provide one
            if instance.file_size.is_none() {
                instance.file_size = dcat_size_map
                    .get(instance.package_moniker.as_str())
                    .copied();
            }
            info!(
                "  package[{i}]: moniker={} fe3_size={:?} dcat_size={:?}",
                instance.package_moniker,
                urls.get(i).and_then(|(_, s)| *s),
                dcat_size_map
                    .get(instance.package_moniker.as_str())
                    .copied(),
            );
        }

        info!("Resolved {} package(s)", instances.len());
        self.emit(
            "fe3.done",
            format!("{} package(s) resolved", instances.len()),
        );
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
        self.emit("search.request", format!("\"{query}\""));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("DCat search response: HTTP {status}");
        self.emit("search.response", format!("HTTP {status}"));

        if status.is_success() {
            let body = response.text().await.map_err(StoreError::Http)?;
            debug!("DCat search response body: {} bytes", body.len());
            self.emit("search.parse", format!("{} bytes", body.len()));
            self.result = Some(DisplayCatalogResult::Found);
            let result: DCatSearch = serde_json::from_str(&body).map_err(|e| {
                error!("DCat search JSON parse error: {e}");
                log_json_context(&body, e.column());
                StoreError::Json(e)
            })?;
            let count = result.total_result_count.unwrap_or(0);
            info!("DCat search: {count} result(s) for \"{query}\"");
            self.emit("search.done", format!("{count} result(s)"));
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn progress_event_serializes_camel_case() {
        let e = ProgressEvent {
            stage: "fe3.syncUpdates",
            message: "cookie 256 bytes".into(),
            current: None,
            total: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(
            json.contains("\"stage\":\"fe3.syncUpdates\""),
            "got: {json}"
        );
        assert!(
            json.contains("\"message\":\"cookie 256 bytes\""),
            "got: {json}",
        );
        assert!(json.contains("\"current\":null"), "got: {json}");
        assert!(json.contains("\"total\":null"), "got: {json}");
    }

    #[test]
    fn progress_event_counter_serializes() {
        let e = ProgressEvent {
            stage: "fe3.resolveUrls.done",
            message: "URLs resolved".into(),
            current: Some(7),
            total: Some(7),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"current\":7"), "got: {json}");
        assert!(json.contains("\"total\":7"), "got: {json}");
    }

    fn capturing_handler() -> (DisplayCatalogHandler, Arc<Mutex<Vec<ProgressEvent>>>) {
        let log = Arc::new(Mutex::new(Vec::<ProgressEvent>::new()));
        let log_cb = log.clone();
        let mut h = DisplayCatalogHandler::production();
        h.set_progress_callback(Box::new(move |e| log_cb.lock().unwrap().push(e)));
        (h, log)
    }

    #[test]
    fn emit_invokes_callback() {
        let (h, log) = capturing_handler();
        h.emit("dcat.request", "GET id=foo");
        let events = log.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stage, "dcat.request");
        assert_eq!(events[0].message, "GET id=foo");
        assert_eq!(events[0].current, None);
        assert_eq!(events[0].total, None);
    }

    #[test]
    fn emit_counter_carries_progress_numbers() {
        let (h, log) = capturing_handler();
        h.emit_counter("fe3.resolveUrls.done", "URLs resolved", 5, 12);
        let events = log.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stage, "fe3.resolveUrls.done");
        assert_eq!(events[0].current, Some(5));
        assert_eq!(events[0].total, Some(12));
    }

    #[test]
    fn multiple_emits_preserve_order() {
        let (h, log) = capturing_handler();
        h.emit("dcat.request", "step 1");
        h.emit("dcat.response", "step 2");
        h.emit_counter("dcat.parse", "step 3", 1, 1);
        h.emit("dcat.done", "step 4");
        let stages: Vec<&str> = log.lock().unwrap().iter().map(|e| e.stage).collect();
        assert_eq!(
            stages,
            vec!["dcat.request", "dcat.response", "dcat.parse", "dcat.done"],
        );
    }

    #[test]
    fn no_callback_means_no_panic() {
        // Without a callback installed, emit must be a no-op.
        let h = DisplayCatalogHandler::production();
        h.emit("x", "y");
        h.emit_counter("x", "y", 1, 1);
    }

    #[test]
    fn clear_callback_stops_delivery() {
        let (mut h, log) = capturing_handler();
        h.emit("first", "");
        h.clear_progress_callback();
        h.emit("second", "");
        let events = log.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].stage, "first");
    }

    #[test]
    fn set_callback_replaces_previous() {
        let log_a: Arc<Mutex<Vec<ProgressEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let log_b: Arc<Mutex<Vec<ProgressEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let mut h = DisplayCatalogHandler::production();

        let a = log_a.clone();
        h.set_progress_callback(Box::new(move |e| a.lock().unwrap().push(e)));
        h.emit("first", "");

        let b = log_b.clone();
        h.set_progress_callback(Box::new(move |e| b.lock().unwrap().push(e)));
        h.emit("second", "");

        assert_eq!(log_a.lock().unwrap().len(), 1);
        assert_eq!(log_a.lock().unwrap()[0].stage, "first");
        assert_eq!(log_b.lock().unwrap().len(), 1);
        assert_eq!(log_b.lock().unwrap()[0].stage, "second");
    }
}
