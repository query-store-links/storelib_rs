//! JS bindings produced by `wasm-bindgen`.
//!
//! Exposes the full library surface — handlers, helpers, and value types — to
//! JavaScript consumers. Enum-typed parameters are accepted as camelCase
//! strings, Market/Lang as their canonical wire form (e.g. `"US"`, `"en-US"`),
//! and complex values cross the FFI boundary as plain JS objects via
//! `serde-wasm-bindgen`.

use std::str::FromStr;

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::error::StoreError;
use crate::models::enums::{DCatEndpoint, DeviceFamily, IdentifierType};
use crate::models::locale::{Lang, Locale, Market};
use crate::services::display_catalog::DisplayCatalogHandler;
use crate::services::fe3::FE3Handler;
use crate::utilities::helpers as h;

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn js_err<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_enum<T: serde::de::DeserializeOwned>(label: &str, raw: &str) -> Result<T, JsError> {
    serde_json::from_value::<T>(serde_json::Value::String(raw.to_owned()))
        .map_err(|_| JsError::new(&format!("invalid {label}: {raw}")))
}

fn parse_endpoint(s: &str) -> Result<DCatEndpoint, JsError> {
    parse_enum("endpoint", s)
}

fn parse_id_type(s: &str) -> Result<IdentifierType, JsError> {
    parse_enum("identifierType", s)
}

fn parse_device_family(s: &str) -> Result<DeviceFamily, JsError> {
    parse_enum("deviceFamily", s)
}

fn parse_market(s: &str) -> Result<Market, JsError> {
    Market::from_str(s).map_err(|e| JsError::new(&e))
}

fn parse_lang(s: &str) -> Result<Lang, JsError> {
    Lang::from_str(s).map_err(|e| JsError::new(&e))
}

