#requires -Version 5.1
[CmdletBinding()]
param(
    [switch]$BuildOnly,
    [switch]$Release,
    [switch]$UseDefaultFeatures,
    [string]$Features
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. $PSScriptRoot/setup-windows.ps1

function Get-AssetStatePath {
    param([Parameter(Mandatory)][string]$RepoRoot)
    $id = [BitConverter]::ToString(([Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($RepoRoot)))).Replace('-', '').Substring(0, 16)
    $dir = Join-Path $env:LOCALAPPDATA 'KeystoneCC\asset-state'
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    return Join-Path $dir "$id.json"
}

function Get-AssetFingerprint {
    param([Parameter(Mandatory)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) { return $null }
    $lines = foreach ($file in Get-ChildItem -LiteralPath $Path -File -Recurse -Force | Sort-Object FullName) {
        $relative = $file.FullName.Substring($Path.Length).TrimStart('\', '/')
        "$relative|$((Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash)"
    }
    return [BitConverter]::ToString(([Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))))).Replace('-', '')
}

function Test-AssetPlaceholder {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Name)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    return ((Get-Content -LiteralPath $Path -Raw).Trim() -replace '\\', '/') -eq "../ext-assets/$Name"
}

function Copy-VerifiedAssets {
    param([Parameter(Mandatory)][string]$RepoRoot, [Parameter(Mandatory)][string]$StatePath)
    $state = @{}
    foreach ($name in @('fonts', 'images')) {
        $source = Join-Path $RepoRoot "ext-assets\$name"; $destination = Join-Path $RepoRoot "assets\$name"
        if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw "Missing submodule assets: ext-assets/$name. Run: git submodule update --init --recursive" }
        if (Test-AssetPlaceholder $destination $name) {
            Remove-Item -LiteralPath $destination -Force
            Copy-Item -LiteralPath $source -Destination $destination -Recurse -Force
            Write-Host "Prepared assets/$name from ext-assets/$name."
        } elseif (-not (Test-Path -LiteralPath $destination -PathType Container)) {
            throw "Unexpected assets/$name. Restore the tracked placeholder with Git, or create a verified asset directory."
        } else {
            $sourceFingerprint = Get-AssetFingerprint $source; $destinationFingerprint = Get-AssetFingerprint $destination
            $previous = $null
            if (Test-Path -LiteralPath $StatePath) { $previous = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json }
            $old = if ($previous) { $previous.$name } else { $null }
            if ($sourceFingerprint -ne $destinationFingerprint) {
                if ($old -and $old.destination -eq $destinationFingerprint) {
                    Remove-Item -LiteralPath $destination -Recurse -Force
                    Copy-Item -LiteralPath $source -Destination $destination -Recurse -Force
                    Write-Host "Refreshed assets/$name after a source asset change."
                } else { throw "assets/$name differs from ext-assets/$name and was not overwritten. Preserve or reconcile local asset edits, then rerun." }
            }
        }
        $state[$name] = @{ source = (Get-AssetFingerprint $source); destination = (Get-AssetFingerprint $destination) }
    }
    $state | ConvertTo-Json | Set-Content -LiteralPath $StatePath -Encoding UTF8
}

function Invoke-RunWindows {
    if ($env:OS -ne 'Windows_NT') { throw 'This runner currently supports Windows only.' }
    $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot 'Cargo.toml'))) { throw 'Run this script from a checked-out Keystone repository.' }
    Refresh-ToolPath
    $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if (-not $rustup) { throw "rustup.exe is not on PATH. Run scripts/setup-windows.ps1, or set CARGO_HOME. Expected: $(Get-CargoBin)" }
    $toolchain = 'stable-x86_64-pc-windows-msvc'
    $git = Get-Command git.exe -ErrorAction SilentlyContinue
    if (-not $git) { throw 'git.exe is not on PATH. Run scripts/setup-windows.ps1 again.' }
    $commit = Invoke-Native $git.Source @('rev-parse', 'HEAD') -WorkingDirectory $repoRoot
    $dirty = Invoke-Native $git.Source @('status', '--short') -WorkingDirectory $repoRoot
    Write-Host "Repository: $repoRoot"
    Write-Host "Commit: $commit"
    if ($dirty) { Write-Host "Working tree changes:`n$dirty" } else { Write-Host 'Working tree: clean' }
    Invoke-Native $rustup.Source @('run', $toolchain, 'cargo', '--version') -WorkingDirectory $repoRoot
    Copy-VerifiedAssets $repoRoot (Get-AssetStatePath $repoRoot)
    $cargoArgs = @('run', '--locked')
    if ($BuildOnly) { $cargoArgs[0] = 'build' }
    if ($Release) { $cargoArgs += '--release' }
    if ($UseDefaultFeatures -and $Features) { throw 'Use either -UseDefaultFeatures or -Features, not both.' }
    if ($Features) { $cargoArgs += @('--no-default-features', '--features', $Features) } elseif (-not $UseDefaultFeatures) { $cargoArgs += '--no-default-features' }
    Write-Host "Command: rustup run $toolchain cargo $($cargoArgs -join ' ')"
    Invoke-Native $rustup.Source (@('run', $toolchain, 'cargo') + $cargoArgs) -WorkingDirectory $repoRoot
}

if ($MyInvocation.InvocationName -ne '.') {
    $reportDir = Join-Path $env:LOCALAPPDATA ('KeystoneCC\run\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path $reportDir -Force | Out-Null
    $report = Join-Path $reportDir 'run.log'
    Start-Transcript -Path $report | Out-Null
    try { Invoke-RunWindows }
    finally {
        Stop-Transcript | Out-Null
        Write-Host "Run log: $report"
    }
}
