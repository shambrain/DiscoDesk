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
    [switch]$SkipDeferredGuard,
    [switch]$SkipSmoke,
    # Skips the GUI launch test. It needs an interactive desktop session; CI
    # provides one, a headless agent context may not.
    [switch]$SkipStudioLaunch,
    # Skips the Studio frontend build. Only for engine-only debugging: the
    # studio crate cannot compile without it.
    [switch]$SkipFrontend
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
    $deferred = @(        'crates/dd-audio',
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
# The Studio frontend must be built before any cargo command that touches the
# studio crate: `tauri-build` embeds `frontendDist` at compile time and fails if
# it is missing. This runs first so a fresh checkout behaves like CI.
# ---------------------------------------------------------------------------
if (-not $SkipFrontend) {
    Write-Section 'Studio frontend (typecheck, tests, production build)'
    $studioDir = Join-Path $PSScriptRoot '..\apps\discodesk-studio'
    $npm = Get-Command npm.cmd -ErrorAction SilentlyContinue
    if (-not $npm) { $npm = Get-Command npm -ErrorAction SilentlyContinue }

    if (-not $npm) {
        Write-Host 'FAIL: npm was not found; the Studio frontend cannot be built.' -ForegroundColor Red
        $failed += 'frontend (npm missing)'
    } elseif (-not (Test-Path $studioDir)) {
        Write-Host "FAIL: Studio directory missing at $studioDir" -ForegroundColor Red
        $failed += 'frontend (missing directory)'
    } else {
        Push-Location $studioDir
        try {
            if (-not (Test-Path 'node_modules')) {
                & $npm.Source ci --no-audit --no-fund
                if ($LASTEXITCODE -ne 0) { $failed += 'frontend npm ci' }
            }
            if ($failed -notcontains 'frontend npm ci') {
                & $npm.Source run typecheck
                if ($LASTEXITCODE -ne 0) { $failed += 'frontend typecheck' }

                & $npm.Source test
                if ($LASTEXITCODE -ne 0) { $failed += 'frontend tests' }

                & $npm.Source run build
                if ($LASTEXITCODE -ne 0) { $failed += 'frontend build' }
            }
        } finally {
            Pop-Location
        }
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
# End-to-end agreement between the binary and the contract. Unit tests cannot
# prove that the engine process actually serves the protocol over a real pipe.
# ---------------------------------------------------------------------------
if (-not $SkipSmoke) {
    Write-Section 'IPC smoke test (real engine process, real named pipe)'
    $smoke = Join-Path $PSScriptRoot 'ipc-smoke.ps1'
    if (-not (Test-Path $smoke)) {
        Write-Host 'FAIL: scripts/ipc-smoke.ps1 is missing' -ForegroundColor Red
        $failed += 'ipc smoke (missing script)'
    } else {
        $configuration = if ($Release) { 'release' } else { 'debug' }
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $smoke -Configuration $configuration
        if ($LASTEXITCODE -ne 0) { $failed += 'ipc smoke' }
    }
}

# ---------------------------------------------------------------------------
# Application launch: real window, real lifecycle. Unit tests cannot show that
# closing the Studio leaves the engine running, which is a product invariant
# (docs/ADR/0001).
# ---------------------------------------------------------------------------
if (-not $SkipStudioLaunch) {
    Write-Section 'Studio launch smoke (real window, engine survives Studio close)'
    $studioSmoke = Join-Path $PSScriptRoot 'studio-smoke.ps1'
    if (-not (Test-Path $studioSmoke)) {
        Write-Host 'FAIL: scripts/studio-smoke.ps1 is missing' -ForegroundColor Red
        $failed += 'studio launch smoke (missing script)'
    } else {
        $configuration = if ($Release) { 'release' } else { 'debug' }
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $studioSmoke -Configuration $configuration
        if ($LASTEXITCODE -ne 0) { $failed += 'studio launch smoke' }
    }
}

# ---------------------------------------------------------------------------
# Dependency licence and advisory policy. This is a required gate in CI; locally
# it runs when the tool is installed and reports an explicit SKIP otherwise.
# A skip is deliberately loud: it is not the same result as a pass.
# ---------------------------------------------------------------------------
Write-Section 'cargo deny check all (licences, advisories, bans, sources)'
$deny = Get-Command cargo-deny.exe -ErrorAction SilentlyContinue
if ($deny) {
    & $deny.Source check all
    if ($LASTEXITCODE -ne 0) { $failed += 'cargo deny' }
} else {
    Write-Host 'SKIPPED: cargo-deny is not installed; the licence/advisory gate did NOT run locally.' -ForegroundColor Yellow
    Write-Host '  Install it with: cargo install cargo-deny --locked --version 0.22.1' -ForegroundColor Yellow
    Write-Host '  CI enforces this check regardless of local installation.' -ForegroundColor Yellow
}

# ---------------------------------------------------------------------------
Write-Host ''
if ($failed.Count -gt 0) {
    Write-Host "FAILED: $($failed -join ', ')" -ForegroundColor Red
    exit 1
}

Write-Host 'All checks passed.' -ForegroundColor Green
exit 0
