//! C-compatible FFI layer for `storelib_rs`.
//!
//! Enabled with `--features ffi` and excluded on `wasm32` targets.
//!
//! # Memory contract
//!
//! - Strings **returned** by the library (`char *`) are heap-allocated by
//!   Rust and **must** be released by the caller with [`storelib_free_string`].
//! - Strings **passed in** by the caller are borrowed for the duration of the
//!   call only; the caller retains ownership.
//! - The last-error string returned by [`storelib_last_error`] is owned by the
//!   handle and is valid until the next call on that handle or until the handle
//!   is freed — do **not** pass it to [`storelib_free_string`].

#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use std::ffi::c_void;

use crate::models::enums::{DeviceFamily, IdentifierType};
use crate::services::display_catalog::{DisplayCatalogHandler, ProgressEvent};

// ---------------------------------------------------------------------------
// Error codes (mirrored in the C header)
// ---------------------------------------------------------------------------

pub const STORELIB_OK: i32 = 0;
pub const STORELIB_ERR_NULL: i32 = -1;
pub const STORELIB_ERR_HTTP: i32 = -2;
pub const STORELIB_ERR_JSON: i32 = -3;
pub const STORELIB_ERR_XML: i32 = -4;
pub const STORELIB_ERR_NOT_FOUND: i32 = -5;
pub const STORELIB_ERR_TIMEOUT: i32 = -6;
pub const STORELIB_ERR_OTHER: i32 = -7;
pub const STORELIB_ERR_CANCELLED: i32 = -8;

fn err_code(e: &crate::error::StoreError) -> i32 {
    use crate::error::StoreError::*;
    match e {
        Http(_) => STORELIB_ERR_HTTP,
        Json(_) => STORELIB_ERR_JSON,
        Xml(_) => STORELIB_ERR_XML,
        NotFound => STORELIB_ERR_NOT_FOUND,
        TimedOut => STORELIB_ERR_TIMEOUT,
        Cancelled => STORELIB_ERR_CANCELLED,
        Other(_) => STORELIB_ERR_OTHER,
    }
}

// ---------------------------------------------------------------------------
// Opaque handle
// ---------------------------------------------------------------------------

pub struct StorelibHandle {
    handler: DisplayCatalogHandler,
    rt: tokio::runtime::Runtime,
    last_error: Option<CString>,
}

impl StorelibHandle {
    fn set_error(&mut self, msg: &str) {
        self.last_error = CString::new(msg).ok();
    }
    fn clear_error(&mut self) {
        self.last_error = None;
    }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Create a new handle configured for the production endpoint (US/en locale).
///
/// Returns `NULL` if the tokio runtime cannot be initialised.
/// Free with [`storelib_free`].
#[no_mangle]
pub extern "C" fn storelib_new() -> *mut StorelibHandle {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };
    let handle = Box::new(StorelibHandle {
        handler: DisplayCatalogHandler::production(),
        rt,
        last_error: None,
    });
    Box::into_raw(handle)
}

/// Free a handle created with [`storelib_new`].
///
/// # Safety
/// `handle` must be a valid pointer returned by [`storelib_new`] that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn storelib_free(handle: *mut StorelibHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

// ---------------------------------------------------------------------------
// Error retrieval
// ---------------------------------------------------------------------------

