<#
.SYNOPSIS
    End-to-end smoke test of the DiscoDesk engine IPC contract.

.DESCRIPTION
    Starts the real engine binary, talks to it over the real Windows named pipe,
    and verifies the framing, envelope, command handling and shutdown path from
    the outside. Nothing here mocks the transport.

    This is the Gate 0 integration check that the Studio will later replace with
    its own client. It exists because "the crate compiles" is not evidence that
    the engine process actually serves the protocol.

    Exits 0 only if every assertion passes.

.PARAMETER Configuration
    debug (default) or release-with-symbols.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release-with-symbols', 'release')]
    [string]$Configuration = 'debug'
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $repoRoot "target\$Configuration\discodesk-engine.exe"

if (-not (Test-Path -LiteralPath $exe)) {
    Write-Error "Engine binary not found at $exe. Build it first: scripts/vs/Invoke-DiscoDeskVS.ps1 -Action BuildDebug"
    exit 2
}

$magic = [byte[]](0x44, 0x44, 0x46, 0x31)   # "DDF1"
$protocolVersion = 1
$failures = New-Object System.Collections.Generic.List[string]

function Write-Frame {
    param(
        [System.IO.Stream]$Stream,
        [int]$Version,
        [string]$Json
    )
    $payload = [System.Text.Encoding]::UTF8.GetBytes($Json)
    $header = New-Object System.Collections.Generic.List[byte]
    $header.AddRange($script:magic)
    $header.AddRange([BitConverter]::GetBytes([uint16]$Version))
    $header.AddRange([BitConverter]::GetBytes([uint32]$payload.Length))
    $Stream.Write($header.ToArray(), 0, 10)
    if ($payload.Length -gt 0) { $Stream.Write($payload, 0, $payload.Length) }
    $Stream.Flush()
}

function Read-Exact {
    param([System.IO.Stream]$Stream, [int]$Count)
    $buffer = New-Object byte[] $Count
    $offset = 0
    while ($offset -lt $Count) {
        $read = $Stream.Read($buffer, $offset, $Count - $offset)
        if ($read -le 0) { throw "connection closed after $offset of $Count bytes" }
        $offset += $read
    }
    return $buffer
}

function Read-Frame {
    param([System.IO.Stream]$Stream)
    $header = Read-Exact -Stream $Stream -Count 10
    if (-not ($header[0] -eq 0x44 -and $header[1] -eq 0x44 -and $header[2] -eq 0x46 -and $header[3] -eq 0x31)) {
        throw "bad magic in response header"
    }
    $version = [BitConverter]::ToUInt16($header, 4)
    $length = [BitConverter]::ToUInt32($header, 6)
    if ($length -gt 1048576) { throw "response payload of $length bytes exceeds the limit" }
    $payload = if ($length -gt 0) { Read-Exact -Stream $Stream -Count $length } else { @() }
    $json = [System.Text.Encoding]::UTF8.GetString($payload)
    return [pscustomobject]@{ Version = $version; Json = ($json | ConvertFrom-Json) }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$What)
    if ($Actual -ne $Expected) {
        $script:failures.Add("$What : expected '$Expected', got '$Actual'")
        Write-Host "  FAIL $What (expected '$Expected', got '$Actual')" -ForegroundColor Red
    } else {
        Write-Host "  ok   $What = $Actual" -ForegroundColor Green
    }
}

Write-Host "Engine: $exe" -ForegroundColor Cyan

# 1. Ask the engine where its endpoint is (its own default derivation).
$endpoint = (& $exe --print-endpoint).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($endpoint)) {
    Write-Error "--print-endpoint failed"
    exit 1
}
Write-Host "Endpoint: $endpoint" -ForegroundColor Cyan

# 2. Verify --version works and does not start the engine.
$versionText = (& $exe --version).Trim()
Assert-Equal -Actual ($versionText -like 'discodesk-engine *') -Expected $true -What '--version prints a name and version'

