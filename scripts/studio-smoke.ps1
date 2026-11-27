<#
.SYNOPSIS
    Application-launch smoke test for the DiscoDesk Studio.

.DESCRIPTION
    Proves the lifecycle contract from docs/ADR/0001 with the real binaries:

      1. The engine starts and reports readiness.
      2. The Studio executable launches as its own process.
      3. If an interactive desktop exists: the Studio creates a real window with
         the expected title, and closing it exits the process.
      4. The engine is STILL running after the Studio is gone.
      5. The engine still answers a status request afterwards.
      6. The engine shuts down cleanly when asked.

    Step 4 is the important one: "closing Studio must not stop the engine" is a
    product invariant no unit test can demonstrate, and it is enforced in every
    environment.

    # Environment dependence, stated honestly

    Assertions 3 (a visible window) require an interactive desktop session.
    GitHub-hosted Windows runners have none, so there the window assertions are
    SKIPPED with an explicit notice rather than reported as passing. Everything
    else — including the engine-survives-Studio invariant — still runs and is
    still enforced. The `--probe` path in scripts/ipc-smoke.ps1 independently
    proves a Studio process connecting and exiting leaves the engine serving.

    Exits 0 when every applicable assertion passes. A skipped assertion is
    reported as skipped, never as passed.

.PARAMETER Configuration
    debug (default), release or release-with-symbols.

.PARAMETER WindowTimeoutSeconds
    How long to wait for the window when a desktop is available.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release-with-symbols', 'release')]
    [string]$Configuration = 'debug',

    [int]$WindowTimeoutSeconds = 30,

    # Forces the window assertions on or off. By default they are enabled only
    # when an interactive desktop is detected.
    [ValidateSet('auto', 'require', 'skip')]
    [string]$WindowAssertions = 'auto'
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$engineExe = Join-Path $repoRoot "target\$Configuration\discodesk-engine.exe"
$studioExe = Join-Path $repoRoot "target\$Configuration\discodesk-studio.exe"

$script:failures = New-Object System.Collections.Generic.List[string]
$script:skips = New-Object System.Collections.Generic.List[string]

# Emitting GitHub workflow commands turns diagnostics into check-run annotations,
# which are readable through the API even when job logs require authentication.
function Write-GhCommand {
    param([string]$Kind, [string]$Message)
    if ($env:GITHUB_ACTIONS -eq 'true') {
        Write-Host "::$Kind::$Message"
    }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$What)
    if ($Actual -ne $Expected) {
        $script:failures.Add("$What : expected '$Expected', got '$Actual'")
        Write-Host "  FAIL $What (expected '$Expected', got '$Actual')" -ForegroundColor Red
        Write-GhCommand 'error' "studio-smoke: $What expected '$Expected', got '$Actual'"
    } else {
        Write-Host "  ok   $What = $Actual" -ForegroundColor Green
    }
}

function Assert-True {
    param([bool]$Condition, [string]$What)
    if (-not $Condition) {
        $script:failures.Add("$What : assertion failed")
        Write-Host "  FAIL $What" -ForegroundColor Red
        Write-GhCommand 'error' "studio-smoke: $What failed"
    } else {
        Write-Host "  ok   $What" -ForegroundColor Green
    }
}

function Skip-Assertion {
    param([string]$What, [string]$Reason)
    $script:skips.Add("$What : $Reason")
    Write-Host "  SKIP $What ($Reason)" -ForegroundColor Yellow
    Write-GhCommand 'notice' "studio-smoke: skipped '$What' - $Reason"
}

# An interactive desktop exists when the session is interactive AND a shell is
# running. Both are checked: UserInteractive alone can be true for a service
# session, and a stray explorer process alone is not conclusive either.
function Test-InteractiveDesktop {
    $interactive = [Environment]::UserInteractive
    $shell = Get-Process explorer -ErrorAction SilentlyContinue
    return ($interactive -and ($null -ne $shell))
}

if (-not (Test-Path -LiteralPath $engineExe)) {
    Write-Error "Engine binary missing at $engineExe. Build it first (-Action BuildDebug)."
    exit 2
}
if (-not (Test-Path -LiteralPath $studioExe)) {
    Write-Error "Studio binary missing at $studioExe. Build it first (-Action BuildStudio)."
    exit 2
}

$desktopDetected = Test-InteractiveDesktop
switch ($WindowAssertions) {
    'require' { $windowRequired = $true }
    'skip' { $windowRequired = $false }
    default { $windowRequired = $desktopDetected }
}

Write-Host "Engine: $engineExe" -ForegroundColor Cyan
Write-Host "Studio: $studioExe" -ForegroundColor Cyan
Write-Host ("Desktop: interactive={0} explorer={1} -> window assertions {2}" -f `
    [Environment]::UserInteractive, $desktopDetected, $(if ($windowRequired) { 'ENABLED' } else { 'SKIPPED' }))
Write-GhCommand 'notice' ("studio-smoke: desktopDetected={0} userInteractive={1} windowAssertions={2}" -f `
    $desktopDetected, [Environment]::UserInteractive, $(if ($windowRequired) { 'enabled' } else { 'skipped' }))

$endpoint = (& $engineExe --print-endpoint).Trim()
Write-Host "Endpoint: $endpoint" -ForegroundColor Cyan

$engine = $null
$studio = $null