/// Return the last error message for `handle`, or `NULL` if there was none.
///
/// The returned pointer is valid until the next call on this handle or until
/// the handle is freed. **Do not** pass it to [`storelib_free_string`].
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn storelib_last_error(handle: *const StorelibHandle) -> *const c_char {
    if handle.is_null() {
        return std::ptr::null();
    }
    match &(*handle).last_error {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    }
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// Query the DisplayCatalog for a product.
///
/// `id_type` is one of the `STORELIB_ID_*` constants.
/// `auth_token` may be `NULL` for unauthenticated queries.
///
/// Returns [`STORELIB_OK`] on success or a negative error code on failure.
/// Call [`storelib_is_found`] afterwards to check whether the product exists.
///
/// # Safety
/// `handle` and `id` must be valid non-null pointers; `auth_token` may be null.
#[no_mangle]
pub unsafe extern "C" fn storelib_query(
    handle: *mut StorelibHandle,
    id: *const c_char,
    id_type: u32,
    auth_token: *const c_char,
) -> i32 {
    if handle.is_null() || id.is_null() {
        return STORELIB_ERR_NULL;
    }
    let h = &mut *handle;
    h.clear_error();

    let id_str = match CStr::from_ptr(id).to_str() {
        Ok(s) => s,
        Err(_) => {
            h.set_error("id is not valid UTF-8");
            return STORELIB_ERR_NULL;
        }
    };

    let token: Option<&str> = if auth_token.is_null() {
        None
    } else {
        match CStr::from_ptr(auth_token).to_str() {
            Ok(s) => Some(s),
            Err(_) => {
                h.set_error("auth_token is not valid UTF-8");
                return STORELIB_ERR_NULL;
            }
        }
    };

    let id_enum = id_type_from_u32(id_type);

    match h.rt.block_on(h.handler.query_dcat(id_str, id_enum, token)) {
        Ok(_) => STORELIB_OK,
        Err(e) => {
            let code = err_code(&e);
            h.set_error(&e.to_string());
            code
        }
    }
}

/// Returns `1` if the last [`storelib_query`] found the product, `0` otherwise.
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn storelib_is_found(handle: *const StorelibHandle) -> i32 {
    if handle.is_null() {
        return 0;
    }
    if (*handle).handler.is_found {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Product info
// ---------------------------------------------------------------------------

/// Return the product listing as a JSON string, or `NULL` if no product has
/// been queried yet.
///
/// The caller **must** free the returned string with [`storelib_free_string`].
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn storelib_product_json(handle: *const StorelibHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let listing = match &(*handle).handler.product_listing {
        Some(l) => l,
        None => return std::ptr::null_mut(),
    };
    match serde_json::to_string(listing) {
        Ok(json) => cstring_into_raw(json),
        Err(_) => std::ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Packages
// ---------------------------------------------------------------------------

/// Resolve and return the package list as a JSON array.
///
/// `msa_token` may be `NULL`.  The caller **must** free the returned string
/// with [`storelib_free_string`].
///
/// Returns `NULL` on error; inspect [`storelib_last_error`] for details.
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn storelib_packages_json(
    handle: *mut StorelibHandle,
    msa_token: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let h = &mut *handle;
    h.clear_error();

    let token: Option<&str> = if msa_token.is_null() {
        None
    } else {
        match CStr::from_ptr(msa_token).to_str() {
            Ok(s) => Some(s),
            Err(_) => {
                h.set_error("msa_token is not valid UTF-8");
                return std::ptr::null_mut();
            }
        }
    };

    match h.rt.block_on(h.handler.get_packages_for_product(token)) {
        Ok(pkgs) => match serde_json::to_string(&pkgs) {
            Ok(json) => cstring_into_raw(json),
            Err(e) => {
                h.set_error(&e.to_string());
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            h.set_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Search the catalog and return the results as a JSON string.
///
/// `family` is one of the `STORELIB_FAMILY_*` constants.
/// The caller **must** free the returned string with [`storelib_free_string`].
///
/// Returns `NULL` on error; inspect [`storelib_last_error`] for details.
///
/// # Safety
/// `handle` and `query` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn storelib_search_json(
    handle: *mut StorelibHandle,
    query: *const c_char,
    family: u32,
) -> *mut c_char {
    if handle.is_null() || query.is_null() {
        return std::ptr::null_mut();
    }
    let h = &mut *handle;
    h.clear_error();

    let query_str = match CStr::from_ptr(query).to_str() {
        Ok(s) => s,
        Err(_) => {
            h.set_error("query is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };

    let fam = family_from_u32(family);

    match h.rt.block_on(h.handler.search_dcat(query_str, fam)) {
        Ok(results) => match serde_json::to_string(&results) {
            Ok(json) => cstring_into_raw(json),
            Err(e) => {
                h.set_error(&e.to_string());
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            h.set_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Progress callback
// ---------------------------------------------------------------------------

/// C-compatible progress callback signature.
///
/// Fired during `storelib_query`, `storelib_packages_json`, and
/// `storelib_search_json` at each phase boundary. `stage` and `message` are
/// NUL-terminated UTF-8, valid only for the duration of the call. `has_current`
/// and `has_total` are `1` when `current`/`total` carry a meaningful counter
/// (e.g. "5 of 12 packages") and `0` otherwise. `user_data` is the opaque
/// pointer passed to [`storelib_set_progress_callback`].
pub type StorelibProgressCallback = extern "C" fn(
    stage: *const c_char,
    message: *const c_char,
    has_current: i32,
    current: u32,
    has_total: i32,
    total: u32,
    user_data: *mut c_void,
);

/// Install a progress callback. Pass `NULL` as `callback` to detach.
///
/// `user_data` is opaque and is passed back to the callback on every event;
/// it may be `NULL`. The callback is invoked from the Tokio runtime worker
/// thread that drives the active async call, so it must be thread-safe.
///
/// # Safety
/// `handle` must be a valid non-null pointer; `callback` must outlive any
/// pending in-flight `storelib_*` call on this handle.
#[no_mangle]
pub unsafe extern "C" fn storelib_set_progress_callback(
    handle: *mut StorelibHandle,
    callback: Option<StorelibProgressCallback>,
    user_data: *mut c_void,
) -> i32 {
    if handle.is_null() {
        return STORELIB_ERR_NULL;
    }
    let h = &mut *handle;
    match callback {
        Some(cb) => {
            // Carry user_data across the Send boundary as an integer; the
            // closure casts it back to *mut c_void on entry. Function pointers
            // are already Send + Sync.
            let user_data_addr = user_data as usize;
            h.handler
                .set_progress_callback(Box::new(move |e: ProgressEvent| {
                    let stage = CString::new(e.stage).unwrap_or_default();
                    let message = CString::new(e.message).unwrap_or_default();
                    let (has_c, cur) = match e.current {
                        Some(v) => (1, v),
                        None => (0, 0),
                    };
                    let (has_t, tot) = match e.total {
                        Some(v) => (1, v),
                        None => (0, 0),
                    };
                    (cb)(
                        stage.as_ptr(),
                        message.as_ptr(),
                        has_c,
                        cur,
                        has_t,
                        tot,
                        user_data_addr as *mut c_void,
                    );
                }));
        }
        None => h.handler.clear_progress_callback(),
    }
    STORELIB_OK
}

/// Detach the progress callback (equivalent to passing `NULL` to
/// [`storelib_set_progress_callback`]).
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn storelib_clear_progress_callback(handle: *mut StorelibHandle) -> i32 {
    if handle.is_null() {
        return STORELIB_ERR_NULL;
    }
    (*handle).handler.clear_progress_callback();
    STORELIB_OK
}

// ---------------------------------------------------------------------------
// String management
// ---------------------------------------------------------------------------

/// Free a string that was returned by this library.
///
/// Passing a string **not** allocated by this library is undefined behaviour.
/// Passing `NULL` is safe (no-op).
///
/// # Safety
/// `s` must be a pointer previously returned by a `storelib_*` function, or
/// `NULL`.
#[no_mangle]
pub unsafe extern "C" fn storelib_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn cstring_into_raw(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn id_type_from_u32(v: u32) -> IdentifierType {
    match v {
        1 => IdentifierType::XboxTitleId,
        2 => IdentifierType::PackageFamilyName,
        3 => IdentifierType::ContentId,
        4 => IdentifierType::LegacyWindowsPhoneProductId,
        5 => IdentifierType::LegacyWindowsStoreProductId,
        6 => IdentifierType::LegacyXboxProductId,
        _ => IdentifierType::ProductId,
    }
}

fn family_from_u32(v: u32) -> DeviceFamily {
    match v {
        1 => DeviceFamily::Mobile,
        2 => DeviceFamily::Xbox,
        3 => DeviceFamily::ServerCore,
        4 => DeviceFamily::IotCore,
        5 => DeviceFamily::HoloLens,
        6 => DeviceFamily::Andromeda,
        7 => DeviceFamily::Universal,
        8 => DeviceFamily::Wcos,
        _ => DeviceFamily::Desktop,
    }
}