# 3. Start the real engine process.
#    ProcessStartInfo is used instead of Start-Process -PassThru because the
#    object Start-Process returns does not always expose a reliable ExitCode.
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $exe
$psi.Arguments = "--endpoint `"$endpoint`" --log-level info"
$psi.RedirectStandardOutput = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true

$proc = [System.Diagnostics.Process]::Start($psi)

try {
    # 4. Wait for the readiness line rather than sleeping blindly.
    $readyLine = $proc.StandardOutput.ReadLine()
    if ($null -eq $readyLine -or $readyLine -notmatch 'engine ready') {
        throw "engine did not report readiness; first line was: $readyLine"
    }
    $null = $proc.StandardOutput.ReadLine()   # protocol/state line
    Write-Host "  ok   engine reported readiness ($readyLine)" -ForegroundColor Green

    # 5. Connect over the real named pipe.
    #    NamedPipeClientStream takes the pipe NAME, not the full `\\.\pipe\...`
    #    path that --print-endpoint reports, so strip the fixed prefix.
    $pipePrefix = '\\.\pipe\'
    $pipeName = if ($endpoint.StartsWith($pipePrefix)) { $endpoint.Substring($pipePrefix.Length) } else { $endpoint }
    $pipe = New-Object System.IO.Pipes.NamedPipeClientStream('.', $pipeName, [System.IO.Pipes.PipeDirection]::InOut)
    $pipe.Connect(5000)
    Write-Host "  ok   connected to the named pipe" -ForegroundColor Green

    # 6. GetEngineStatus
    Write-Frame -Stream $pipe -Version $protocolVersion `
        -Json '{"version":1,"request_id":1,"command":"GetEngineStatus"}'
    $response = Read-Frame -Stream $pipe
    Assert-Equal -Actual $response.Version -Expected 1 -What 'response frame version'
    Assert-Equal -Actual $response.Json.request_id -Expected 1 -What 'request_id is echoed'
    Assert-Equal -Actual $response.Json.result.state -Expected 'IdleReady' -What 'engine state'
    Assert-Equal -Actual $response.Json.result.isRunning -Expected $true -What 'isRunning'
    Assert-Equal -Actual $response.Json.result.surfaceCount -Expected 0 -What 'surfaceCount (no desktop host yet)'

    # 7. GetPlatformCapabilities must not overclaim.
    Write-Frame -Stream $pipe -Version $protocolVersion `
        -Json '{"version":1,"request_id":2,"command":"GetPlatformCapabilities"}'
    $caps = Read-Frame -Stream $pipe
    Assert-Equal -Actual $caps.Json.result.probed -Expected $false -What 'capabilities are marked not probed'
    Assert-Equal -Actual $caps.Json.result.wallpaperHosting -Expected 'unknown' -What 'wallpaper hosting is unknown, not supported'

    # 8. A deferred command must refuse with NotImplemented (1015) and keep the
    #    connection open, so the next request still works.
    Write-Frame -Stream $pipe -Version $protocolVersion `
        -Json '{"version":1,"request_id":3,"command":"GetAudioSources"}'
    $deferred = Read-Frame -Stream $pipe
    Assert-Equal -Actual $deferred.Json.error.code -Expected 1015 -What 'GetAudioSources is NotImplemented'
    Assert-Equal -Actual ($deferred.Json.error.message -match 'Gate 1') -Expected $true -What 'the refusal names its gate'

    # 9. An unknown command must be rejected, not defaulted.
    Write-Frame -Stream $pipe -Version $protocolVersion `
        -Json '{"version":1,"request_id":4,"command":"NoSuchCommand"}'
    $unknown = Read-Frame -Stream $pipe
    Assert-Equal -Actual $unknown.Json.error.code -Expected 1006 -What 'unknown command is rejected'

    # 10. ShutdownEngine replies, then the process must exit on its own.
    Write-Frame -Stream $pipe -Version $protocolVersion `
        -Json '{"version":1,"request_id":5,"command":"ShutdownEngine"}'
    $shutdown = Read-Frame -Stream $pipe
    Assert-Equal -Actual $shutdown.Json.request_id -Expected 5 -What 'shutdown is acknowledged'

    $pipe.Dispose()

    if (-not $proc.WaitForExit(10000)) {
        $failures.Add('the engine did not exit within 10s of ShutdownEngine')
        Write-Host '  FAIL engine did not exit after ShutdownEngine' -ForegroundColor Red
    } else {
        # The timed overload does not guarantee the exit code is populated on
        # the object returned by Start-Process; the parameterless WaitForExit
        # plus Refresh does.
        $proc.WaitForExit()
        $proc.Refresh()
        Write-Host "  ok   engine exited with code $($proc.ExitCode)" -ForegroundColor Green
        Assert-Equal -Actual $proc.ExitCode -Expected 0 -What 'engine exit code'
    }
}
finally {
    if (-not $proc.HasExited) {
        Write-Host '  cleaning up: stopping the engine process' -ForegroundColor Yellow
        try { $proc.Kill() } catch { }
        $proc.WaitForExit(5000) | Out-Null
    }
    $proc.Dispose()
}

Write-Host ''
if ($failures.Count -gt 0) {
    Write-Host "SMOKE TEST FAILED ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host 'IPC SMOKE TEST PASSED' -ForegroundColor Green
exit 0
