//! JS bindings produced by `wasm-bindgen`.
//!
//! Exposes the full library surface — handlers, helpers, and value types — to
//! JavaScript consumers. Enum-typed parameters are accepted as camelCase
//! strings, Market/Lang as their canonical wire form (e.g. `"US"`, `"en-US"`),
//! and complex values cross the FFI boundary as plain JS objects via
//! `serde-wasm-bindgen`.

use std::str::FromStr;

use js_sys::{Function, Reflect};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::cancellation::CancellationToken;
use crate::error::StoreError;
use crate::models::enums::{DCatEndpoint, DeviceFamily, IdentifierType};
use crate::models::locale::{Lang, LanguageTag, Locale, Market};
use crate::services::display_catalog::{DisplayCatalogHandler, ProgressEvent};
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
    IdentifierType::parse_tolerant(s)
        .ok_or_else(|| JsError::new(&format!("unknown identifierType: {s}")))
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

/// Build a JS `Error` with an extra `kind` discriminant so JS callers can
/// branch on the failure mode without string-matching the message:
///
/// ```js
/// try { await handler.queryDcat(...) }
/// catch (e) {
///   if (e.kind === 'cancelled') return;
///   if (e.kind === 'notFound') showNotFound();
///   else showFatal(e.message);
/// }
/// ```
fn store_err(e: StoreError) -> JsValue {
    let kind = match &e {
        StoreError::Http(_) => "http",
        StoreError::Json(_) => "json",
        StoreError::Xml(_) => "xml",
        StoreError::NotFound => "notFound",
        StoreError::TimedOut => "timedOut",
        StoreError::Cancelled => "cancelled",
        StoreError::Other(_) => "other",
    };
    let err = js_sys::Error::new(&e.to_string());
    let _ = Reflect::set(&err, &JsValue::from_str("kind"), &JsValue::from_str(kind));
    err.into()
}

// ---------------------------------------------------------------------------
// AbortSignal → CancellationToken bridge
// ---------------------------------------------------------------------------

/// Adapter that mirrors a JS `AbortSignal` into a [`CancellationToken`].
///
/// The closure that fires on the `abort` event is owned by this adapter,
/// so the adapter must outlive the operation it cancels. The token can be
/// cheaply cloned and handed to service-level `_with_cancel` methods.
struct AbortAdapter {
    token: CancellationToken,
    _closure: Option<Closure<dyn FnMut(JsValue)>>,
}

impl AbortAdapter {
    fn from_signal(signal: &JsValue) -> Result<Self, JsError> {
        let token = CancellationToken::new();

        // If the signal is already aborted, propagate immediately.
        let already_aborted = Reflect::get(signal, &JsValue::from_str("aborted"))
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);
        if already_aborted {
            token.cancel();
            return Ok(AbortAdapter {
                token,
                _closure: None,
            });
        }

        let token_for_closure = token.clone();
        let closure = Closure::wrap(Box::new(move |_: JsValue| {
            token_for_closure.cancel();
        }) as Box<dyn FnMut(JsValue)>);

        let add: Function = Reflect::get(signal, &JsValue::from_str("addEventListener"))
            .map_err(|_| JsError::new("signal.addEventListener not callable"))?
            .dyn_into()
            .map_err(|_| JsError::new("signal.addEventListener not a function"))?;
        add.call2(
            signal,
            &JsValue::from_str("abort"),
            closure.as_ref().unchecked_ref(),
        )
        .map_err(|_| JsError::new("failed to attach abort listener"))?;

        Ok(AbortAdapter {
            token,
            _closure: Some(closure),
        })
    }
}

