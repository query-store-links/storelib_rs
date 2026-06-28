//! JS bindings produced by `wasm-bindgen`.
//!
//! Exposes the full library surface — handlers, helpers, and value types — to
//! JavaScript consumers. Enum-typed parameters are accepted as camelCase
//! strings, Market/Lang as their canonical wire form (e.g. `"US"`, `"en-US"`),
//! and complex values cross the FFI boundary as plain JS objects via
//! `serde-wasm-bindgen`.

use std::str::FromStr;

use js_sys::{Function, Reflect};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, Serializer};
use wasm_bindgen::prelude::*;

use crate::cancellation::CancellationToken;
use crate::error::StoreError;
use crate::models::enums::{DCatEndpoint, DeviceFamily, IdentifierType};
use crate::models::locale::{Lang, LanguageTag, Locale, Market};
use crate::services::display_catalog::{DisplayCatalogHandler, ProgressEmitter, ProgressEvent};
use crate::services::fe3::FE3Handler;
use crate::utilities::helpers as h;

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn js_err<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}

/// Serialize any `serde::Serialize` value into a `JsValue`, emitting JS
/// `BigInt` for 64-bit integers that fall outside JS's safe-integer range
/// instead of throwing.
///
/// The DisplayCatalog API legitimately returns counts like
/// `RatingCount: 1407657960666562560` for popular products — well above
/// `Number.MAX_SAFE_INTEGER` (2⁵³ − 1). The default `to_value` would throw
/// `"… can't be represented as a JavaScript number"`; this helper renders
/// them as `BigInt` (`1407657960666562560n`) so the value survives the
/// crossing.
///
/// Note for JS consumers: with this setting, *every* `i64`/`u64` field is
/// serialized as `BigInt`, even small values. If you `JSON.stringify` the
/// result, install a BigInt-aware replacer first:
///
/// ```js
/// JSON.stringify(obj, (_k, v) => typeof v === 'bigint' ? v.toString() : v);
/// ```
fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let serializer = Serializer::new().serialize_large_number_types_as_bigints(true);
    value.serialize(&serializer)
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

    // Expose the full Rust-side source chain as `causes: string[]` so JS can
    // render a complete diagnostic without losing the underlying reason
    // (e.g. `reqwest::Error → hyper::Error → io::Error`). Index 0 of
    // `causes` is the layer directly under `e.message`; the array is empty
    // when the error has no underlying cause. (Rust backtraces aren't useful
    // in wasm — the JS engine already populates `.stack` on `Error`.)
    let causes_arr = js_sys::Array::new();
    for c in e.causes().into_iter().skip(1) {
        causes_arr.push(&JsValue::from_str(&c));
    }
    let _ = Reflect::set(&err, &JsValue::from_str("causes"), &causes_arr);
    err.into()
}

