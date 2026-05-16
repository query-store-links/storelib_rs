use std::future::Future;
use std::time::Duration;

use futures_util::future::{select, Either};

use crate::cancellation::CancellationToken;
use crate::error::StoreError;
use crate::models::catalog::DisplayCatalogModel;
use crate::models::enums::{DCatEndpoint, DeviceFamily, DisplayCatalogResult, IdentifierType};
use crate::models::fe3::PackageInstance;
use crate::models::locale::Locale;
use crate::models::search::DCatSearch;
use crate::services::fe3::FE3Handler;
use crate::utilities::helpers::{create_dcat_uri, endpoint_to_search_url};
use crate::utilities::sleep::sleep;
use log::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// HTTP client config
// ---------------------------------------------------------------------------

/// Tunable HTTP-client parameters shared by [`DisplayCatalogHandler`] and the
/// FE3 SOAP calls it triggers.
///
/// Construct with [`Default`] for the recommended values, or build one
/// explicitly:
///
/// ```
/// use std::time::Duration;
/// use storelib_rs::ClientConfig;
///
/// let cfg = ClientConfig {
///     timeout: Duration::from_secs(60),
///     max_retries: 5,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Per-request timeout. The whole request (incl. body read) must finish
    /// inside this window or it fails with [`StoreError::TimedOut`].
    /// Default: 30 s.
    pub timeout: Duration,
    /// Maximum retry attempts on transient failures
    /// (connection errors, timeouts, statuses in [`Self::retry_on_status`]).
    /// `0` disables retries. Default: `3`.
    pub max_retries: u32,
    /// Initial backoff between attempts; doubled each retry up to
    /// [`Self::max_backoff`]. Default: 500 ms.
    pub initial_backoff: Duration,
    /// Upper cap on per-attempt backoff. Default: 5 s.
    pub max_backoff: Duration,
    /// HTTP status codes that trigger a retry. Default:
    /// `[408, 429, 502, 503, 504]`.
    pub retry_on_status: Vec<u16>,
    /// User-Agent header. Default: `"StoreLib"`.
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(5),
            retry_on_status: vec![408, 429, 502, 503, 504],
            user_agent: "StoreLib".into(),
        }
    }
}

impl ClientConfig {
    pub(crate) fn backoff_for(&self, attempt: u32) -> Duration {
        // attempt 0 → initial; attempt 1 → 2× initial; attempt N → 2^N × initial, capped.
        let factor: u64 = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
        let nanos = self
            .initial_backoff
            .as_nanos()
            .saturating_mul(factor as u128);
        let dur = Duration::from_nanos(nanos.min(u64::MAX as u128) as u64);
        std::cmp::min(dur, self.max_backoff)
    }
}

/// Race `op` against an optional cancellation token. Returns
/// [`StoreError::Cancelled`] if the token is cancelled before `op` completes.
pub(crate) async fn race_cancel<F, T>(
    op: F,
    cancel: Option<&CancellationToken>,
) -> Result<T, StoreError>
where
    F: Future<Output = Result<T, StoreError>>,
{
    let op = std::pin::pin!(op);
    match cancel {
        Some(tok) => {
            let cancel_fut = tok.cancelled();
            let cancel_fut = std::pin::pin!(cancel_fut);
            match select(op, cancel_fut).await {
                Either::Left((res, _)) => res,
                Either::Right(_) => Err(StoreError::Cancelled),
            }
        }
        None => op.await,
    }
}

