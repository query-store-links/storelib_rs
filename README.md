# storelib_rs

A Rust port of [StoreLib](https://github.com/query-store-links/StoreLib) — a library for interacting with Microsoft Store endpoints to query product information and resolve package download URLs.

Supports native (tokio), **WebAssembly**, and **C / FFI** targets.

## Features

- Query the Microsoft Store Display Catalog by Product ID, Package Family Name, Xbox Title ID, and more
- Resolve direct `.appx` / `.msix` / `.eappx` download URLs via the FE3 delivery service
- Search the catalog by query string and device family
- 7 endpoints (Production, Int, Xbox, XboxInt, Dev, OneP, OnePInt)
- 259 markets + 185 ISO 639-1 languages + 350 Microsoft Store BCP-47 language tags (sourced from the IANA registry + `learn.microsoft.com`'s supported-languages table)
- `Locale::from_tag("en-US")` / `Locale.fromTag("zh-Hant-TW")` builds a locale directly from a BCP-47 tag
- Optional MSA / XBL3.0 authentication token support for sandboxed and flighted listings
- **Real-time progress reporting** via a callback on every platform (native closure / JS function / C function pointer)
- **Cancellation** via `AbortSignal` (WASM) / `StoreError::Cancelled` (native)
- **Structured errors** in JS — thrown `Error` objects carry a `kind` discriminant (`"http" | "json" | "xml" | "notFound" | "timedOut" | "cancelled" | "other"`)
- camelCase JSON wire format across all bindings; PascalCase from the upstream MS Store API is accepted on the way in
- Structured logging via the `log` facade (`env_logger` on native, pluggable on WASM)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
storelib_rs = { git = "https://github.com/query-store-links/storelib_rs" }
```

For WebAssembly, enable the `wasm` feature:

```toml
[dependencies]
storelib_rs = { git = "https://github.com/query-store-links/storelib_rs", features = ["wasm"] }
```

## Library usage

### Query a product

```rust
use storelib_rs::{DisplayCatalogHandler, IdentifierType};

let mut handler = DisplayCatalogHandler::production();
handler.query_dcat("9wzdncrfj3tj", IdentifierType::ProductId, None).await?;

if handler.is_found {
    let listing = handler.product_listing.as_ref().unwrap();
    // listing.products / listing.product contain the full catalog model
}
```

### Resolve package download URLs

```rust
let packages = handler.get_packages_for_product(None).await?;
for pkg in &packages {
    println!("{} [{:?}]", pkg.package_moniker, pkg.package_type);
    if let Some(url) = &pkg.package_uri {
        println!("  {url}");
    }
}
```

### Search the catalog

```rust
use storelib_rs::DeviceFamily;

let results = handler.search_dcat("netflix", DeviceFamily::Desktop).await?;
println!("Total: {}", results.total_result_count.unwrap_or(0));
```

### Custom locale and endpoint

```rust
use storelib_rs::{DCatEndpoint, DisplayCatalogHandler, Lang, Locale, Market};

let locale = Locale::new(Market::Gb, Lang::En, true);
let mut handler = DisplayCatalogHandler::new(DCatEndpoint::Production, locale);
```

### Building a locale from a BCP-47 tag

```rust
use storelib_rs::{LanguageTag, Locale};
use std::str::FromStr;

// "zh-Hant-TW" → Locale { market: TW, language: zh, include_neutral: true }
let tag = LanguageTag::from_str("zh-Hant-TW")?;
let locale = Locale::from_tag(tag, true)?;
```

### Authentication (sandboxed / flighted listings)

```rust
// MSA token or XBL3.0 token
handler.query_dcat("9wzdncrfj3tj", IdentifierType::ProductId, Some("your-token")).await?;

// Authenticated package resolution
let packages = handler.get_packages_for_product(Some("your-token")).await?;
```

### Progress reporting

```rust
use storelib_rs::ProgressEvent;

handler.set_progress_callback(Box::new(|e: ProgressEvent| {
    eprintln!("[{}] {} ({:?}/{:?})", e.stage, e.message, e.current, e.total);
}));
```

Stages emitted during `query_dcat` / `get_packages_for_product` / `search_dcat`:

| Operation             | Stages |
| --------------------- | ------ |
| `query_dcat`          | `dcat.request` → `dcat.response` → `dcat.parse` → `dcat.done` (or `dcat.notFound`) |
| `get_packages_for_product` | `fe3.start` → `fe3.getCookie` → `fe3.syncUpdates` → `fe3.parseUpdateIds`[`.done`] → `fe3.parsePackages`[`.done`] → `fe3.resolveUrls`[`.done`] → `fe3.done` |
| `search_dcat`         | `search.request` → `search.response` → `search.parse` → `search.done` |

`.done` counter stages populate `current`/`total` with `N` of `N` items processed.

## JavaScript / WebAssembly usage

```js
import {
    DisplayCatalogHandler,
    Locale,
    parseLanguageTag,
    listLanguageTags,
} from 'storelib_rs';

// Build a locale directly from a BCP-47 tag.
const locale = Locale.fromTag('en-US', /* includeNeutral */ false);
const handler = new DisplayCatalogHandler('production', locale);

// Subscribe to per-stage progress updates.
handler.onProgress(({stage, message, current, total}) => {
    console.log(`[${stage}] ${message}` + (total != null ? ` (${current}/${total})` : ''));
});

// Cancel a stalled call after 5s.
const ctrl = new AbortController();
setTimeout(() => ctrl.abort(), 5000);

try {
    await handler.queryDcat('9WZDNCRFJ3TJ', 'ProductId', null, ctrl.signal);
    const pkgs = await handler.getPackagesForProduct(null, ctrl.signal);
    // pkgs: Array<{ packageMoniker, packageUri, packageType, applicabilityBlob, updateId, packageSize }>
} catch (e) {
    if (e.kind === 'cancelled') return;
    if (e.kind === 'notFound') showNotFound();
    else console.error(e.kind, e.message);
}

// Enumerate the full Microsoft Store BCP-47 tag list.
for (const {code, englishName} of listLanguageTags()) {
    console.log(code, englishName);
}
```

The `idType` argument to `queryDcat` is tolerant — `"ProductId"`, `"productId"`, `"product-id"`, and `"PRODUCT_ID"` all resolve to the same enum.

## CLI usage

```
storelib_rs [--log-level <LEVEL>] <COMMAND>

Commands:
  packages  Fetch direct download URLs for a product's packages
  query     Query detailed product information
  search    Search the store catalog

Options:
  --log-level <LEVEL>  error | warn | info | debug | trace  [default: info]
```

### Examples

```sh
# Resolve all package URLs for an app
storelib_rs packages 9wzdncrfj3tj

# Query by Package Family Name
storelib_rs query NETFLIX.APP_mcm4njqhnhss8 --type pfn

# Search for Xbox games
storelib_rs search halo --family xbox

# Authenticated query (flighted / sandbox)
storelib_rs packages 9wzdncrfj3tj --token "XBL3.0 x=..."

# Debug logging (shows HTTP requests, FE3 update IDs, URL resolution)
storelib_rs packages 9wzdncrfj3tj --log-level debug

# Trace logging (also dumps raw SOAP response bodies)
storelib_rs packages 9wzdncrfj3tj --log-level trace
```

`RUST_LOG` overrides `--log-level`:

```sh
RUST_LOG=storelib_rs=debug storelib_rs packages 9wzdncrfj3tj
```

## Identifier types

| `--type` value | Description |
|---|---|
| `product-id` *(default)* | Microsoft Store Product ID (e.g. `9wzdncrfj3tj`) |
| `pfn` | Package Family Name |
| `content-id` | Content ID |
| `xbox-title-id` | Xbox Title ID |
| `legacy-phone` | Legacy Windows Phone Product ID |
| `legacy-store` | Legacy Windows Store Product ID |
| `legacy-xbox` | Legacy Xbox Product ID |

## C / FFI usage

Build the shared library with `cargo build --release --features ffi`. The C header is at `include/storelib_rs.h`. See the header for the full API; minimal example:

```c
#include "storelib_rs.h"
#include <stdio.h>

static void on_progress(
    const char* stage,
    const char* message,
    int32_t has_current, uint32_t current,
    int32_t has_total,   uint32_t total,
    void* user_data
) {
    (void)user_data;
    if (has_total) {
        printf("[%s] %s (%u/%u)\n", stage, message, current, total);
    } else {
        printf("[%s] %s\n", stage, message);
    }
}

int main(void) {
    StorelibHandle* h = storelib_new();
    storelib_set_progress_callback(h, on_progress, NULL);

    int32_t rc = storelib_query(h, "9wzdncrfj3tj", STORELIB_ID_PRODUCT_ID, NULL);
    if (rc == STORELIB_OK && storelib_is_found(h)) {
        char* json = storelib_packages_json(h, NULL);
        if (json) {
            puts(json);
            storelib_free_string(json);
        }
    } else {
        fprintf(stderr, "query failed (%d): %s\n", rc, storelib_last_error(h));
    }
    storelib_free(h);
    return 0;
}
```

## Building

```sh
# Native
cargo build --release

# WebAssembly
cargo build --target wasm32-unknown-unknown --features wasm

# C / FFI shared library + header
cargo build --release --features ffi
# Header is at include/storelib_rs.h
```

## License

This program is free software: you can redistribute it and/or modify it under the terms of the
[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html) as published by the
Free Software Foundation.

See [LICENSE](LICENSE) for the full text.
