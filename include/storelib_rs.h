/**
 * storelib_rs.h — C API for storelib_rs
 *
 * Build the DLL with:
 *   cargo build --release --features ffi
 *
 * The output is:
 *   target/release/storelib_rs.dll          (Windows)
 *   target/release/storelib_rs.dll.lib      (import library)
 *   target/release/libstorelib_rs.so        (Linux)
 *   target/release/libstorelib_rs.dylib     (macOS)
 *
 * License: GPL-3.0-only
 */

#ifndef STORELIB_RS_H
#define STORELIB_RS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* -------------------------------------------------------------------------
 * Opaque handle
 * ---------------------------------------------------------------------- */

typedef struct StorelibHandle       StorelibHandle;
typedef struct StorelibCancellation StorelibCancellation;

/* -------------------------------------------------------------------------
 * Return / error codes
 * ---------------------------------------------------------------------- */

#define STORELIB_OK             0
#define STORELIB_ERR_NULL      -1  /* null pointer argument                 */
#define STORELIB_ERR_HTTP      -2  /* HTTP transport error                  */
#define STORELIB_ERR_JSON      -3  /* JSON parse / serialise error          */
#define STORELIB_ERR_XML       -4  /* XML parse error (FE3 responses)       */
#define STORELIB_ERR_NOT_FOUND -5  /* product not found in the catalog      */
#define STORELIB_ERR_TIMEOUT   -6  /* request timed out                     */
#define STORELIB_ERR_OTHER     -7  /* all other errors                      */
#define STORELIB_ERR_CANCELLED -8  /* operation cancelled                   */

/* -------------------------------------------------------------------------
 * IdentifierType constants  (storelib_query id_type parameter)
 * ---------------------------------------------------------------------- */

#define STORELIB_ID_PRODUCT_ID           0
#define STORELIB_ID_XBOX_TITLE_ID        1
#define STORELIB_ID_PACKAGE_FAMILY_NAME  2
#define STORELIB_ID_CONTENT_ID           3
#define STORELIB_ID_LEGACY_PHONE         4
#define STORELIB_ID_LEGACY_STORE         5
#define STORELIB_ID_LEGACY_XBOX          6

/* -------------------------------------------------------------------------
 * DeviceFamily constants  (storelib_search_json family parameter)
 * ---------------------------------------------------------------------- */

#define STORELIB_FAMILY_DESKTOP    0
#define STORELIB_FAMILY_MOBILE     1
#define STORELIB_FAMILY_XBOX       2
#define STORELIB_FAMILY_SERVER     3
#define STORELIB_FAMILY_IOT        4
#define STORELIB_FAMILY_HOLOLENS   5
#define STORELIB_FAMILY_ANDROMEDA  6
#define STORELIB_FAMILY_UNIVERSAL  7
#define STORELIB_FAMILY_WCOS       8

/* -------------------------------------------------------------------------
 * Lifecycle
 * ---------------------------------------------------------------------- */

/**
 * Create a new handle configured for the production endpoint (US/en locale).
 * Returns NULL if the internal async runtime cannot be initialised.
 * Free with storelib_free().
 */
StorelibHandle* storelib_new(void);

/**
 * Free a handle created with storelib_new().
 * Passing NULL is safe (no-op).
 */