/// Send a reqwest request with exponential-backoff retries, optional
/// cancellation, and progress events on each retry boundary.
///
/// `make_req` is called once per attempt — it must rebuild the
/// [`reqwest::RequestBuilder`] each time because `RequestBuilder` isn't
/// cloneable. `on_progress` receives `("retry.wait", …)` before each backoff
/// sleep and `("retry.attempt", …)` after the sleep, just before the next
/// send.
///
/// Returns the [`reqwest::Response`] on success or on the final attempt
/// (caller inspects status). Returns [`StoreError::Cancelled`] if the token
/// fires at any point (including mid-backoff). Returns [`StoreError::TimedOut`]
/// or [`StoreError::Http`] when retries are exhausted on a transport error.
pub(crate) async fn send_with_retry<F, P>(
    make_req: F,
    cfg: &ClientConfig,
    cancel: Option<&CancellationToken>,
    on_progress: P,
) -> Result<reqwest::Response, StoreError>
where
    F: Fn() -> reqwest::RequestBuilder,
    P: Fn(&'static str, String),
{
    let total_attempts = cfg.max_retries.saturating_add(1);
    let mut last_err: Option<StoreError> = None;

    for attempt in 0..total_attempts {
        if attempt > 0 {
            let backoff = cfg.backoff_for(attempt - 1);
            on_progress(
                "retry.wait",
                format!(
                    "waiting {}ms before attempt {}/{}",
                    backoff.as_millis(),
                    attempt + 1,
                    total_attempts,
                ),
            );
            // Cancellation-aware sleep. Returns Cancelled if the token fires.
            race_cancel(
                async {
                    sleep(backoff).await;
                    Ok::<(), StoreError>(())
                },
                cancel,
            )
            .await?;
            on_progress(
                "retry.attempt",
                format!("attempt {}/{}", attempt + 1, total_attempts),
            );
        }

        let send_fut = make_req().send();
        let result = race_cancel(
            async {
                send_fut.await.map_err(|e| {
                    if e.is_timeout() {
                        StoreError::TimedOut
                    } else {
                        StoreError::Http(e)
                    }
                })
            },
            cancel,
        )
        .await;

        match result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let retryable = cfg.retry_on_status.contains(&status);
                let attempts_left = attempt + 1 < total_attempts;
                if retryable && attempts_left {
                    // Drop the response and try again.
                    continue;
                }
                return Ok(resp);
            }
            Err(StoreError::Cancelled) => return Err(StoreError::Cancelled),
            Err(e) => {
                if attempt + 1 < total_attempts {
                    last_err = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| StoreError::Other("retries exhausted".into())))
}

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
    pub(crate) config: ClientConfig,
    progress: Option<ProgressCallback>,
}

impl DisplayCatalogHandler {
    /// Create a new handler pointing at the given endpoint with the given
    /// locale, using the default [`ClientConfig`].
    pub fn new(endpoint: DCatEndpoint, locale: Locale) -> Self {
        Self::with_config(endpoint, locale, ClientConfig::default())
    }

