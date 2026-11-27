<#
.SYNOPSIS
    Application-launch smoke test for the DiscoDesk Studio.

.DESCRIPTION
    Proves the lifecycle contract from docs/ADR/0001 with the real binaries:

      1. The engine starts and reports readiness.
      2. The Studio executable launches and creates a real window.
      3. The Studio window can be closed while the engine keeps running.
      4. The engine still answers a status request afterwards.
      5. The engine shuts down cleanly when asked.

    Step 3 is the important one: "closing Studio must not stop the engine" is a
    product invariant that no unit test can demonstrate.

    Exits 0 only if every assertion passes.

.PARAMETER Configuration
    debug (default) or release / release-with-symbols.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release-with-symbols', 'release')]
    [string]$Configuration = 'debug',

    # Seconds to wait for the Studio window to appear.
    [int]$WindowTimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$engineExe = Join-Path $repoRoot "target\$Configuration\discodesk-engine.exe"
$studioExe = Join-Path $repoRoot "target\$Configuration\discodesk-studio.exe"

$failures = New-Object System.Collections.Generic.List[string]

function Assert-Equal {
    param($Actual, $Expected, [string]$What)
    if ($Actual -ne $Expected) {
        $script:failures.Add("$What : expected '$Expected', got '$Actual'")
        Write-Host "  FAIL $What (expected '$Expected', got '$Actual')" -ForegroundColor Red
    } else {
        Write-Host "  ok   $What = $Actual" -ForegroundColor Green
    }
}

function Assert-True {
    param([bool]$Condition, [string]$What)
    if (-not $Condition) {
        $script:failures.Add("$What : assertion failed")
        Write-Host "  FAIL $What" -ForegroundColor Red
    } else {
        Write-Host "  ok   $What" -ForegroundColor Green
    }
}

if (-not (Test-Path -LiteralPath $engineExe)) {
    Write-Error "Engine binary missing at $engineExe. Build it first (-Action BuildDebug)."
    exit 2
}
if (-not (Test-Path -LiteralPath $studioExe)) {
    Write-Error "Studio binary missing at $studioExe. Build it first (-Action BuildStudio)."
    exit 2
}

Write-Host "Engine: $engineExe" -ForegroundColor Cyan
Write-Host "Studio: $studioExe" -ForegroundColor Cyan

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

    # 2. Launch the Studio GUI.
    $studio = Start-Process -FilePath $studioExe -ArgumentList @('--endpoint', $endpoint) -PassThru
    Assert-True -Condition ($null -ne $studio) -What 'Studio process started'

    $windowAppeared = $false
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

    if ($studio.HasExited) {
        $failures.Add("the Studio exited during startup with code $($studio.ExitCode)")
        Write-Host "  FAIL Studio exited during startup (code $($studio.ExitCode))" -ForegroundColor Red
    } else {
        Assert-True -Condition $windowAppeared -What 'Studio created a visible window'
        if ($windowAppeared) {
            Write-Host "       window title: $title"
            Assert-True -Condition ($title -match 'DiscoDesk') -What 'window title identifies the Studio'
        }

        # Give the UI one poll cycle to reach the engine, then confirm the engine
        # is still healthy while the Studio is open.
        Start-Sleep -Seconds 3
        Assert-True -Condition (-not $engine.HasExited) -What 'engine is running while the Studio is open'

        # 3. Close the Studio window (graceful), falling back to a kill.
        $closed = $false
        try { $closed = $studio.CloseMainWindow() } catch { $closed = $false }
        if (-not $closed) {
            try { $studio.Kill() } catch { }
        }
        $studio.WaitForExit(10000) | Out-Null
        Assert-True -Condition $studio.HasExited -What 'Studio process exited after the window closed'
        Write-Host "       Studio exit code: $($studio.ExitCode)"

        # 4. The engine must survive. This is the whole point of the test.
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
            }
        }

        # 5. Ask the engine to stop.
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
if ($failures.Count -gt 0) {
    Write-Host "STUDIO LAUNCH SMOKE FAILED ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host 'STUDIO LAUNCH SMOKE PASSED' -ForegroundColor Green
exit 0
