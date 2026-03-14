#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use clap::{Parser, Subcommand, ValueEnum};
    use storelib_rs::{DeviceFamily, DisplayCatalogHandler, IdentifierType};

    #[derive(Parser)]
    #[command(name = "storelib_rs", about = "Microsoft Store API client", version)]
    pub struct Cli {
        /// Logging verbosity (default: info)
        #[arg(long, global = true, default_value = "info", value_name = "LEVEL")]
        pub log_level: LogLevel,

        #[command(subcommand)]
        pub command: Command,
    }

    #[derive(ValueEnum, Clone)]
    pub enum LogLevel {
        Error,
        Warn,
        Info,
        Debug,
        Trace,
    }

    impl From<LogLevel> for log::LevelFilter {
        fn from(l: LogLevel) -> Self {
            match l {
                LogLevel::Error => log::LevelFilter::Error,
                LogLevel::Warn => log::LevelFilter::Warn,
                LogLevel::Info => log::LevelFilter::Info,
                LogLevel::Debug => log::LevelFilter::Debug,
                LogLevel::Trace => log::LevelFilter::Trace,
            }
        }
    }

    #[derive(Subcommand)]
    pub enum Command {
        /// Fetch direct download URLs for a product's packages
        Packages {
            /// Product ID (or other ID type, see --type)
            id: String,
            /// MSA authentication token
            #[arg(long)]
            token: Option<String>,
            /// Identifier type
            #[arg(long = "type", default_value = "product-id")]
            id_type: IdType,
        },
        /// Query detailed product information
        Query {
            /// The product identifier
            id: String,
            /// Identifier type
            #[arg(long = "type", default_value = "product-id")]
            id_type: IdType,
            /// MSA authentication token (for sandboxed/flighted listings)
            #[arg(long)]
            token: Option<String>,
        },
        /// Search the store catalog
        Search {
            /// Search query string
            query: String,
            /// Target device family
            #[arg(long, default_value = "desktop")]
            family: Family,
            /// Number of results to skip (for pagination)
            #[arg(long, default_value_t = 0)]
            skip: u32,
        },
    }

    #[derive(ValueEnum, Clone)]
    pub enum IdType {
        ProductId,
        Pfn,
        ContentId,
        XboxTitleId,
        LegacyPhone,
        LegacyStore,
        LegacyXbox,
    }

    impl From<IdType> for IdentifierType {
        fn from(t: IdType) -> Self {
            match t {
                IdType::ProductId => IdentifierType::ProductId,
                IdType::Pfn => IdentifierType::PackageFamilyName,
                IdType::ContentId => IdentifierType::ContentId,
                IdType::XboxTitleId => IdentifierType::XboxTitleId,
                IdType::LegacyPhone => IdentifierType::LegacyWindowsPhoneProductId,
                IdType::LegacyStore => IdentifierType::LegacyWindowsStoreProductId,
                IdType::LegacyXbox => IdentifierType::LegacyXboxProductId,
            }
        }
    }

    #[derive(ValueEnum, Clone)]
    pub enum Family {
        Desktop,
        Mobile,
        Xbox,
        Universal,
        Holographic,
        Iot,
        Server,
        Andromeda,
        Wcos,
    }

    impl From<Family> for DeviceFamily {
        fn from(f: Family) -> Self {
            match f {
                Family::Desktop => DeviceFamily::Desktop,
                Family::Mobile => DeviceFamily::Mobile,
                Family::Xbox => DeviceFamily::Xbox,
                Family::Universal => DeviceFamily::Universal,
                Family::Holographic => DeviceFamily::HoloLens,
                Family::Iot => DeviceFamily::IotCore,
                Family::Server => DeviceFamily::ServerCore,
                Family::Andromeda => DeviceFamily::Andromeda,
                Family::Wcos => DeviceFamily::Wcos,
            }
        }
    }

    pub async fn run(cli: Cli) {
        match cli.command {
            Command::Packages { id, token, id_type } => {
                log::info!("Command: packages id={id}");
                let mut handler = DisplayCatalogHandler::production();
                if let Err(e) = handler.query_dcat(&id, id_type.into(), None).await {
                    log::error!("Error querying product: {e}");
                    return;
                }
                match handler.get_packages_for_product(token.as_deref()).await {
                    Ok(pkgs) if pkgs.is_empty() => log::info!("No packages found."),
                    Ok(pkgs) => {
                        log::info!("Found {} package(s):", pkgs.len());
                        for pkg in &pkgs {
                            log::info!("  {} [{:?}]", pkg.package_moniker, pkg.package_type);
                            if let Some(uri) = &pkg.package_uri {
                                log::info!("    URL: {uri}");
                            }
                        }
                    }
                    Err(e) => log::error!("Error fetching packages: {e}"),
                }
            }

            Command::Query { id, id_type, token } => {
                log::info!("Command: query id={id}");
                let mut handler = DisplayCatalogHandler::production();
                match handler
                    .query_dcat(&id, id_type.into(), token.as_deref())
                    .await
                {
                    Ok(_) => {
                        let listing = handler.product_listing.as_ref().unwrap();
                        let product = listing
                            .products
                            .as_deref()
                            .and_then(|v| v.first())
                            .or(listing.product.as_ref());

                        match product {
                            Some(p) => {
                                let title = p
                                    .localized_properties
                                    .as_deref()
                                    .and_then(|v| v.first())
                                    .and_then(|lp| lp.product_title.as_deref())
                                    .unwrap_or("<no title>");
                                let kind = p.product_kind.as_deref().unwrap_or("<unknown>");
                                let pfn = p
                                    .properties
                                    .as_ref()
                                    .and_then(|pr| pr.package_family_name.as_deref())
                                    .unwrap_or("<none>");
                                log::info!("Title:  {title}");
                                log::info!("Kind:   {kind}");
                                log::info!("PFN:    {pfn}");
                            }
                            None => log::info!("Product found but no details available."),
                        }
                    }
                    Err(e) => log::error!("Error: {e}"),
                }
            }

            Command::Search {
                query,
                family,
                skip,
            } => {
                log::info!("Command: search query=\"{query}\"");
                let mut handler = DisplayCatalogHandler::production();
                let result = if skip > 0 {
                    handler.search_dcat_paged(&query, family.into(), skip).await
                } else {
                    handler.search_dcat(&query, family.into()).await
                };

                match result {
                    Ok(results) => {
                        log::info!("Total results: {}", results.total_result_count.unwrap_or(0));
                        if let Some(groups) = &results.results {
                            for group in groups {
                                let fam =
                                    group.product_family_name.as_deref().unwrap_or("<unknown>");
                                log::info!("  Family: {fam}");
                                if let Some(products) = &group.products {
                                    for p in products.iter().take(10) {
                                        let title = p
                                            .localized_properties
                                            .as_deref()
                                            .and_then(|v| v.first())
                                            .and_then(|lp| lp.product_title.as_deref())
                                            .unwrap_or("<no title>");
                                        log::info!("    - {title}");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("Search error: {e}"),
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    use clap::Parser;

    let cli = cli::Cli::parse();

    env_logger::Builder::new()
        .filter_level(cli.log_level.clone().into())
        .parse_default_env() // RUST_LOG still overrides --log-level
        .format_timestamp_millis()
        .init();

    log::debug!("storelib_rs starting");
    cli::run(cli).await;
}