try {
    # 1. Start the engine.
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $engineExe
    $psi.Arguments = "--endpoint `"$endpoint`" --log-level warn"
    $psi.RedirectStandardOutput = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $engine = [System.Diagnostics.Process]::Start($psi)

    $readyLine = $engine.StandardOutput.ReadLine()
    Assert-True -Condition ($null -ne $readyLine -and $readyLine -match 'engine ready') -What 'engine reported readiness'

    # 2. Launch the Studio GUI as its own process.
    $studio = Start-Process -FilePath $studioExe -ArgumentList @('--endpoint', $endpoint) -PassThru
    Assert-True -Condition ($null -ne $studio) -What 'Studio process started'

    # 3. Window assertions, when the environment can actually show a window.
    $windowAppeared = $false
    if ($windowRequired) {
        $title = ''
        for ($i = 0; $i -lt ($WindowTimeoutSeconds * 4); $i++) {
            Start-Sleep -Milliseconds 250
            if ($studio.HasExited) { break }
            $studio.Refresh()
            if (-not [string]::IsNullOrWhiteSpace($studio.MainWindowTitle)) {
                $windowAppeared = $true
                $title = $studio.MainWindowTitle
                break
            }
        }
    } else {
        Skip-Assertion -What 'Studio created a visible window' -Reason 'no interactive desktop in this environment'
        # Give the process a moment to either stay up or exit.
        Start-Sleep -Seconds 2
    }

    $studio.Refresh()
    if ($studio.HasExited) {
        if ($windowRequired) {
            $failures.Add("the Studio exited during startup with code $($studio.ExitCode)")
            Write-Host "  FAIL Studio exited during startup (code $($studio.ExitCode))" -ForegroundColor Red
            Write-GhCommand 'error' "studio-smoke: Studio exited during startup with code $($studio.ExitCode)"
        } else {
            # Without a desktop a GUI process may exit immediately. That is an
            # environment limitation, not a lifecycle failure, so it is reported
            # as a skip. The engine-survival invariant below is still enforced.
            Skip-Assertion -What 'Studio stays alive with a window' -Reason "exited with code $($studio.ExitCode) and no desktop is available"
        }
    } else {
        if ($windowRequired) {
            Assert-True -Condition $windowAppeared -What 'Studio created a visible window'
            if ($windowAppeared) {
                Write-Host "       window title: $title"
                Assert-True -Condition ($title -match 'DiscoDesk') -What 'window title identifies the Studio'
            }
        }

        # 4. Close the Studio (graceful, falling back to a kill).
        $closed = $false
        try { $closed = $studio.CloseMainWindow() } catch { $closed = $false }
        if (-not $closed) {
            try { $studio.Kill() } catch { }
        }
        $studio.WaitForExit(10000) | Out-Null
        Assert-True -Condition $studio.HasExited -What 'Studio process exited after the window closed'
        if ($studio.HasExited) {
            Write-Host "       Studio exit code: $($studio.ExitCode)"
        }
    }

    # 5. The invariant: the engine must survive the Studio going away.
    Start-Sleep -Seconds 1
    Assert-True -Condition (-not $engine.HasExited) -What 'engine is STILL running after the Studio closed'

    if (-not $engine.HasExited) {
        $probeOutput = & $studioExe --probe --endpoint $endpoint 2>&1
        $probeExit = $LASTEXITCODE
        Assert-Equal -Actual $probeExit -Expected 0 -What 'a fresh Studio probe still reaches the engine'
        $probe = $null
        try { $probe = (($probeOutput | Out-String) | ConvertFrom-Json) } catch { }
        if ($null -ne $probe) {
            Assert-Equal -Actual $probe.kind -Expected 'connected' -What 'engine answers after the Studio closed'
        } else {
            $failures.Add('the post-close probe did not print JSON')
            Write-Host '  FAIL post-close probe output was not JSON' -ForegroundColor Red
            Write-GhCommand 'error' 'studio-smoke: post-close probe output was not JSON'
        }

        # 6. Ask the engine to stop.
        $pipePrefix = '\\.\pipe\'
        $pipeName = if ($endpoint.StartsWith($pipePrefix)) { $endpoint.Substring($pipePrefix.Length) } else { $endpoint }
        $pipe = New-Object System.IO.Pipes.NamedPipeClientStream('.', $pipeName, [System.IO.Pipes.PipeDirection]::InOut)
        $pipe.Connect(5000)
        $json = '{"version":1,"request_id":99,"command":"ShutdownEngine"}'
        $payload = [System.Text.Encoding]::UTF8.GetBytes($json)
        $header = New-Object System.Collections.Generic.List[byte]
        $header.AddRange([byte[]](0x44, 0x44, 0x46, 0x31))
        $header.AddRange([BitConverter]::GetBytes([uint16]1))
        $header.AddRange([BitConverter]::GetBytes([uint32]$payload.Length))
        $pipe.Write($header.ToArray(), 0, 10)
        $pipe.Write($payload, 0, $payload.Length)
        $pipe.Flush()
        $pipe.Dispose()

        $exited = $engine.WaitForExit(10000)
        Assert-True -Condition $exited -What 'engine exited after ShutdownEngine'
        if ($exited) {
            Assert-Equal -Actual $engine.ExitCode -Expected 0 -What 'engine exit code'
        }
    }
}
finally {
    foreach ($p in @($studio, $engine)) {
        if ($null -ne $p -and -not $p.HasExited) {
            try { $p.Kill() } catch { }
            try { $p.WaitForExit(5000) | Out-Null } catch { }
        }
        if ($null -ne $p) { try { $p.Dispose() } catch { } }
    }
}

Write-Host ''
if ($script:skips.Count -gt 0) {
    Write-Host "SKIPPED assertions in this environment ($($script:skips.Count)):" -ForegroundColor Yellow
    $script:skips | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
}

if ($failures.Count -gt 0) {
    Write-Host "STUDIO LAUNCH SMOKE FAILED ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-GhCommand 'notice' ("studio-smoke: passed with {0} skipped assertion(s)" -f $script:skips.Count)
Write-Host 'STUDIO LAUNCH SMOKE PASSED' -ForegroundColor Green
exit 0
