[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$Message,

    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]]$Paths,

    [switch]$Push
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Invoke-Git {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw ("git {0} failed with exit code {1}" -f ($Arguments -join ' '), $LASTEXITCODE)
    }
}

Push-Location $repoRoot
try {
    $root = (& git rev-parse --show-toplevel 2>$null)
    if ($LASTEXITCODE -ne 0 -or -not $root) { throw 'The script directory is not inside a Git working tree.' }
    $root = [System.IO.Path]::GetFullPath($root.Trim())
    if ($root.TrimEnd('\') -ne $repoRoot.TrimEnd('\')) { throw "Repository root mismatch. Expected $repoRoot but Git reports $root." }

    $branch = (& git branch --show-current).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $branch) { throw 'Git is in detached HEAD state. Switch to a named branch before committing.' }

    & git diff --cached --quiet
    if ($LASTEXITCODE -eq 1) { throw 'The Git index already contains staged changes. Commit or unstage them first; unrelated staged work will not be mixed.' }
    if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect the Git index.' }

    $safePaths = New-Object System.Collections.Generic.List[string]
    foreach ($rawPath in $Paths) {
        $path = $rawPath.Replace('\', '/').Trim()
        if ([string]::IsNullOrWhiteSpace($path) -or [System.IO.Path]::IsPathRooted($path)) { throw "Invalid repository-relative path: $rawPath" }
        if ($path -match '(^|/)\.\.(/|$)') { throw "Parent-directory traversal is not allowed: $rawPath" }
        if ($path -match '(^|/)(\.env(\..*)?|.*\.(pem|pfx|p12|key)|secrets?(\..*)?)$') { throw "Potential secret path refused: $rawPath" }

        $fullPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $path))
        $prefix = $repoRoot.TrimEnd('\') + '\'
        if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) { throw "Path escapes repository root: $rawPath" }
        if (-not (Test-Path -LiteralPath $fullPath)) { throw "Selected path does not exist: $rawPath" }
        $safePaths.Add($path)
    }

    $safePaths = @($safePaths | Select-Object -Unique)
    if ($safePaths.Count -eq 0) { throw 'No paths were selected.' }

    Write-Host ("Repository: " + $repoRoot)
    Write-Host ("Branch: " + $branch)
    Write-Host 'Selected paths:'
    $safePaths | ForEach-Object { Write-Host ('  ' + $_) }

    $status = & git status --porcelain -- @safePaths
    if ($LASTEXITCODE -ne 0) { throw 'Unable to inspect selected path status.' }
    if (-not $status) { throw 'None of the selected paths has changes to commit.' }

    Invoke-Git -Arguments (@('add', '--') + $safePaths)
    Invoke-Git -Arguments @('diff', '--cached', '--check')

    Write-Host ''
    Write-Host 'Staged change summary:'
    Invoke-Git -Arguments @('diff', '--cached', '--stat')
    Write-Host ''
    Write-Host 'Review the staged diff:'
    Invoke-Git -Arguments @('diff', '--cached', '--')

    $confirmation = Read-Host 'Type COMMIT to create this commit'
    if ($confirmation -cne 'COMMIT') {
        Invoke-Git -Arguments (@('reset', '--') + $safePaths)
        throw 'Commit cancelled. Selected files remain modified but unstaged.'
    }

    Invoke-Git -Arguments @('commit', '-m', $Message)
    if ($Push) {
        Invoke-Git -Arguments @('push', '-u', 'origin', $branch)
    } else {
        Write-Host ''
        Write-Host 'Local commit created. No push was performed.' -ForegroundColor Green
        Write-Host ("To publish later: git push -u origin " + $branch)
    }
} finally {
    Pop-Location
}