fn store_err(e: StoreError) -> JsError {
    JsError::new(&e.to_string())
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

/// Installs a panic hook that forwards Rust panics to `console.error` with a
/// readable stack trace. Called automatically by wasm-bindgen on module load.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Map an `AppxMetadata` package-type string (e.g. `"AppX"`) to the canonical
/// camelCase enum value (`"appX"`, `"uap"`, `"xap"`, or `"unknown"`).
#[wasm_bindgen(js_name = stringToPackageType)]
pub fn string_to_package_type_js(raw: &str) -> Result<JsValue, JsError> {
    to_value(&h::string_to_package_type(raw)).map_err(js_err)
}

/// Returns the base URL for a DisplayCatalog product endpoint.
#[wasm_bindgen(js_name = endpointToBaseUrl)]
pub fn endpoint_to_base_url_js(endpoint: &str) -> Result<String, JsError> {
    let e = parse_endpoint(endpoint)?;
    Ok(h::endpoint_to_base_url(&e).to_string())
}

/// Returns the base URL for a DisplayCatalog autosuggest search endpoint.
#[wasm_bindgen(js_name = endpointToSearchUrl)]
pub fn endpoint_to_search_url_js(endpoint: &str) -> Result<String, JsError> {
    let e = parse_endpoint(endpoint)?;
    Ok(h::endpoint_to_search_url(&e).to_string())
}

/// Build a full DisplayCatalog request URL from its components.
#[wasm_bindgen(js_name = createDcatUri)]
pub fn create_dcat_uri_js(
    endpoint: &str,
    id: &str,
    id_type: &str,
    locale: &LocaleJs,
) -> Result<String, JsError> {
    let e = parse_endpoint(endpoint)?;
    let t = parse_id_type(id_type)?;
    Ok(h::create_dcat_uri(&e, id, &t, &locale.inner))
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Combined locale used when forming DisplayCatalog request URLs.
#[wasm_bindgen(js_name = Locale)]
pub struct LocaleJs {
    inner: Locale,
}

#[wasm_bindgen(js_class = Locale)]
impl LocaleJs {
    /// Create a new locale. `market` is a two-letter market code (e.g.
    /// `"US"`); `language` is a BCP-47 tag (e.g. `"en-US"`, `"zh-Hant"`).
    /// When `includeNeutral` is true, the neutral English language is appended
    /// to the language list.
    #[wasm_bindgen(constructor)]
    pub fn new(market: &str, language: &str, include_neutral: bool) -> Result<LocaleJs, JsError> {
        Ok(LocaleJs {
            inner: Locale::new(
                parse_market(market)?,
                parse_lang(language)?,
                include_neutral,
            ),
        })
    }

    /// Default production locale: `US / en-US`, neutral included.
    #[wasm_bindgen(js_name = production)]
    pub fn production() -> LocaleJs {
        LocaleJs {
            inner: Locale::production(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn market(&self) -> String {
        self.inner.market.as_str().to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn language(&self) -> String {
        self.inner.language.as_str().to_owned()
    }

    #[wasm_bindgen(getter, js_name = includeNeutral)]
    pub fn include_neutral(&self) -> bool {
        self.inner.include_neutral
    }

    /// Returns the trailing query-string fragment appended to DCat URLs
    /// (e.g. `market=US&languages=en-US,en&catalogsource=apps`).
    #[wasm_bindgen(js_name = dcatTrail)]
    pub fn dcat_trail(&self) -> String {
        self.inner.dcat_trail()
    }

    /// Returns the locale as a plain object: `{market, language, includeNeutral}`.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner).map_err(js_err)
    }
}

// ---------------------------------------------------------------------------
// DisplayCatalogHandler
// ---------------------------------------------------------------------------

/// High-level client for the Microsoft DisplayCatalog API.
#[wasm_bindgen(js_name = DisplayCatalogHandler)]
pub struct DisplayCatalogHandlerJs {
    inner: DisplayCatalogHandler,
}

#[wasm_bindgen(js_class = DisplayCatalogHandler)]
impl DisplayCatalogHandlerJs {
    /// Create a handler pointing at `endpoint` (one of `"production"`,
    /// `"int"`, `"xbox"`, `"xboxInt"`, `"dev"`, `"oneP"`, `"onePInt"`) with
    /// the given `Locale`.
    #[wasm_bindgen(constructor)]
    pub fn new(endpoint: &str, locale: &LocaleJs) -> Result<DisplayCatalogHandlerJs, JsError> {
        let e = parse_endpoint(endpoint)?;
        Ok(DisplayCatalogHandlerJs {
            inner: DisplayCatalogHandler::new(e, locale.inner.clone()),
        })
    }

    /// Convenience constructor for the production endpoint with the default
    /// US/en locale.
    #[wasm_bindgen(js_name = production)]
    pub fn production() -> DisplayCatalogHandlerJs {
        DisplayCatalogHandlerJs {
            inner: DisplayCatalogHandler::production(),
        }
    }

    /// Query DisplayCatalog for a product by `id` and `idType`. Resolves to the
    /// full product listing on success. An optional `authToken` may be provided
    /// for flighted/sandbox queries.
    #[wasm_bindgen(js_name = queryDcat)]
    pub async fn query_dcat(
        &mut self,
        id: String,
        id_type: String,
        auth_token: Option<String>,
    ) -> Result<JsValue, JsError> {
        let t = parse_id_type(&id_type)?;
        self.inner
            .query_dcat(&id, t, auth_token.as_deref())
            .await
            .map_err(store_err)?;
        to_value(&self.inner.product_listing).map_err(js_err)
    }

    /// Resolve the direct download URLs for the currently-loaded product.
    /// Requires `queryDcat` to have been called successfully first.
    #[wasm_bindgen(js_name = getPackagesForProduct)]
    pub async fn get_packages_for_product(
        &self,
        msa_token: Option<String>,
    ) -> Result<JsValue, JsError> {
        let packages = self
            .inner
            .get_packages_for_product(msa_token.as_deref())
            .await
            .map_err(store_err)?;
        to_value(&packages).map_err(js_err)
    }

    /// Search DisplayCatalog for the given query string.
    #[wasm_bindgen(js_name = searchDcat)]
    pub async fn search_dcat(
        &mut self,
        query: String,
        device_family: String,
    ) -> Result<JsValue, JsError> {
        let df = parse_device_family(&device_family)?;
        let result = self
            .inner
            .search_dcat(&query, df)
            .await
            .map_err(store_err)?;
        to_value(&result).map_err(js_err)
    }

    /// Same as `searchDcat` but skips the first `skipCount` results (pages of
    /// up to 100 items each).
    #[wasm_bindgen(js_name = searchDcatPaged)]
    pub async fn search_dcat_paged(
        &mut self,
        query: String,
        device_family: String,
        skip_count: u32,
    ) -> Result<JsValue, JsError> {
        let df = parse_device_family(&device_family)?;
        let result = self
            .inner
            .search_dcat_paged(&query, df, skip_count)
            .await
            .map_err(store_err)?;
        to_value(&result).map_err(js_err)
    }

    // -- state accessors ---------------------------------------------------

    #[wasm_bindgen(getter, js_name = isFound)]
    pub fn is_found(&self) -> bool {
        self.inner.is_found
    }

    #[wasm_bindgen(getter, js_name = productListing)]
    pub fn product_listing(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.product_listing).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = searchResult)]
    pub fn search_result(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.search_result).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedEndpoint)]
    pub fn selected_endpoint(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.selected_endpoint).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedLocale)]
    pub fn selected_locale(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.selected_locale).map_err(js_err)
    }

    #[wasm_bindgen(getter)]
    pub fn result(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.result).map_err(js_err)
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> Option<String> {
        self.inner.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.inner.error.clone()
    }
}