/// Wire a JS `Function` into `emitter`, or detach when `callback` is
/// `null`/`undefined`. Shared by every `onProgress` binding.
fn install_js_progress(callback: JsValue, emitter: &mut ProgressEmitter) {
    if callback.is_null() || callback.is_undefined() {
        emitter.clear();
        return;
    }
    let Ok(func): Result<Function, _> = callback.dyn_into() else {
        emitter.clear();
        return;
    };
    emitter.set(Box::new(move |event: ProgressEvent| {
        let val = to_js(&event).unwrap_or(JsValue::NULL);
        let _ = func.call1(&JsValue::NULL, &val);
    }));
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
// Generated at build time by `tools/gen-ts.mjs` (run from `build.rs`) from
// every `#[derive(Serialize)]` struct and enum on the wasm surface, plus the
// `ProgressStage` union scraped from `.emit()` call sites and the
// `StorelibError` shape derived from `store_err`'s match arms. To regenerate
// without a full build:  node tools/gen-ts.mjs
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = include_str!(concat!(env!("OUT_DIR"), "/wasm_types.d.ts"));

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Map an `AppxMetadata` package-type string (e.g. `"AppX"`) to the canonical
/// camelCase enum value (`"appX"`, `"uap"`, `"xap"`, or `"unknown"`).
#[wasm_bindgen(js_name = stringToPackageType)]
pub fn string_to_package_type_js(raw: &str) -> Result<JsValue, JsError> {
    to_js(&h::string_to_package_type(raw)).map_err(js_err)
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
    to_js(&entries).map_err(js_err)
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
    to_js(&entries).map_err(js_err)
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
    to_js(&entries).map_err(js_err)
}

/// Validate a market code and return its canonical form + English name.
/// Throws when the code isn't a known ISO 3166-1 alpha-2 market.
#[wasm_bindgen(js_name = parseMarket, unchecked_return_type = "CodeEntry")]
pub fn parse_market_js(code: &str) -> Result<JsValue, JsError> {
    let m = parse_market(code)?;
    to_js(&CodeEntry {
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
    to_js(&CodeEntry {
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
    to_js(&CodeEntry {
        code: t.as_str(),
        english_name: t.english_name(),
    })
    .map_err(js_err)
}

/// Validate an identifier-type string in any reasonable casing
/// (`ProductId`, `productId`, `product-id`, `PRODUCT_ID`) and return the
/// canonical camelCase form used by `queryDcat`. Throws on unknown values.
#[wasm_bindgen(js_name = parseIdentifierType, unchecked_return_type = "IdentifierType")]
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
        #[wasm_bindgen(unchecked_param_type = "Market")] market: &str,
        #[wasm_bindgen(unchecked_param_type = "Lang")] language: &str,
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

    /// Default production locale: `US / en`, neutral disabled, full-tag
    /// emission enabled so DCat requests carry `languages=en-US` and
    /// pick up CMS video metadata (which is empty for the bare `en`
    /// language form).
    #[wasm_bindgen(js_name = production)]
    pub fn production() -> LocaleJs {
        LocaleJs {
            inner: Locale::production(),
        }
    }

    /// Toggle [`useFullTag`] fluently — returns a new `Locale` rather
    /// than mutating in place (the underlying Rust struct is `Clone`).
    /// Pass `true` to send BCP-47 tags (`en-US`); `false` for the bare
    /// ISO 639-1 code (`en`).
    #[wasm_bindgen(js_name = withFullTag)]
    pub fn with_full_tag(&self, enabled: bool) -> LocaleJs {
        LocaleJs {
            inner: self.inner.clone().with_full_tag(enabled),
        }
    }

    /// Returns whether DCat requests will carry `languages=<lang>-<market>`
    /// (true) or just `languages=<lang>` (false).
    #[wasm_bindgen(getter, js_name = useFullTag)]
    pub fn use_full_tag(&self) -> bool {
        self.inner.use_full_tag
    }

    /// Build a `Locale` from a Microsoft Store BCP-47 tag (e.g. `"en-US"`,
    /// `"zh-Hant-TW"`, `"sr-Cyrl-RS"`). Throws when the tag is unknown, has
    /// no region (`zh-Hant`, `en-053`), or its primary subtag is not
    /// ISO 639-1 (`chr-Cher-US`).
    #[wasm_bindgen(js_name = fromTag)]
    pub fn from_tag(
        #[wasm_bindgen(unchecked_param_type = "LanguageTag")] tag: &str,
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
        to_js(&self.inner).map_err(js_err)
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
    /// - `dcat.request`, `dcat.response`, `dcat.parse`, `dcat.done`,
    ///   `dcat.dependencies` *(framework / platform dependency counts;
    ///   `current`/`total` = framework count / framework+platform)*,
    ///   `dcat.notFound`
    /// - `fe3.start`, `fe3.getCookie`, `fe3.syncUpdates`,
    ///   `fe3.parseUpdateIds`, `fe3.parseUpdateIds.done`,
    ///   `fe3.parsePackages`, `fe3.parsePackages.done`,
    ///   `fe3.prerequisites` *(dependency-edge totals;
    ///   `current`/`total` = packages-with-prereqs / total packages)*,
    ///   `fe3.packageFound` *(per package; `message` includes `prereqs=N`)*,
    ///   `fe3.resolveUrls`, `fe3.resolveUrls.done`,
    ///   `fe3.linkReceived` *(per URL; `message` = URL)*,
    ///   `fe3.packageResolved` *(per package; `message` includes
    ///   `size=`, `uri=`, `prereqs=`)*,
    ///   `fe3.done`
    /// - `search.request`, `search.response`, `search.parse`, `search.done`
    /// - `retry.wait`, `retry.attempt`
    #[wasm_bindgen(js_name = onProgress)]
    pub fn on_progress(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "OnProgress | null")] callback: JsValue,
    ) {
        install_js_progress(callback, &mut self.inner.progress);
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
        #[wasm_bindgen(unchecked_param_type = "IdentifierType | string")] id_type: String,
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
        Ok(to_js(&self.inner.product_listing).map_err(js_err)?)
    }

    /// Resolve the direct download URLs for the currently-loaded product.
    /// Requires `queryDcat` to have been called successfully first.
    ///
    /// Returns `PackageInstance[]` — see the `PackageInstance` type for the
    /// full shape. Beyond the download fields (`packageMoniker`, `packageUri`,
    /// `packageSize`, …) each entry carries the complete SyncUpdates metadata
    /// with nothing dropped: `prerequisites` / `bundledUpdates` / the full
    /// `relationships` graph, `deployment`, `updateProperties`,
    /// `familyMetadata`, `categoryInformation`, raw `applicabilityRulesXml`,
    /// and an `extraAttributes` catch-all. `packageSize` is in bytes (prefer
    /// it over a HEAD on `packageUri`); for the *named* framework dependency
    /// map use the handler's `frameworkDependencies` getter. Pass an
    /// `AbortSignal` to cancel stalled FE3 SOAP calls.
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
        Ok(to_js(&packages).map_err(js_err)?)
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
        #[wasm_bindgen(unchecked_param_type = "DeviceFamily | string")] device_family: String,
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
        Ok(to_js(&result).map_err(js_err)?)
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
        #[wasm_bindgen(unchecked_param_type = "DeviceFamily | string")] device_family: String,
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
        Ok(to_js(&result).map_err(js_err)?)
    }

    // -- state accessors ---------------------------------------------------

    #[wasm_bindgen(getter, js_name = isFound)]
    pub fn is_found(&self) -> bool {
        self.inner.is_found
    }

    #[wasm_bindgen(getter, js_name = productListing, unchecked_return_type = "DisplayCatalogModel | null")]
    pub fn product_listing(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.product_listing).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = searchResult, unchecked_return_type = "DCatSearch | null")]
    pub fn search_result(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.search_result).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedEndpoint, unchecked_return_type = "DCatEndpoint")]
    pub fn selected_endpoint(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.selected_endpoint).map_err(js_err)
    }

    #[wasm_bindgen(getter, js_name = selectedLocale, unchecked_return_type = "LocaleJson")]
    pub fn selected_locale(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.selected_locale).map_err(js_err)
    }

    #[wasm_bindgen(getter)]
    pub fn result(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.result).map_err(js_err)
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> Option<String> {
        self.inner.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.inner.error.clone()
    }

    // -- typed product accessors ------------------------------------------
    //
    // These mirror the native accessors on `DisplayCatalogHandler`.

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> Option<String> {
        self.inner.title().map(str::to_owned)
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description().map(str::to_owned)
    }

    #[wasm_bindgen(getter, js_name = publisherName)]
    pub fn publisher_name(&self) -> Option<String> {
        self.inner.publisher_name().map(str::to_owned)
    }

    #[wasm_bindgen(getter, js_name = wuCategoryId)]
    pub fn wu_category_id(&self) -> Option<String> {
        self.inner.wu_category_id().map(str::to_owned)
    }

    #[wasm_bindgen(getter, js_name = lastModifiedDate)]
    pub fn last_modified_date(&self) -> Option<String> {
        self.inner.last_modified_date().map(str::to_owned)
    }

    /// First [`Price`] across all SKUs / availabilities, or `null` if none
    /// is listed (e.g. the product is free or unavailable in the locale).
    #[wasm_bindgen(getter, unchecked_return_type = "Price | null")]
    pub fn price(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.price()).map_err(js_err)
    }

    /// Every [`Price`] across all availabilities (an app can have multiple
    /// for different markets / channels).
    #[wasm_bindgen(getter, unchecked_return_type = "Price[]")]
    pub fn prices(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.prices()).map_err(js_err)
    }

    /// Packages from the first SKU's properties. For the resolved download
    /// URLs, use `getPackagesForProduct` instead.
    #[wasm_bindgen(getter, unchecked_return_type = "Package[]")]
    pub fn packages(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.packages()).map_err(js_err)
    }

    /// Distinct framework / runtime dependencies declared across the product's
    /// packages (DisplayCatalog `FrameworkDependencies`) — the *named*
    /// dependency map (`packageIdentity` + `minVersion`), deduplicated by
    /// `packageIdentity`.
    #[wasm_bindgen(getter, js_name = frameworkDependencies, unchecked_return_type = "FrameworkDependency[]")]
    pub fn framework_dependencies(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.framework_dependencies()).map_err(js_err)
    }

    /// Distinct platform dependencies (`Windows.Universal`, `Windows.Desktop`,
    /// …) declared across the product's packages.
    #[wasm_bindgen(getter, js_name = platformDependencies, unchecked_return_type = "PlatformDependency[]")]
    pub fn platform_dependencies(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.platform_dependencies()).map_err(js_err)
    }

    /// All `Availability` entries flattened across the product's SKUs.
    #[wasm_bindgen(getter, unchecked_return_type = "Availability[]")]
    pub fn availabilities(&self) -> Result<JsValue, JsError> {
        to_js(&self.inner.availabilities()).map_err(js_err)
    }

    /// All products from the most recent query (single or batch).
    #[wasm_bindgen(getter, unchecked_return_type = "Product[]")]
    pub fn products(&self) -> Result<JsValue, JsError> {
        to_js(self.inner.products()).map_err(js_err)
    }

    /// Images on the first localized property filtered by `purpose`
    /// (case-sensitive PascalCase, e.g. `"Logo"`, `"Tile"`, `"Screenshot"`).
    #[wasm_bindgen(js_name = imagesWithPurpose, unchecked_return_type = "Image[]")]
    pub fn images_with_purpose(&self, purpose: &str) -> Result<JsValue, JsError> {
        to_js(&self.inner.images_with_purpose(purpose)).map_err(js_err)
    }

    // -- batch product query ----------------------------------------------

    /// Query DisplayCatalog for many products in a single round-trip.
    /// `ids` must be Microsoft Store Product IDs — alternate identifiers
    /// are not supported by the batch endpoint. Populates `productListing`
    /// and `products`.
    #[wasm_bindgen(
        js_name = queryDcatBatch,
        unchecked_return_type = "Product[]"
    )]
    pub async fn query_dcat_batch(
        &mut self,
        ids: Vec<String>,
        auth_token: Option<String>,
        #[wasm_bindgen(unchecked_param_type = "AbortSignal | null")] signal: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let adapter = adapt_signal(&signal)?;
        let cancel = adapter.as_ref().map(|a| &a.token);
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        self.inner
            .query_dcat_batch_with_cancel(&id_refs, auth_token.as_deref(), cancel)
            .await
            .map_err(store_err)?;
        Ok(to_js(self.inner.products()).map_err(js_err)?)
    }
}

