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

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* -------------------------------------------------------------------------
 * Opaque handle
 * ---------------------------------------------------------------------- */

typedef struct StorelibHandle StorelibHandle;

/* -------------------------------------------------------------------------
 * Return / error codes
 * ---------------------------------------------------------------------- */

#define STORELIB_OK            0
#define STORELIB_ERR_NULL     -1   /* null pointer argument                 */
#define STORELIB_ERR_HTTP     -2   /* HTTP transport error                  */
#define STORELIB_ERR_JSON     -3   /* JSON parse / serialise error          */
#define STORELIB_ERR_XML      -4   /* XML parse error (FE3 responses)       */
#define STORELIB_ERR_NOT_FOUND -5  /* product not found in the catalog      */
#define STORELIB_ERR_TIMEOUT  -6   /* request timed out                     */
#define STORELIB_ERR_OTHER    -7   /* all other errors                      */

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
 * Package resolution
 * ---------------------------------------------------------------------- */

/**
 * Resolve and return the package list as a JSON array.
 *
 * Each element is an object with fields:
 *   "package_moniker" : string
 *   "package_uri"     : string | null
 *   "package_type"    : "Uap" | "Xap" | "AppX" | "Unknown"
 *   "update_id"       : string
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
