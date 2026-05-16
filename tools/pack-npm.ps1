<#
.SYNOPSIS
    Build storelib_rs as a universal wasm-bindgen npm package.

.DESCRIPTION
    Builds three wasm-pack outputs (nodejs, web, bundler) and assembles them
    into a single npm package at ./pkg with conditional `exports` so the same
    `@<scope>/storelib_rs` works in Node.js, browsers, and bundlers.

    Requires:
      - rustup (for `wasm32-unknown-unknown` target installation)
      - wasm-pack on PATH (the script offers to `cargo install` it if missing)
      - Node.js on PATH (for the package-assembly step)

.PARAMETER OutDir
    Top-level output directory. Defaults to "pkg" at the repo root.
    Per-target subdirectories live at "<OutDir>/{nodejs,web,bundler}".

.PARAMETER Pack
    If set, runs `npm pack` in the assembled package, producing a single .tgz.

.PARAMETER Profile
    Build profile: release (default) or dev.

.PARAMETER Scope
    npm scope (without the leading '@'). Defaults to "query-store-links",
    producing "@query-store-links/storelib_rs". Pass an empty string to publish
    unscoped.

.EXAMPLE
    ./tools/pack-npm.ps1
    Builds all three targets and assembles ./pkg as a universal npm package.

.EXAMPLE
    ./tools/pack-npm.ps1 -Pack
    As above, plus produces ./pkg/<name>-<version>.tgz ready for `npm publish`.
#>
[CmdletBinding()]
param(
    [string]$OutDir = 'pkg',

    [switch]$Pack,

    [ValidateSet('release', 'dev')]
    [string]$Profile = 'release',

    [string]$Scope = 'query-store-links'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Test-Command([string]$Name) {
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

if (-not (Test-Command 'cargo')) { throw "cargo not found on PATH. Install Rust via https://rustup.rs/." }
if (-not (Test-Command 'node'))  { throw "node not found on PATH (required for package assembly)." }

if (-not (Test-Command 'wasm-pack')) {
    Write-Host "wasm-pack not found. Installing via 'cargo install wasm-pack'..." -ForegroundColor Yellow
    & cargo install wasm-pack
    if ($LASTEXITCODE -ne 0) { throw "cargo install wasm-pack failed." }
}

$installedTargets = & rustup target list --installed 2>$null
if ($installedTargets -notcontains 'wasm32-unknown-unknown') {
    Write-Host "Adding wasm32-unknown-unknown target..." -ForegroundColor Yellow
    & rustup target add wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw "rustup target add failed." }
}

$absOutDir = Join-Path $repoRoot $OutDir
New-Item -ItemType Directory -Force -Path $absOutDir | Out-Null

$profileFlag = if ($Profile -eq 'dev') { '--dev' } else { '--release' }
$scopeArgs = @()
if ($Scope) { $scopeArgs = @('--scope', $Scope) }

foreach ($t in @('nodejs', 'web', 'bundler')) {
    $targetDir = Join-Path $absOutDir $t
    Write-Host "==> Building target '$t' -> $targetDir" -ForegroundColor Cyan

    & wasm-pack build `
        @scopeArgs `
        --target $t `
        --out-dir $targetDir `
        $profileFlag `
        -- --features wasm
    if ($LASTEXITCODE -ne 0) { throw "wasm-pack build ($t) failed." }
}

Write-Host "==> Assembling universal package at $absOutDir" -ForegroundColor Cyan
& node (Join-Path $PSScriptRoot 'assemble-pkg.mjs') $OutDir
if ($LASTEXITCODE -ne 0) { throw "Package assembly failed." }

if ($Pack) {
    if (-not (Test-Command 'npm')) { throw "npm not found on PATH (required for -Pack)." }
    Write-Host "==> Packing $absOutDir" -ForegroundColor Cyan
    Push-Location $absOutDir
    try {
        & npm pack
        if ($LASTEXITCODE -ne 0) { throw "npm pack failed." }
    } finally {
        Pop-Location
    }
}

Write-Host ""
Write-Host "Done. Universal package at: $absOutDir" -ForegroundColor Green
if ($Pack) {
    Get-ChildItem -Path $absOutDir -Filter '*.tgz' |
        ForEach-Object { Write-Host "  $($_.FullName)" }
}
