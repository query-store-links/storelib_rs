# storelib_rs

A Rust port of [StoreLib](https://github.com/query-store-links/StoreLib) — a library for interacting with Microsoft Store endpoints to query product information and resolve package download URLs.

Supports native (tokio) and **WebAssembly** targets.

## Features

- Query the Microsoft Store Display Catalog by Product ID, Package Family Name, Xbox Title ID, and more
- Resolve direct `.appx` / `.msix` / `.eappx` download URLs via the FE3 delivery service
- Search the catalog by query string and device family
- 7 endpoints (Production, Int, Xbox, XboxInt, Dev, OneP, OnePInt)
- 250+ markets and 150+ languages via `Locale`
- Optional MSA / XBL3.0 authentication token support for sandboxed and flighted listings
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

let locale = Locale::new(Market::Gb, Lang::EnGb, true);
let mut handler = DisplayCatalogHandler::new(DCatEndpoint::Production, locale);
```

### Authentication (sandboxed / flighted listings)

```rust
// MSA token or XBL3.0 token
handler.query_dcat("9wzdncrfj3tj", IdentifierType::ProductId, Some("your-token")).await?;

// Authenticated package resolution
let packages = handler.get_packages_for_product(Some("your-token")).await?;
```

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

## Building

```sh
# Native
cargo build --release

# WebAssembly
cargo build --target wasm32-unknown-unknown --features wasm
```

## License

This program is free software: you can redistribute it and/or modify it under the terms of the
[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html) as published by the
Free Software Foundation.

See [LICENSE](LICENSE) for the full text.