    /// Create a new handler with explicit [`ClientConfig`] (timeout, retry
    /// policy, user agent).
    pub fn with_config(endpoint: DCatEndpoint, locale: Locale, config: ClientConfig) -> Self {
        let client = Self::build_client(&config);
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
            config,
            progress: None,
        }
    }

    /// Returns a reference to the handler's [`ClientConfig`].
    pub fn config(&self) -> &ClientConfig {
        &self.config
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

    fn build_client(config: &ClientConfig) -> reqwest::Client {
        #[cfg(not(target_arch = "wasm32"))]
        let builder = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.timeout);
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder().user_agent(&config.user_agent);
        builder.build().unwrap_or_default()
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
        self.query_dcat_with_cancel(id, id_type, auth_token, None)
            .await
    }

    /// Same as [`Self::query_dcat`] but races the request against an optional
    /// [`CancellationToken`]. When the token is cancelled before the request
    /// completes, dropping the in-flight future cancels the underlying HTTP
    /// request and this method returns [`StoreError::Cancelled`].
    pub async fn query_dcat_with_cancel(
        &mut self,
        id: &str,
        id_type: IdentifierType,
        auth_token: Option<&str>,
        cancel: Option<&CancellationToken>,
    ) -> Result<(), StoreError> {
        race_cancel(self.query_dcat_inner(id, id_type, auth_token), cancel).await
    }

    async fn query_dcat_inner(
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

        let auth = auth_token.filter(|t| !t.is_empty());
        let response = send_with_retry(
            || {
                let mut r = self.client.get(&url);
                if let Some(token) = auth {
                    r = r.header("Authentication", token);
                }
                r
            },
            &self.config,
            None,
            |stage, msg| self.emit(stage, msg),
        )
        .await
        .map_err(|e| {
            if matches!(e, StoreError::TimedOut) {
                warn!("DCat query timed out for id={id}");
            }
            e
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
        self.get_packages_for_product_with_cancel(msa_token, None)
            .await
    }

    /// Same as [`Self::get_packages_for_product`] but races the FE3 SOAP call
    /// sequence against an optional [`CancellationToken`].
    pub async fn get_packages_for_product_with_cancel(
        &self,
        msa_token: Option<&str>,
        cancel: Option<&CancellationToken>,
    ) -> Result<Vec<PackageInstance>, StoreError> {
        race_cancel(self.get_packages_for_product_inner(msa_token), cancel).await
    }

    async fn get_packages_for_product_inner(
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

    /// Same as [`Self::search_dcat`] but races against an optional
    /// [`CancellationToken`].
    pub async fn search_dcat_with_cancel(
        &mut self,
        query: &str,
        device_family: DeviceFamily,
        cancel: Option<&CancellationToken>,
    ) -> Result<DCatSearch, StoreError> {
        self.search_dcat_paged_with_cancel(query, device_family, 0, cancel)
            .await
    }

    /// Search DisplayCatalog for the given query string, skipping `skip_count`
    /// results (each page holds up to 100 results).
    pub async fn search_dcat_paged(
        &mut self,
        query: &str,
        device_family: DeviceFamily,
        skip_count: u32,
    ) -> Result<DCatSearch, StoreError> {
        self.search_dcat_paged_with_cancel(query, device_family, skip_count, None)
            .await
    }

    /// Same as [`Self::search_dcat_paged`] but races against an optional
    /// [`CancellationToken`].
    pub async fn search_dcat_paged_with_cancel(
        &mut self,
        query: &str,
        device_family: DeviceFamily,
        skip_count: u32,
        cancel: Option<&CancellationToken>,
    ) -> Result<DCatSearch, StoreError> {
        race_cancel(
            self.search_dcat_paged_inner(query, device_family, skip_count),
            cancel,
        )
        .await
    }

    async fn search_dcat_paged_inner(
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

        let response = send_with_retry(
            || self.client.get(&url),
            &self.config,
            None,
            |stage, msg| self.emit(stage, msg),
        )
        .await?;

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
    use std::time::Duration;

    // -- ClientConfig --------------------------------------------------------

    #[test]
    fn client_config_default_matches_docs() {
        let cfg = ClientConfig::default();
        assert_eq!(cfg.timeout, Duration::from_secs(30));
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.initial_backoff, Duration::from_millis(500));
        assert_eq!(cfg.max_backoff, Duration::from_secs(5));
        assert_eq!(cfg.retry_on_status, vec![408, 429, 502, 503, 504]);
        assert_eq!(cfg.user_agent, "StoreLib");
    }

    #[test]
    fn backoff_for_doubles_until_cap() {
        let cfg = ClientConfig {
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(800),
            ..Default::default()
        };
        assert_eq!(cfg.backoff_for(0), Duration::from_millis(100));
        assert_eq!(cfg.backoff_for(1), Duration::from_millis(200));
        assert_eq!(cfg.backoff_for(2), Duration::from_millis(400));
        // 800ms is the cap; attempt 3 would be 800ms (cap), attempt 4+ stays at cap.
        assert_eq!(cfg.backoff_for(3), Duration::from_millis(800));
        assert_eq!(cfg.backoff_for(4), Duration::from_millis(800));
        assert_eq!(cfg.backoff_for(50), Duration::from_millis(800));
    }

    #[test]
    fn backoff_for_handles_huge_attempt_without_overflow() {
        let cfg = ClientConfig::default();
        // Should not panic on overflow.
        let _ = cfg.backoff_for(64);
        let _ = cfg.backoff_for(u32::MAX);
    }

    #[test]
    fn with_config_keeps_overrides() {
        let cfg = ClientConfig {
            user_agent: "TestUA/1.0".into(),
            max_retries: 7,
            ..Default::default()
        };
        let h = DisplayCatalogHandler::with_config(
            DCatEndpoint::Production,
            Locale::production(),
            cfg.clone(),
        );
        assert_eq!(h.config().max_retries, 7);
        assert_eq!(h.config().user_agent, "TestUA/1.0");
    }

    // -- race_cancel ---------------------------------------------------------

    #[tokio::test]
    async fn race_cancel_returns_op_result_when_not_cancelled() {
        let op = async { Ok::<_, StoreError>(42_u32) };
        let result = race_cancel(op, None).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn race_cancel_returns_op_result_when_token_uncancelled() {
        let token = CancellationToken::new();
        let op = async { Ok::<_, StoreError>("done") };
        let result = race_cancel(op, Some(&token)).await.unwrap();
        assert_eq!(result, "done");
    }

    #[tokio::test]
    async fn race_cancel_returns_cancelled_when_token_fires_first() {
        let token = CancellationToken::new();
        let canceller = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            canceller.cancel();
        });
        // Op never completes on its own.
        let op = async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok::<_, StoreError>(())
        };
        let err = race_cancel(op, Some(&token)).await.unwrap_err();
        assert!(matches!(err, StoreError::Cancelled));
    }

    #[tokio::test]
    async fn race_cancel_returns_cancelled_when_token_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        // Op would succeed if allowed to run, but token is already cancelled
        // so race_cancel resolves immediately on the Right branch.
        let op = async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<_, StoreError>(())
        };
        let err = race_cancel(op, Some(&token)).await.unwrap_err();
        assert!(matches!(err, StoreError::Cancelled));
    }

    #[tokio::test]
    async fn race_cancel_propagates_op_error() {
        let token = CancellationToken::new();
        let op = async { Err::<u32, _>(StoreError::NotFound) };
        let err = race_cancel(op, Some(&token)).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }

    // -- send_with_retry (wiremock-backed) ----------------------------------

    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fast_retry_cfg() -> ClientConfig {
        // Tiny backoffs so tests don't drag.
        ClientConfig {
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(2),
            max_retries: 3,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn send_with_retry_succeeds_on_first_attempt() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1) // exactly one request
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let cfg = fast_retry_cfg();
        let resp = send_with_retry(|| client.get(server.uri()), &cfg, None, |_, _| {})
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn send_with_retry_retries_on_503_then_succeeds() {
        let server = MockServer::start().await;
        // First two requests return 503; the third returns 200.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let cfg = fast_retry_cfg();
        let resp = send_with_retry(|| client.get(server.uri()), &cfg, None, |_, _| {})
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn send_with_retry_gives_up_after_max_retries() {
        let server = MockServer::start().await;
        // Every request returns 503; expect max_retries + 1 attempts total.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .expect(4) // 1 initial + 3 retries
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let cfg = fast_retry_cfg();
        let resp = send_with_retry(|| client.get(server.uri()), &cfg, None, |_, _| {})
            .await
            .unwrap();
        // After exhausting retries we return the last response unchanged.
        assert_eq!(resp.status(), 503);
    }

    #[tokio::test]
    async fn send_with_retry_does_not_retry_on_4xx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1) // 404 is not in retry_on_status by default
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let cfg = fast_retry_cfg();
        let resp = send_with_retry(|| client.get(server.uri()), &cfg, None, |_, _| {})
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn send_with_retry_emits_progress_per_attempt() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let log_cb = log.clone();
        let client = reqwest::Client::new();
        let cfg = fast_retry_cfg();
        let _ = send_with_retry(
            || client.get(server.uri()),
            &cfg,
            None,
            |stage, _msg| log_cb.lock().unwrap().push(stage.to_string()),
        )
        .await
        .unwrap();

        let stages = log.lock().unwrap().clone();
        // Two retries → two (retry.wait, retry.attempt) pairs.
        assert_eq!(
            stages,
            vec![
                "retry.wait".to_string(),
                "retry.attempt".to_string(),
                "retry.wait".to_string(),
                "retry.attempt".to_string(),
            ],
        );
    }

    #[tokio::test]
    async fn send_with_retry_cancel_during_backoff_returns_cancelled_fast() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            // We expect at most 1 hit: first 503, then we cancel during backoff
            // so the retry never fires.
            .expect(1)
            .mount(&server)
            .await;

        let cfg = ClientConfig {
            initial_backoff: Duration::from_secs(60),
            max_backoff: Duration::from_secs(60),
            max_retries: 3,
            ..Default::default()
        };

        let token = CancellationToken::new();
        let canceller = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            canceller.cancel();
        });

        let client = reqwest::Client::new();
        let started = std::time::Instant::now();
        let err = send_with_retry(|| client.get(server.uri()), &cfg, Some(&token), |_, _| {})
            .await
            .unwrap_err();
        let elapsed = started.elapsed();

        assert!(matches!(err, StoreError::Cancelled));
        // Should resolve well under the 60s backoff once the token fires.
        assert!(
            elapsed < Duration::from_secs(2),
            "cancel-during-backoff took too long: {:?}",
            elapsed,
        );
    }

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