// ---------------------------------------------------------------------------
// FE3Handler
// ---------------------------------------------------------------------------

/// Low-level wrapper around the FE3 (Windows Update) SOAP endpoints used to
/// resolve direct package download URLs.
#[wasm_bindgen(js_name = Fe3Handler)]
pub struct Fe3HandlerJs {
    inner: FE3Handler,
}

#[wasm_bindgen(js_class = Fe3Handler)]
impl Fe3HandlerJs {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Fe3HandlerJs {
        let client = reqwest::Client::builder()
            .user_agent("StoreLib")
            .build()
            .unwrap_or_default();
        Fe3HandlerJs {
            inner: FE3Handler::new(client),
        }
    }

    /// Install a progress callback fired during `getFileUrls`. Pass `null`
    /// to detach. The callback receives a `{stage, message, current, total}`
    /// object.
    ///
    /// Stages currently emitted:
    /// - `fe3.linkReceived` *(per URL; `message` =
    ///   `"uri=<url> | size=<bytes-or-?> | updateId=<id>"`)*
    #[wasm_bindgen(js_name = onProgress)]
    pub fn on_progress(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "OnProgress | null")] callback: JsValue,
    ) {
        install_js_progress(callback, &mut self.inner.progress);
    }

    /// POST `GetCookie` and return the `EncryptedData` value from the response.
    #[wasm_bindgen(js_name = getCookie)]
    pub async fn get_cookie(&self) -> Result<String, JsValue> {
        FE3Handler::get_cookie(&self.inner.client)
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
        FE3Handler::sync_updates(&wu_category_id, msa_token.as_deref(), &self.inner.client)
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
        Ok(to_js(&Ids {
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
        Ok(to_js(&instances).map_err(js_err)?)
    }

    /// Resolve direct download URLs for the given update + revision IDs.
    /// Returns `Array<{url: string, size: number | null}>`.
    ///
    /// Subscribe via [`Self::on_progress`] to stream `fe3.linkReceived`
    /// events as each `GetExtendedUpdateInfo2` response is parsed.
    #[wasm_bindgen(js_name = getFileUrls)]
    pub async fn get_file_urls(
        &self,
        update_ids: JsValue,
        revision_ids: JsValue,
        msa_token: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let update_ids: Vec<String> = from_value(update_ids).map_err(js_err)?;
        let revision_ids: Vec<String> = from_value(revision_ids).map_err(js_err)?;

        let pairs = self
            .inner
            .get_file_urls(&update_ids, &revision_ids, msa_token.as_deref())
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
        Ok(to_js(&mapped).map_err(js_err)?)
    }
}

impl Default for Fe3HandlerJs {
    fn default() -> Self {
        Self::new()
    }
}