/// Adapt an optional JS `AbortSignal` into an optional [`AbortAdapter`]. The
/// returned adapter must be held by the caller for the duration of the
/// operation it cancels.
fn adapt_signal(signal: &Option<JsValue>) -> Result<Option<AbortAdapter>, JsValue> {
    match signal {
        Some(sig) if !sig.is_null() && !sig.is_undefined() => {
            let adapter = AbortAdapter::from_signal(sig)
                .map_err(|e| store_err(StoreError::Other(format!("invalid AbortSignal: {e:?}"))))?;
            Ok(Some(adapter))
        }
        _ => Ok(None),
    }
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
// TypeScript declarations
// ---------------------------------------------------------------------------
//
// These are emitted verbatim into the generated `.d.ts`, so JS consumers see
// real types (not `any`) for the values that cross the wasm-bindgen boundary.

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

/** Two-letter ISO 3166-1 alpha-2 market code (e.g. "US", "JP"). */
export type MarketCode = string;

/** Two-letter ISO 639-1 language code (e.g. "en", "zh"). */
export type LanguageCode = string;

/** Microsoft Store BCP-47 language tag (e.g. "en-US", "zh-Hant-TW"). */
export type LanguageTagCode = string;

/** A code/name pair returned by `listMarkets`, `listLanguages`,
 *  `listLanguageTags`, and the `parse*` validators. */
export interface CodeEntry {
    code: string;
    englishName: string;
}

/** Locale JSON shape (also accepted as input by `createDcatUri`). */
export interface LocaleJson {
    market: MarketCode;
    language: LanguageCode;
    includeNeutral: boolean;
}

/** Identifier type accepted by `queryDcat`. `parseIdentifierType` will
 *  canonicalise any common casing into this exact form. */
export type IdentifierTypeStr =
    | "productId"
    | "xboxTitleId"
    | "packageFamilyName"
    | "contentId"
    | "legacyWindowsPhoneProductId"
    | "legacyWindowsStoreProductId"
    | "legacyXboxProductId";

/** Microsoft Store DisplayCatalog endpoint identifier. */
export type DCatEndpointStr =
    | "production" | "int" | "xbox" | "xboxInt" | "dev" | "oneP" | "onePInt";

/** Device family used for search filtering. */
export type DeviceFamilyStr =
    | "desktop" | "mobile" | "xbox" | "serverCore" | "iotCore"
    | "holoLens" | "andromeda" | "universal" | "wcos";

/** Package format type returned by FE3. */
export type PackageTypeStr = "uap" | "xap" | "appX" | "unknown";

/** Image purpose / role inside `Product.localizedProperties[].images`. */
export type ImagePurposeStr =
    | "logo" | "tile" | "screenshot" | "boxArt" | "brandedKeyArt"
    | "poster" | "featurePromotionalSquareArt" | "imageGallery"
    | "superHeroArt" | "titledHeroArt";

/** Top-level Product kind. */
export type ProductKindStr =
    | "game" | "application" | "book" | "movie" | "physical" | "software";

/** Per-platform applicability slice inside an `ApplicabilityBlob`. */
export interface ContentTargetPlatform {
    "platform.maxVersionTested"?: number | null;
    "platform.minVersion"?: number | null;
    "platform.target"?: number | null;
}

/** FE3 applicability metadata. Keys are passed through verbatim from the
 *  SOAP payload, including the dot-separated names. */
export interface ApplicabilityBlob {
    "blob.version"?: number | null;
    "content.isMain"?: boolean | null;
    "content.packageId"?: string | null;
    "content.productId"?: string | null;
    "content.targetPlatforms"?: ContentTargetPlatform[] | null;
    "content.type"?: number | null;
}

/** A resolved package instance returned by `getPackagesForProduct`. */
export interface PackageInstance {
    packageMoniker: string;
    packageUri: string | null;
    packageType: PackageTypeStr;
    applicabilityBlob: ApplicabilityBlob | null;
    updateId: string;
    /** Download size in bytes. FE3-reported first, falling back to the
     *  DisplayCatalog `MaxDownloadSizeInBytes` field. `null` only for
     *  framework packages that DCat doesn't list a size for. */
    packageSize: number | null;
}

/** Stage identifier passed to `onProgress`. Stable across releases. */
export type ProgressStage =
    | "dcat.request" | "dcat.response" | "dcat.parse" | "dcat.done" | "dcat.notFound"
    | "fe3.start" | "fe3.getCookie" | "fe3.syncUpdates"
    | "fe3.parseUpdateIds" | "fe3.parseUpdateIds.done"
    | "fe3.parsePackages" | "fe3.parsePackages.done"
    | "fe3.resolveUrls" | "fe3.resolveUrls.done" | "fe3.done"
    | "search.request" | "search.response" | "search.parse" | "search.done"
    | "retry.wait" | "retry.attempt";

/** Progress event emitted during long-running operations. */
export interface ProgressEvent {
    stage: ProgressStage;
    message: string;
    /** "N of M" counter — set on `.done` stages, null otherwise. */
    current: number | null;
    total: number | null;
}

/** Progress callback installed via `DisplayCatalogHandler.onProgress`. */
export type OnProgress = (event: ProgressEvent) => void;

/** Error thrown by async handler methods. Branch on `.kind` to decide
 *  what to surface to the user. */
export interface StorelibError extends Error {
    kind:
        | "http" | "json" | "xml" | "notFound"
        | "timedOut" | "cancelled" | "other";
}

/** Top-level DisplayCatalog response. The MS Store schema is large and
 *  loosely typed; consumers typically access well-known fields directly.
 *  This stub keeps the basic shape navigable in TypeScript without
 *  pretending to fully model every field. */
export interface DisplayCatalogModel {
    product?: ProductLike | null;
    products?: ProductLike[] | null;
    bigIds?: string[] | null;
    hasMorePages?: boolean | null;
    totalResultCount?: number | null;
}

/** Loosely-typed Product shape. Camel-cased throughout. */
export interface ProductLike {
    lastModifiedDate?: string | null;
    localizedProperties?: ProductLocalizedPropertyLike[] | null;
    properties?: { [k: string]: any } | null;
    displaySkuAvailabilities?: { [k: string]: any }[] | null;
    productKind?: ProductKindStr | string | null;
    [key: string]: any;
}

export interface ProductLocalizedPropertyLike {
    productTitle?: string | null;
    productDescription?: string | null;
    publisherName?: string | null;
    language?: string | null;
    images?: { [k: string]: any }[] | null;
    [key: string]: any;
}

/** DCat search response. */
export interface DCatSearch {
    results?: SearchResult[] | null;
    totalResultCount?: number | null;
}

export interface SearchResult {
    productFamilyName?: string | null;
    products?: ProductLike[] | null;
}
"#;

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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CodeEntry {
    code: &'static str,
    english_name: &'static str,
}

/// Returns every ISO 3166-1 alpha-2 market code with its English name.
#[wasm_bindgen(js_name = listMarkets, unchecked_return_type = "CodeEntry[]")]
pub fn list_markets_js() -> Result<JsValue, JsError> {
    let entries: Vec<CodeEntry> = Market::all()
        .iter()
        .map(|m| CodeEntry {
            code: m.as_str(),
            english_name: m.english_name(),
        })
        .collect();
    to_value(&entries).map_err(js_err)
}

/// Returns every ISO 639-1 alpha-2 language code with its English name.
#[wasm_bindgen(js_name = listLanguages, unchecked_return_type = "CodeEntry[]")]
pub fn list_languages_js() -> Result<JsValue, JsError> {
    let entries: Vec<CodeEntry> = Lang::all()
        .iter()
        .map(|l| CodeEntry {
            code: l.as_str(),
            english_name: l.english_name(),
        })
        .collect();
    to_value(&entries).map_err(js_err)
}

/// Returns every Microsoft Store BCP-47 language tag (e.g. `en-US`,
/// `zh-Hant`, `sr-Cyrl-RS`) with its English name.
#[wasm_bindgen(js_name = listLanguageTags, unchecked_return_type = "CodeEntry[]")]
pub fn list_language_tags_js() -> Result<JsValue, JsError> {
    let entries: Vec<CodeEntry> = LanguageTag::all()
        .iter()
        .map(|t| CodeEntry {
            code: t.as_str(),
            english_name: t.english_name(),
        })
        .collect();
    to_value(&entries).map_err(js_err)
}

/// Validate a market code and return its canonical form + English name.
/// Throws when the code isn't a known ISO 3166-1 alpha-2 market.
#[wasm_bindgen(js_name = parseMarket, unchecked_return_type = "CodeEntry")]
pub fn parse_market_js(code: &str) -> Result<JsValue, JsError> {
    let m = parse_market(code)?;
    to_value(&CodeEntry {
        code: m.as_str(),
        english_name: m.english_name(),
    })
    .map_err(js_err)
}

/// Validate a language code and return its canonical form + English name.
/// Throws when the code isn't a known ISO 639-1 alpha-2 language.
#[wasm_bindgen(js_name = parseLanguage, unchecked_return_type = "CodeEntry")]
pub fn parse_language_js(code: &str) -> Result<JsValue, JsError> {
    let l = parse_lang(code)?;
    to_value(&CodeEntry {
        code: l.as_str(),
        english_name: l.english_name(),
    })
    .map_err(js_err)
}

/// Validate a BCP-47 language tag against the Microsoft Store list and return
/// its canonical form (e.g. `"en-us"` → `"en-US"`) + English name. Throws
/// when the tag isn't accepted by the Store.
#[wasm_bindgen(js_name = parseLanguageTag, unchecked_return_type = "CodeEntry")]
pub fn parse_language_tag_js(tag: &str) -> Result<JsValue, JsError> {
    let t = LanguageTag::from_str(tag).map_err(|e| JsError::new(&e))?;
    to_value(&CodeEntry {
        code: t.as_str(),
        english_name: t.english_name(),
    })
    .map_err(js_err)
}

/// Validate an identifier-type string in any reasonable casing
/// (`ProductId`, `productId`, `product-id`, `PRODUCT_ID`) and return the
/// canonical camelCase form used by `queryDcat`. Throws on unknown values.
#[wasm_bindgen(js_name = parseIdentifierType, unchecked_return_type = "IdentifierTypeStr")]
pub fn parse_identifier_type_js(raw: &str) -> Result<String, JsError> {
    let it = IdentifierType::parse_tolerant(raw)
        .ok_or_else(|| JsError::new(&format!("unknown identifierType: {raw}")))?;
    Ok(it.as_str().to_owned())
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
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "MarketCode")] market: &str,
        #[wasm_bindgen(unchecked_param_type = "LanguageCode")] language: &str,
        include_neutral: bool,
    ) -> Result<LocaleJs, JsError> {
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

    /// Build a `Locale` from a Microsoft Store BCP-47 tag (e.g. `"en-US"`,
    /// `"zh-Hant-TW"`, `"sr-Cyrl-RS"`). Throws when the tag is unknown, has
    /// no region (`zh-Hant`, `en-053`), or its primary subtag is not
    /// ISO 639-1 (`chr-Cher-US`).
    #[wasm_bindgen(js_name = fromTag)]
    pub fn from_tag(
        #[wasm_bindgen(unchecked_param_type = "LanguageTagCode")] tag: &str,
        include_neutral: bool,
    ) -> Result<LocaleJs, JsError> {
        let parsed = LanguageTag::from_str(tag).map_err(|e| JsError::new(&e))?;
        let inner = Locale::from_tag(parsed, include_neutral).map_err(JsError::new)?;
        Ok(LocaleJs { inner })
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
    #[wasm_bindgen(js_name = toJSON, unchecked_return_type = "LocaleJson")]
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

    /// Install a progress callback fired during `queryDcat`,
    /// `getPackagesForProduct`, and `searchDcat`. Pass `null` to detach.
    /// The callback receives a `{stage, message, current, total}` object;
    /// `current`/`total` are `null` except for counter-style stages.
    ///
    /// Stages currently emitted:
    /// - `dcat.request`, `dcat.response`, `dcat.parse`, `dcat.done`, `dcat.notFound`
    /// - `fe3.start`, `fe3.getCookie`, `fe3.syncUpdates`,
    ///   `fe3.parseUpdateIds`, `fe3.parseUpdateIds.done`,
    ///   `fe3.parsePackages`, `fe3.parsePackages.done`,
    ///   `fe3.resolveUrls`, `fe3.resolveUrls.done`, `fe3.done`
    /// - `search.request`, `search.response`, `search.parse`, `search.done`
    /// - `retry.wait`, `retry.attempt`
    #[wasm_bindgen(js_name = onProgress)]
    pub fn on_progress(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "OnProgress | null")] callback: JsValue,
    ) {
        if callback.is_null() || callback.is_undefined() {
            self.inner.clear_progress_callback();
            return;
        }
        let Ok(func): Result<Function, _> = callback.dyn_into() else {
            self.inner.clear_progress_callback();
            return;
        };
        let cb = move |event: ProgressEvent| {
            let val = to_value(&event).unwrap_or(JsValue::NULL);
            let _ = func.call1(&JsValue::NULL, &val);
        };
        self.inner.set_progress_callback(Box::new(cb));
    }

    /// Query DisplayCatalog for a product by `id` and `idType`. Resolves to the
    /// full product listing on success. An optional `authToken` may be provided
    /// for flighted/sandbox queries. Pass an `AbortSignal` to cancel a stalled
    /// request — rejection becomes `"Operation cancelled"`.
    #[wasm_bindgen(
        js_name = queryDcat,
        unchecked_return_type = "DisplayCatalogModel | null"
    )]
    pub async fn query_dcat(
        &mut self,
        id: String,
        #[wasm_bindgen(unchecked_param_type = "IdentifierTypeStr | string")] id_type: String,
        auth_token: Option<String>,
        #[wasm_bindgen(unchecked_param_type = "AbortSignal | null")] signal: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let t = parse_id_type(&id_type)?;
        let adapter = adapt_signal(&signal)?;
        let cancel = adapter.as_ref().map(|a| &a.token);
        self.inner
            .query_dcat_with_cancel(&id, t, auth_token.as_deref(), cancel)
            .await
            .map_err(store_err)?;
        Ok(to_value(&self.inner.product_listing).map_err(js_err)?)
    }

    /// Resolve the direct download URLs for the currently-loaded product.
    /// Requires `queryDcat` to have been called successfully first.
    ///
    /// Returns `Array<{packageMoniker, packageUri, packageType, applicabilityBlob,
    /// updateId, packageSize}>`. `packageSize` is in bytes; prefer it over a
    /// HEAD request on `packageUri`. It's `null` only for framework packages
    /// that DCat doesn't list a size for. Pass an `AbortSignal` to cancel
    /// stalled FE3 SOAP calls.
    #[wasm_bindgen(
        js_name = getPackagesForProduct,
        unchecked_return_type = "PackageInstance[]"
    )]
    pub async fn get_packages_for_product(
        &self,
        msa_token: Option<String>,
        #[wasm_bindgen(unchecked_param_type = "AbortSignal | null")] signal: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let adapter = adapt_signal(&signal)?;
        let cancel = adapter.as_ref().map(|a| &a.token);
        let packages = self
            .inner
            .get_packages_for_product_with_cancel(msa_token.as_deref(), cancel)
            .await
            .map_err(store_err)?;
        Ok(to_value(&packages).map_err(js_err)?)
    }

    /// Search DisplayCatalog for the given query string. Pass an `AbortSignal`
    /// to cancel a stalled request.
    #[wasm_bindgen(
        js_name = searchDcat,
        unchecked_return_type = "DCatSearch"
    )]
    pub async fn search_dcat(
        &mut self,
        query: String,
        #[wasm_bindgen(unchecked_param_type = "DeviceFamilyStr | string")] device_family: String,
        #[wasm_bindgen(unchecked_param_type = "AbortSignal | null")] signal: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let df = parse_device_family(&device_family)?;
        let adapter = adapt_signal(&signal)?;
        let cancel = adapter.as_ref().map(|a| &a.token);
        let result = self
            .inner
            .search_dcat_with_cancel(&query, df, cancel)
            .await
            .map_err(store_err)?;
        Ok(to_value(&result).map_err(js_err)?)
    }

    /// Same as `searchDcat` but skips the first `skipCount` results (pages of
    /// up to 100 items each).
    #[wasm_bindgen(
        js_name = searchDcatPaged,
        unchecked_return_type = "DCatSearch"
    )]
    pub async fn search_dcat_paged(
        &mut self,
        query: String,
        #[wasm_bindgen(unchecked_param_type = "DeviceFamilyStr | string")] device_family: String,
        skip_count: u32,
        #[wasm_bindgen(unchecked_param_type = "AbortSignal | null")] signal: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let df = parse_device_family(&device_family)?;
        let adapter = adapt_signal(&signal)?;
        let cancel = adapter.as_ref().map(|a| &a.token);
        let result = self
            .inner
            .search_dcat_paged_with_cancel(&query, df, skip_count, cancel)
            .await
            .map_err(store_err)?;
        Ok(to_value(&result).map_err(js_err)?)
    }

    // -- state accessors ---------------------------------------------------

    #[wasm_bindgen(getter, js_name = isFound)]
    pub fn is_found(&self) -> bool {
        self.inner.is_found
    }

    #[wasm_bindgen(getter, js_name = productListing, unchecked_return_type = "DisplayCatalogModel | null")]
    pub fn product_listing(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.product_listing).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = searchResult, unchecked_return_type = "DCatSearch | null")]
    pub fn search_result(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.search_result).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedEndpoint, unchecked_return_type = "DCatEndpointStr")]
    pub fn selected_endpoint(&self) -> Result<JsValue, JsError> {
        to_value(&self.inner.selected_endpoint).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedLocale, unchecked_return_type = "LocaleJson")]
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
    pub async fn get_cookie(&self) -> Result<String, JsValue> {
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
    ) -> Result<String, JsValue> {
        FE3Handler::sync_updates(&wu_category_id, msa_token.as_deref(), &self.client)
            .await
            .map_err(store_err)
    }

    /// Parse the raw `SyncUpdates` XML and extract update + revision IDs.
    /// Returns `{updateIds: string[], revisionIds: string[]}`.
    #[wasm_bindgen(js_name = processUpdateIds)]
    pub fn process_update_ids(xml: &str) -> Result<JsValue, JsValue> {
        let (update_ids, revision_ids) = FE3Handler::process_update_ids(xml).map_err(store_err)?;
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Ids {
            update_ids: Vec<String>,
            revision_ids: Vec<String>,
        }
        Ok(to_value(&Ids {
            update_ids,
            revision_ids,
        })
        .map_err(js_err)?)
    }

    /// Parse the raw `SyncUpdates` XML into typed `PackageInstance` records.
    #[wasm_bindgen(js_name = getPackageInstances)]
    pub async fn get_package_instances(xml: String) -> Result<JsValue, JsValue> {
        let instances = FE3Handler::get_package_instances(&xml)
            .await
            .map_err(store_err)?;
        Ok(to_value(&instances).map_err(js_err)?)
    }

    /// Resolve direct download URLs for the given update + revision IDs.
    /// Returns `Array<{url: string, size: number | null}>`.
    #[wasm_bindgen(js_name = getFileUrls)]
    pub async fn get_file_urls(
        &self,
        update_ids: JsValue,
        revision_ids: JsValue,
        msa_token: Option<String>,
    ) -> Result<JsValue, JsValue> {
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
        Ok(to_value(&mapped).map_err(js_err)?)
    }
}

impl Default for Fe3HandlerJs {
    fn default() -> Self {
        Self::new()
    }
}