void storelib_free(StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * Error retrieval
 * ---------------------------------------------------------------------- */

/**
 * Return the last error message for handle, or NULL if there was none.
 *
 * The returned pointer is valid until the next call on this handle or until
 * the handle is freed.  Do NOT pass it to storelib_free_string().
 */
const char* storelib_last_error(const StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * Query
 * ---------------------------------------------------------------------- */

/**
 * Query the DisplayCatalog for a product.
 *
 * @param handle      A valid handle from storelib_new().
 * @param id          The product identifier (UTF-8, NUL-terminated).
 * @param id_type     One of the STORELIB_ID_* constants.
 * @param auth_token  Optional MSA / XBL3.0 token, or NULL.
 *
 * @return STORELIB_OK on success, or a negative STORELIB_ERR_* code.
 *         Call storelib_is_found() to check whether the product exists.
 */
int32_t storelib_query(
    StorelibHandle* handle,
    const char*     id,
    uint32_t        id_type,
    const char*     auth_token
);

/**
 * Returns 1 if the last storelib_query() found the product, 0 otherwise.
 */
int32_t storelib_is_found(const StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * Product info
 * ---------------------------------------------------------------------- */

/**
 * Return the full product listing as a JSON string.
 *
 * Returns NULL if no product has been queried yet or if serialisation fails.
 * The caller MUST free the returned string with storelib_free_string().
 */
char* storelib_product_json(const StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * Typed accessors  (convenience views over storelib_product_json output)
 * ---------------------------------------------------------------------- */

/**
 * Return the first product's title (UTF-8, NUL-terminated), or NULL if
 * the listing has no localized properties or no title.
 *
 * Caller frees the returned string with storelib_free_string().
 */
char* storelib_product_title(const StorelibHandle* handle);

/**
 * Return the first product's publisher name, or NULL.
 * Caller frees with storelib_free_string().
 */
char* storelib_product_publisher(const StorelibHandle* handle);

/**
 * Return the first price as a JSON object:
 *   {"currencyCode", "isPIRequired", "listPrice", "msrp",
 *    "taxType", "wholesaleCurrencyCode"}
 * Returns NULL if the product has no priced availability.
 * Caller frees with storelib_free_string().
 */
char* storelib_price_json(const StorelibHandle* handle);

/**
 * Return all `Package` entries from the first SKU as a JSON array.
 * For *resolved* download URLs use storelib_packages_json() instead;
 * this returns the catalog metadata only.
 * Caller frees with storelib_free_string().
 */
char* storelib_packages_listed_json(const StorelibHandle* handle);

/**
 * Return the product's distinct framework dependencies (DisplayCatalog
 * FrameworkDependencies, deduped by packageIdentity) as a JSON array — the
 * named dependency map: [{"packageIdentity","minVersion","maxTested"}].
 * Returns "[]" if no listing is loaded.
 * Caller frees with storelib_free_string().
 */
char* storelib_framework_dependencies_json(const StorelibHandle* handle);

/**
 * Return the product's distinct platform dependencies (DisplayCatalog
 * PlatformDependencies, deduped by platformName) as a JSON array:
 * [{"platformName","minVersion","maxTested"}].
 * Returns "[]" if no listing is loaded.
 * Caller frees with storelib_free_string().
 */
char* storelib_platform_dependencies_json(const StorelibHandle* handle);

/**
 * Return all `Availability` entries flattened across the product's SKUs,
 * as a JSON array. Returns "[]" if no listing is loaded.
 * Caller frees with storelib_free_string().
 */
char* storelib_availabilities_json(const StorelibHandle* handle);

/**
 * Return the WuCategoryId from the first SKU's fulfillment data, or NULL.
 * Caller frees with storelib_free_string().
 */
char* storelib_wu_category_id(const StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * Batch query  (multiple products in one HTTP round-trip)
 * ---------------------------------------------------------------------- */

/**
 * Query the DisplayCatalog for many products in a single round-trip via
 * the `bigIds` parameter.
 *
 * @param handle      A valid handle from storelib_new().
 * @param ids         Array of `id_count` NUL-terminated UTF-8 strings;
 *                    each must be a Microsoft Store Product ID
 *                    (alternate identifiers are not supported here).
 * @param id_count    Number of entries in `ids` (must be > 0).
 * @param auth_token  Optional MSA / XBL3.0 token, or NULL.
 *
 * Returns the response as a JSON array of products on success, or NULL
 * on error (inspect storelib_last_error()).
 * Caller frees the returned string with storelib_free_string().
 */
char* storelib_query_batch_json(
    StorelibHandle*    handle,
    const char* const* ids,
    size_t             id_count,
    const char*        auth_token
);

/**
 * Cancellable variant of storelib_query_batch_json().
 */
char* storelib_query_batch_json_with_cancel(
    StorelibHandle*             handle,
    const char* const*          ids,
    size_t                      id_count,
    const char*                 auth_token,
    const StorelibCancellation* cancel
);

/* -------------------------------------------------------------------------
 * Package resolution
 * ---------------------------------------------------------------------- */

/**
 * Resolve and return the package list as a JSON array.
 *
 * Each element is an object with fields (camelCase, matching the JS binding):
 *   "packageMoniker"     : string
 *   "packageUri"         : string | null
 *   "packageType"        : "uap" | "xap" | "appX" | "unknown"
 *   "applicabilityBlob"  : object | null
 *   "updateId"           : string
 *   "packageSize"        : number | null  (bytes; FE3 first, falls back to
 *                                          DisplayCatalog MaxDownloadSizeInBytes.
 *                                          null for framework packages that DCat
 *                                          does not list a size for.)
 *   "fileName"           : string | null  (FE3 raw <File FileName="..."> —
 *                                          typically "<guid>.<ext>")
 *   "readableFileName"   : string         (<packageMoniker><real extension>;
 *                                          falls back to ".appx" if FE3 did
 *                                          not report a recognised one)
 *   "prerequisites"      : string[]       (FE3 dependency edges — Windows
 *                                          Update *category* GUIDs from
 *                                          <Relationships><Prerequisites>; one
 *                                          is the product's own WuCategoryId.
 *                                          For named deps use
 *                                          storelib_framework_dependencies_json.)
 *   "bundledUpdates"     : string[]       (child update GUIDs this update bundles)
 *   "relationships"      : object         (full <Relationships> graph with
 *                                          IsCategory grouping + revisionNumbers)
 *
 * The object additionally carries the complete SyncUpdates metadata with no
 * field dropped: "revisionNumber", "updateInfoId", "isLeaf", "isShared",
 * "installerSpecificIdentifier", "packageFileName", "handlerType",
 * "updateProperties", "familyMetadata", "categoryInformation", "deployment",
 * "applicabilityRulesXml", "installationBehaviorXml", plus an
 * "extraAttributes" map that preserves any attribute not mapped above.
 *
 * @param handle     A valid handle after a successful storelib_query().
 * @param msa_token  Optional auth token, or NULL.
 *
 * Returns NULL on error; check storelib_last_error() for the message.
 * The caller MUST free the returned string with storelib_free_string().
 */
char* storelib_packages_json(StorelibHandle* handle, const char* msa_token);

/* -------------------------------------------------------------------------
 * Search
 * ---------------------------------------------------------------------- */

/**
 * Search the catalog and return results as a JSON string.
 *
 * @param handle  A valid handle from storelib_new().
 * @param query   Search query (UTF-8, NUL-terminated).
 * @param family  One of the STORELIB_FAMILY_* constants.
 *
 * Returns NULL on error; check storelib_last_error() for the message.
 * The caller MUST free the returned string with storelib_free_string().
 */
char* storelib_search_json(StorelibHandle* handle, const char* query, uint32_t family);

/* -------------------------------------------------------------------------
 * Cancellation
 * ---------------------------------------------------------------------- */

/**
 * Allocate a fresh, uncancelled cancellation token. Pass it to any
 * `storelib_*_with_cancel` function to give callers a way to abort a
 * stalled call. Cancellation may be signalled from any thread.
 *
 * Free with storelib_cancellation_free().
 *
 * Returns NULL only on allocation failure (extremely unlikely).
 */
StorelibCancellation* storelib_cancellation_new(void);

/**
 * Signal cancellation. Safe to call from any thread (e.g. a watchdog timer
 * thread while another thread is blocked inside storelib_query_with_cancel).
 * Idempotent — calling more than once is harmless.
 *
 * Passing NULL is a safe no-op.
 */
void storelib_cancellation_cancel(const StorelibCancellation* token);

/**
 * Returns 1 if `token` has been cancelled, 0 otherwise (also 0 if `token`
 * is NULL).
 */
int32_t storelib_cancellation_is_cancelled(const StorelibCancellation* token);

/**
 * Free a cancellation token. After this call the pointer is invalid.
 * Passing NULL is a safe no-op.
 */
void storelib_cancellation_free(StorelibCancellation* token);

/**
 * Like storelib_query() but races against `cancel`. Returns
 * STORELIB_ERR_CANCELLED if the token fires before the request completes.
 * Pass NULL for `cancel` to disable cancellation.
 */
int32_t storelib_query_with_cancel(
    StorelibHandle*             handle,
    const char*                 id,
    uint32_t                    id_type,
    const char*                 auth_token,
    const StorelibCancellation* cancel
);

/**
 * Like storelib_packages_json() but races against `cancel`. Returns NULL on
 * error or cancellation; inspect storelib_last_error().
 * The caller MUST free the returned string with storelib_free_string().
 */
char* storelib_packages_json_with_cancel(
    StorelibHandle*             handle,
    const char*                 msa_token,
    const StorelibCancellation* cancel
);

/**
 * Like storelib_search_json() but races against `cancel`. Returns NULL on
 * error or cancellation; inspect storelib_last_error().
 * The caller MUST free the returned string with storelib_free_string().
 */
char* storelib_search_json_with_cancel(
    StorelibHandle*             handle,
    const char*                 query,
    uint32_t                    family,
    const StorelibCancellation* cancel
);

/* -------------------------------------------------------------------------
 * Progress reporting
 * ---------------------------------------------------------------------- */

/**
 * Real-time progress callback signature.
 *
 * Fired during storelib_query(), storelib_packages_json(), and
 * storelib_search_json() at each phase boundary.
 *
 * @param stage        Stable stage identifier, NUL-terminated UTF-8.
 *                     Valid only for the duration of the call. Examples:
 *                       "dcat.request", "dcat.response", "dcat.parse",
 *                       "dcat.done", "dcat.notFound",
 *                       "fe3.start", "fe3.getCookie", "fe3.syncUpdates",
 *                       "fe3.parseUpdateIds", "fe3.parseUpdateIds.done",
 *                       "fe3.parsePackages", "fe3.parsePackages.done",
 *                       "fe3.resolveUrls", "fe3.resolveUrls.done",
 *                       "fe3.done",
 *                       "search.request", "search.response",
 *                       "search.parse", "search.done"
 * @param message      Human-readable detail, NUL-terminated UTF-8.
 *                     Valid only for the duration of the call.
 * @param has_current  1 if `current` carries a meaningful counter, else 0.
 * @param current      Counter value (e.g. "5 of 12 packages"); meaningful
 *                     only when `has_current` is 1.
 * @param has_total    1 if `total` carries a meaningful counter, else 0.
 * @param total        Counter total; meaningful only when `has_total` is 1.
 * @param user_data    Opaque pointer originally passed to
 *                     storelib_set_progress_callback().
 *
 * The callback is invoked from the Tokio runtime worker thread that drives
 * the active async call — it must be thread-safe.
 */
typedef void (*storelib_progress_cb)(
    const char* stage,
    const char* message,
    int32_t     has_current,
    uint32_t    current,
    int32_t     has_total,
    uint32_t    total,
    void*       user_data
);

/**
 * Install a progress callback on `handle`. Pass NULL as `callback` to detach
 * (equivalent to storelib_clear_progress_callback()).
 *
 * `user_data` is opaque to the library and is passed back to every callback
 * invocation; it may be NULL. The callback must outlive any in-flight
 * storelib_* call on this handle.
 *
 * @return STORELIB_OK on success, or STORELIB_ERR_NULL when `handle` is NULL.
 */
int32_t storelib_set_progress_callback(
    StorelibHandle*       handle,
    storelib_progress_cb  callback,
    void*                 user_data
);

/**
 * Detach the progress callback (if any). Equivalent to
 * storelib_set_progress_callback(handle, NULL, NULL).
 */
int32_t storelib_clear_progress_callback(StorelibHandle* handle);

/* -------------------------------------------------------------------------
 * String management
 * ---------------------------------------------------------------------- */

/**
 * Free a string returned by storelib_product_json(), storelib_packages_json(),
 * or storelib_search_json().
 *
 * Passing NULL is safe (no-op).
 * Do NOT call this on the pointer from storelib_last_error().
 */
void storelib_free_string(char* s);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* STORELIB_RS_H */
