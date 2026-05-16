#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use std::io;
    use std::path::{Path, PathBuf};

    use clap::{Parser, Subcommand, ValueEnum};
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use storelib_rs::{DeviceFamily, DisplayCatalogHandler, IdentifierType, PackageInstance};
    use tokio::io::AsyncWriteExt;

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
        /// Resolve a product's packages and stream them to disk
        Download {
            /// Product ID (or other ID type, see --type)
            id: String,
            /// MSA authentication token
            #[arg(long)]
            token: Option<String>,
            /// Identifier type
            #[arg(long = "type", default_value = "product-id")]
            id_type: IdType,
            /// Output directory (default: current dir). Created if missing.
            #[arg(long, short = 'o', default_value = ".")]
            out: PathBuf,
            /// Skip framework / dependency packages (Microsoft.VCLibs,
            /// Microsoft.NET.Native.*, Microsoft.UI.Xaml, etc.)
            #[arg(long)]
            skip_framework: bool,
            /// Overwrite existing files. Without this flag, files that
            /// already exist at the destination are skipped.
            #[arg(long)]
            force: bool,
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
                            if let Some(size) = pkg.file_size {
                                log::info!("    Size: {size} bytes");
                            }
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

            Command::Download {
                id,
                token,
                id_type,
                out,
                skip_framework,
                force,
            } => {
                log::info!("Command: download id={id}");
                let mut handler = DisplayCatalogHandler::production();
                if let Err(e) = handler.query_dcat(&id, id_type.into(), None).await {
                    log::error!("query_dcat failed: {e}");
                    return;
                }
                let packages = match handler.get_packages_for_product(token.as_deref()).await {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("get_packages_for_product failed: {e}");
                        return;
                    }
                };

                if let Err(e) = tokio::fs::create_dir_all(&out).await {
                    log::error!("creating output dir {}: {e}", out.display());
                    return;
                }

                let client = reqwest::Client::builder()
                    .user_agent("StoreLib")
                    .build()
                    .unwrap_or_default();

                let mut planned: Vec<_> = packages
                    .iter()
                    .filter(|p| p.package_uri.is_some())
                    .collect();
                if skip_framework {
                    planned.retain(|p| !is_framework(&p.package_moniker));
                }

                if planned.is_empty() {
                    log::warn!("No downloadable packages for {id}.");
                    return;
                }

                log::info!(
                    "Downloading {} package(s) to {}",
                    planned.len(),
                    out.display(),
                );

                let mp = MultiProgress::new();
                let bar_style = ProgressStyle::with_template(
                    "{prefix:>3} [{bar:40.cyan/blue}] {bytes:>10}/{total_bytes:>10} {bytes_per_sec:>11} {wide_msg}",
                )
                .expect("valid progress template")
                .progress_chars("=> ");

                let mut errors = 0u32;
                let mut skipped = 0u32;
                for (i, pkg) in planned.iter().enumerate() {
                    let uri = pkg.package_uri.as_deref().unwrap();
                    let filename = filename_for_package(pkg);
                    let dest = out.join(&filename);

                    if !force && tokio::fs::try_exists(&dest).await.unwrap_or(false) {
                        log::info!("[{}/{}] skip (exists): {filename}", i + 1, planned.len());
                        skipped += 1;
                        continue;
                    }

                    let total = pkg.file_size.map(|s| s.max(0) as u64).unwrap_or(0);
                    let pb = mp.add(ProgressBar::new(total));
                    pb.set_style(bar_style.clone());
                    pb.set_prefix(format!("{}/{}", i + 1, planned.len()));
                    pb.set_message(filename.clone());

                    match download_one(&client, uri, &dest, &pb).await {
                        Ok(bytes) => {
                            pb.finish_with_message(format!("{filename}  ✓ {bytes} bytes"));
                        }
                        Err(e) => {
                            pb.abandon_with_message(format!("{filename}  ✗ {e}"));
                            errors += 1;
                            // Best-effort cleanup of the partial file.
                            let _ = tokio::fs::remove_file(&dest).await;
                        }
                    }
                }

                if errors == 0 {
                    log::info!(
                        "Done. {} downloaded, {} skipped.",
                        planned.len() - skipped as usize,
                        skipped,
                    );
                } else {
                    log::error!("{errors} of {} download(s) failed.", planned.len(),);
                    std::process::exit(1);
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

    // ---------------------------------------------------------------------
    // Download helpers (CLI-only — kept out of the library on purpose)
    // ---------------------------------------------------------------------

    /// Stream one HTTP body into `dest`, advancing `pb` per chunk. Returns
    /// the total number of bytes written on success.
    async fn download_one(
        client: &reqwest::Client,
        uri: &str,
        dest: &Path,
        pb: &ProgressBar,
    ) -> io::Result<u64> {
        let mut response = client.get(uri).send().await.map_err(io::Error::other)?;

        let status = response.status();
        if !status.is_success() {
            return Err(io::Error::other(format!("HTTP {status}")));
        }

        // Prefer Content-Length when the package didn't ship a `packageSize`.
        if pb.length().unwrap_or(0) == 0 {
            if let Some(len) = response.content_length() {
                pb.set_length(len);
            }
        }

        let mut file = tokio::fs::File::create(dest).await?;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = response.chunk().await.map_err(io::Error::other)? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }
        file.flush().await?;
        Ok(downloaded)
    }

    /// Take the library's `readable_file_name` and make it safe to write to
    /// disk on Windows (strip the reserved character set).
    fn filename_for_package(pkg: &PackageInstance) -> String {
        sanitize_filename(&pkg.readable_file_name)
    }

    fn sanitize_filename(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                // Strip every separator + reserved character on Windows.
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect()
    }

    /// Known framework / dependency package families that are part of
    /// almost every MS Store app and rarely what callers want to download.
    fn is_framework(package_moniker: &str) -> bool {
        // Moniker form: "Microsoft.VCLibs.140.00_14.0.30704.0_x64__8wekyb3d8bbwe"
        // — take everything up to the first underscore.
        let family = package_moniker.split('_').next().unwrap_or(package_moniker);
        let family_lc = family.to_lowercase();
        const KNOWN_FRAMEWORKS: &[&str] = &[
            "microsoft.vclibs",
            "microsoft.net.native.framework",
            "microsoft.net.native.runtime",
            "microsoft.netcore.universalwindowsplatform",
            "microsoft.ui.xaml",
            "microsoft.directx",
            "microsoft.services.store.engagement",
        ];
        KNOWN_FRAMEWORKS
            .iter()
            .any(|prefix| family_lc.starts_with(prefix))
    }

    // ---------------------------------------------------------------------
    // Helper tests
    // ---------------------------------------------------------------------

    #[cfg(test)]
    mod helper_tests {
        use super::*;
        use storelib_rs::PackageType;

        fn pkg(moniker: &str, file_name: Option<&str>) -> PackageInstance {
            PackageInstance {
                package_moniker: moniker.into(),
                package_uri: None,
                package_type: PackageType::AppX,
                applicability_blob: None,
                update_id: String::new(),
                file_size: None,
                file_name: file_name.map(str::to_owned),
                readable_file_name: PackageInstance::build_readable_file_name(moniker, file_name),
            }
        }

        #[test]
        fn filename_for_package_passes_through_readable() {
            assert_eq!(
                filename_for_package(&pkg(
                    "4DF9E0F8.Netflix_8.156.0.0_neutral_~_mcm4njqhnhss8",
                    Some("1b599478-061e-438e-88e1-f8c4de1670d4.appxbundle"),
                )),
                "4DF9E0F8.Netflix_8.156.0.0_neutral_~_mcm4njqhnhss8.appxbundle",
            );
        }

        #[test]
        fn filename_for_package_sanitizes_windows_reserved_chars() {
            // The library doesn't sanitise (different OSes have different
            // reserved sets) — the CLI does.
            let cleaned = filename_for_package(&pkg("Foo:Bar*1.0", Some("x.appx")));
            assert!(!cleaned.contains(':'));
            assert!(!cleaned.contains('*'));
            assert!(cleaned.ends_with(".appx"));
        }

        #[test]
        fn is_framework_matches_known_prefixes() {
            assert!(is_framework(
                "Microsoft.VCLibs.140.00_14.0_x64__8wekyb3d8bbwe"
            ));
            assert!(is_framework(
                "Microsoft.NET.Native.Framework.2.2_2.2_x64__hash"
            ));
            assert!(is_framework("Microsoft.UI.Xaml.2.8_8.2_x64__hash"));
            assert!(is_framework("microsoft.vclibs.140.00_lowercase"));
        }

        #[test]
        fn is_framework_rejects_app_packages() {
            assert!(!is_framework("4DF9E0F8.Netflix_8.1_x64__mcm4njqhnhss8"));
            assert!(!is_framework("Spotify.Spotify_1.0_x64__hash"));
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
