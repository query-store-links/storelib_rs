#!/usr/bin/env bash
# Build storelib_rs as a universal wasm-bindgen npm package.
#
# Builds three wasm-pack outputs (nodejs, web, bundler) and assembles them
# into a single npm package at ./pkg with conditional `exports` so the same
# @<scope>/storelib_rs works in Node.js, browsers, and bundlers.
#
# Usage:
#   tools/pack-npm.sh [--out-dir pkg] [--pack] [--profile release|dev] [--scope <scope>]
#
# Defaults:
#   --out-dir pkg
#   --profile release
#   --scope query-store-links  (pass --scope '' to publish unscoped)

set -euo pipefail

out_dir="pkg"
do_pack=0
profile="release"
scope="query-store-links"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out-dir)   out_dir="$2";    shift 2 ;;
        --out-dir=*) out_dir="${1#*=}"; shift ;;
        --pack)      do_pack=1;       shift   ;;
        --profile)   profile="$2";    shift 2 ;;
        --profile=*) profile="${1#*=}"; shift ;;
        --scope)     scope="$2";      shift 2 ;;
        --scope=*)   scope="${1#*=}"; shift ;;
        -h|--help)
            sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "Unknown argument: $1" >&2; exit 2 ;;
    esac
done

case "$profile" in
    release) profile_flag="--release" ;;
    dev)     profile_flag="--dev" ;;
    *) echo "Invalid --profile: $profile (expected release|dev)" >&2; exit 2 ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
cd "$repo_root"

command -v cargo >/dev/null 2>&1 || { echo "cargo not found on PATH. Install Rust via https://rustup.rs/." >&2; exit 1; }
command -v node  >/dev/null 2>&1 || { echo "node not found on PATH (required for package assembly)." >&2; exit 1; }

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "wasm-pack not found. Installing via 'cargo install wasm-pack'..."
    cargo install wasm-pack
fi

if command -v rustup >/dev/null 2>&1; then
    if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
        echo "Adding wasm32-unknown-unknown target..."
        rustup target add wasm32-unknown-unknown
    fi
fi

scope_args=()
[[ -n "$scope" ]] && scope_args=(--scope "$scope")

abs_out_dir="$repo_root/$out_dir"
mkdir -p "$abs_out_dir"

for t in nodejs web bundler; do
    target_dir="$abs_out_dir/$t"
    echo "==> Building target '$t' -> $target_dir"
    wasm-pack build \
        ${scope_args[@]+"${scope_args[@]}"} \
        --target "$t" \
        --out-dir "$target_dir" \
        "$profile_flag" \
        -- --features wasm
done

echo "==> Assembling universal package at $abs_out_dir"
node "$script_dir/assemble-pkg.mjs" "$out_dir"

if [[ "$do_pack" -eq 1 ]]; then
    command -v npm >/dev/null 2>&1 || { echo "npm not found on PATH (required for --pack)." >&2; exit 1; }
    echo "==> Packing $abs_out_dir"
    ( cd "$abs_out_dir" && npm pack )
fi

echo
echo "Done. Universal package at: $abs_out_dir"
if [[ "$do_pack" -eq 1 ]]; then
    find "$abs_out_dir" -maxdepth 1 -name '*.tgz' -print | sed 's/^/  /'
fi
