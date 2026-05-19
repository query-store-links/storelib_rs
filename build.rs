// Build-time TypeScript generation for the wasm-bindgen typescript_custom_section.
//
// When building with `--features wasm` for `wasm32-unknown-unknown`, this
// runs `node tools/gen-ts.mjs $OUT_DIR/wasm_types.d.ts`, producing the .d.ts
// fragment that wasm.rs embeds via `include_str!`. On native builds (no wasm
// feature) the generator is skipped — Node isn't required.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // The TypeScript section only matters for wasm-bindgen builds.
    let wasm_feature = env::var("CARGO_FEATURE_WASM").is_ok();
    let target_wasm = env::var("CARGO_CFG_TARGET_ARCH")
        .map(|s| s == "wasm32")
        .unwrap_or(false);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let dest = out_dir.join("wasm_types.d.ts");

    // Re-run when any Rust file the generator parses changes, or the generator itself.
    for path in [
        "tools/gen-ts.mjs",
        "src/models/enums.rs",
        "src/models/fe3.rs",
        "src/models/locale.rs",
        "src/models/search.rs",
        "src/models/catalog.rs",
        "src/services/display_catalog.rs",
        "src/wasm.rs",
        "src/error.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-changed=build.rs");

    if !(wasm_feature && target_wasm) {
        // wasm.rs isn't compiled in this configuration, so we still need the
        // file to exist (include_str! is checked even when its caller is
        // cfg'd out, depending on rustc version) — write an empty stub.
        std::fs::write(&dest, "// wasm feature not enabled\n").expect("write stub");
        return;
    }

    let node = env::var("NODE").unwrap_or_else(|_| "node".to_string());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let script = PathBuf::from(&manifest_dir).join("tools/gen-ts.mjs");

    let status = Command::new(&node)
        .arg(&script)
        .arg(&dest)
        .current_dir(&manifest_dir)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "failed to invoke `{} {} {}`: {e}\n\
                 The wasm build needs Node.js on PATH to regenerate the \
                 TypeScript type declarations. Install Node or unset the \
                 wasm feature.",
                node,
                script.display(),
                dest.display(),
            )
        });
    if !status.success() {
        panic!(
            "gen-ts.mjs exited with {status:?} while writing {}",
            dest.display(),
        );
    }
}