// ---------------------------------------------------------------------------
// FE3Handler
// ---------------------------------------------------------------------------

/// Low-level wrapper around the FE3 (Windows Update) SOAP endpoints used to
/// resolve direct package download URLs.
#[wasm_bindgen(js_name = Fe3Handler)]
pub struct Fe3HandlerJs {
    client: reqwest::Client,
}

#[wasm_bindgen(js_class = Fe3Handler)]
impl Fe3HandlerJs {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Fe3HandlerJs {
        Fe3HandlerJs {
            client: reqwest::Client::builder()
                .user_agent("StoreLib")
                .build()
                .unwrap_or_default(),
        }
    }

    /// POST `GetCookie` and return the `EncryptedData` value from the response.
    #[wasm_bindgen(js_name = getCookie)]
    pub async fn get_cookie(&self) -> Result<String, JsError> {
        FE3Handler::get_cookie(&self.client)
            .await
            .map_err(store_err)
    }

    /// POST `SyncUpdates` for the given `wuCategoryId`. Returns the HTML-decoded
    /// SOAP response body.
    #[wasm_bindgen(js_name = syncUpdates)]
    pub async fn sync_updates(
        &self,
        wu_category_id: String,
        msa_token: Option<String>,
    ) -> Result<String, JsError> {
        FE3Handler::sync_updates(&wu_category_id, msa_token.as_deref(), &self.client)
            .await
            .map_err(store_err)
    }

    /// Parse the raw `SyncUpdates` XML and extract update + revision IDs.
    /// Returns `{updateIds: string[], revisionIds: string[]}`.
    #[wasm_bindgen(js_name = processUpdateIds)]
    pub fn process_update_ids(xml: &str) -> Result<JsValue, JsError> {
        let (update_ids, revision_ids) = FE3Handler::process_update_ids(xml).map_err(store_err)?;
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Ids {
            update_ids: Vec<String>,
            revision_ids: Vec<String>,
        }
        to_value(&Ids {
            update_ids,
            revision_ids,
        })
        .map_err(js_err)
    }

    /// Parse the raw `SyncUpdates` XML into typed `PackageInstance` records.
    #[wasm_bindgen(js_name = getPackageInstances)]
    pub async fn get_package_instances(xml: String) -> Result<JsValue, JsError> {
        let instances = FE3Handler::get_package_instances(&xml)
            .await
            .map_err(store_err)?;
        to_value(&instances).map_err(js_err)
    }

    /// Resolve direct download URLs for the given update + revision IDs.
    /// Returns `Array<{url: string, size: number | null}>`.
    #[wasm_bindgen(js_name = getFileUrls)]
    pub async fn get_file_urls(
        &self,
        update_ids: JsValue,
        revision_ids: JsValue,
        msa_token: Option<String>,
    ) -> Result<JsValue, JsError> {
        let update_ids: Vec<String> = from_value(update_ids).map_err(js_err)?;
        let revision_ids: Vec<String> = from_value(revision_ids).map_err(js_err)?;
        let pairs = FE3Handler::get_file_urls(
            &update_ids,
            &revision_ids,
            msa_token.as_deref(),
            &self.client,
        )
        .await
        .map_err(store_err)?;

        #[derive(serde::Serialize)]
        struct UrlEntry {
            url: String,
            size: Option<i64>,
        }
        let mapped: Vec<UrlEntry> = pairs
            .into_iter()
            .map(|(url, size)| UrlEntry { url, size })
            .collect();
        to_value(&mapped).map_err(js_err)
    }
}

impl Default for Fe3HandlerJs {
    fn default() -> Self {
        Self::new()
    }
}
