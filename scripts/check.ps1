# Single source of truth for local verification.
#
# CI invokes this script, and a developer runs the same script. If the two ever
# diverge, the CI definition is wrong, not this file.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/check.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/check.ps1 -Release

[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$SkipDeferredGuard
)

$ErrorActionPreference = 'Stop'
$failed = @()

function Write-Section([string]$name) {
    Write-Host ''
    Write-Host "=== $name ===" -ForegroundColor Cyan
}

# ---------------------------------------------------------------------------
# Resolve cargo. Rust is not always on PATH on the development host, so fall
# back to the rustup install location rather than assuming ambient state.
# ---------------------------------------------------------------------------
$cargo = $null
$cmd = Get-Command cargo -ErrorAction SilentlyContinue
if ($cmd) {
    $cargo = $cmd.Source
} elseif (Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe") {
    $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

if (-not $cargo) {
    Write-Error 'cargo not found on PATH and not present at %USERPROFILE%\.cargo\bin\cargo.exe'
    exit 127
}

Write-Host "cargo: $cargo"
& $cargo --version
Write-Host "rustc: $((& rustc --version 2>&1))"

# ---------------------------------------------------------------------------
# Gate 0 discipline guard: deferred crates must not exist as placeholders.
# A planned module is not evidence that code exists, and an empty crate that
# claims to exist is worse than an absent one. See docs/ADR/0006.
# ---------------------------------------------------------------------------
if (-not $SkipDeferredGuard) {
    Write-Section 'Gate 0 deferred-crate guard'
    $deferred = @(
        'crates/dd-audio',
        'crates/dd-dsp',
        'crates/dd-analysis',
        'crates/dd-renderer',
        'crates/dd-package',
        'crates/dd-steam',
        'crates/dd-desktop-automation'
    )
    $present = @($deferred | Where-Object { Test-Path $_ })
    if ($present.Count -gt 0) {
        Write-Host "FAIL: deferred crates present at Gate 0:" -ForegroundColor Red
        $present | ForEach-Object { Write-Host "  - $_" }
        Write-Host 'Either implement and test them for their own gate, or remove the placeholder.'
        $failed += 'deferred-crate guard'
    } else {
        Write-Host 'ok: no deferred crate directories present'
    }

    # The Windows desktop-host adapter belongs to Gate 3.
    if (Test-Path 'crates/dd-platform/src/windows') {
        Write-Host 'FAIL: crates/dd-platform/src/windows exists before Gate 3' -ForegroundColor Red
        $failed += 'deferred-crate guard (platform windows)'
    }

    # ADR-0009: no unbounded channel may be introduced anywhere in the engine.
    $unbounded = @(Get-ChildItem crates, apps -Recurse -Filter *.rs -ErrorAction SilentlyContinue |
        Select-String -Pattern 'unbounded\s*\(' )
    if ($unbounded.Count -gt 0) {
        Write-Host 'FAIL: unbounded channel found (see docs/ADR/0009)' -ForegroundColor Red
        $unbounded | ForEach-Object { Write-Host "  $($_.Path):$($_.LineNumber)" }
        $failed += 'unbounded-channel guard'
    } else {
        Write-Host 'ok: no unbounded channels'
    }
}

# ---------------------------------------------------------------------------
# The four canonical commands. Order matches .github/workflows/ci.yml.
# ---------------------------------------------------------------------------
Write-Section 'cargo fmt --all -- --check'
& $cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { $failed += 'fmt' }

Write-Section 'cargo clippy --workspace --all-targets --all-features -- -D warnings'
& $cargo clippy --workspace --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { $failed += 'clippy' }

Write-Section 'cargo test --workspace --all-features'
& $cargo test --workspace --all-features
if ($LASTEXITCODE -ne 0) { $failed += 'test' }

if ($Release) {
    Write-Section 'cargo build --workspace --release'
    & $cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) { $failed += 'build --release' }
} else {
    Write-Section 'cargo build --workspace'
    & $cargo build --workspace
    if ($LASTEXITCODE -ne 0) { $failed += 'build' }
}

# ---------------------------------------------------------------------------
Write-Host ''
if ($failed.Count -gt 0) {
    Write-Host "FAILED: $($failed -join ', ')" -ForegroundColor Red
    exit 1
}

Write-Host 'All checks passed.' -ForegroundColor Green
exit 0
