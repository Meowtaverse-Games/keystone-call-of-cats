#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'setup-windows.ps1')
. (Join-Path $PSScriptRoot 'run-windows.ps1')
$temp = Join-Path ([IO.Path]::GetTempPath()) ('keystone-windows-tests-' + [guid]::NewGuid().ToString('N'))
try {
    $env:USERPROFILE = $temp; $env:LOCALAPPDATA = Join-Path $temp 'local-app-data'; $env:CARGO_HOME = Join-Path $temp 'cargo-home'; $env:Path = 'original-path'
    Refresh-ToolPath
    if (-not $env:Path.StartsWith((Join-Path $env:CARGO_HOME 'bin') + ';')) { throw 'PATH refresh did not put CARGO_HOME/bin first.' }
    $failed = $false
    try { Invoke-Native '/bin/sh' @('-c', 'exit 9') } catch { $failed = $true }
    if (-not $failed) { throw 'Native nonzero exits must throw.' }
    New-Item -ItemType Directory -Path (Join-Path $temp 'assets'), (Join-Path $temp 'ext-assets/fonts'), (Join-Path $temp 'ext-assets/images') -Force | Out-Null
    Set-Content (Join-Path $temp 'assets/fonts') '../ext-assets/fonts'; Set-Content (Join-Path $temp 'assets/images') '../ext-assets/images'
    Set-Content (Join-Path $temp 'ext-assets/fonts/a.txt') 'first'; Set-Content (Join-Path $temp 'ext-assets/images/a.txt') 'first'
    $state = Join-Path $temp 'state.json'; Copy-VerifiedAssets $temp $state
    if (-not (Test-Path (Join-Path $temp 'assets/fonts/a.txt'))) { throw 'Verified placeholders were not prepared.' }
    Copy-VerifiedAssets $temp $state
    Set-Content (Join-Path $temp 'ext-assets/fonts/a.txt') 'second'; Copy-VerifiedAssets $temp $state
    if ((Get-Content (Join-Path $temp 'assets/fonts/a.txt') -Raw).Trim() -ne 'second') { throw 'Unchanged copied assets were not refreshed.' }
    Set-Content (Join-Path $temp 'assets/fonts/a.txt') 'local'; Set-Content (Join-Path $temp 'ext-assets/fonts/a.txt') 'third'
    $protected = $false
    try { Copy-VerifiedAssets $temp $state } catch { $protected = $_.Exception.Message -match 'not overwritten' }
    if (-not $protected) { throw 'Local asset edits must be protected.' }
    $otherRoot = Join-Path $temp 'other-checkout'; New-Item -ItemType Directory -Path $otherRoot | Out-Null
    if ((Get-AssetStatePath $temp) -eq (Get-AssetStatePath $otherRoot)) { throw 'Asset state must be isolated by checkout.' }
    $linkedRoot = Join-Path $temp 'linked-checkout'
    New-Item -ItemType Directory -Path (Join-Path $linkedRoot 'assets'), (Join-Path $linkedRoot 'ext-assets/fonts'), (Join-Path $linkedRoot 'ext-assets/images') -Force | Out-Null
    Set-Content (Join-Path $linkedRoot 'ext-assets/fonts/a.txt') 'linked'; Set-Content (Join-Path $linkedRoot 'ext-assets/images/a.txt') 'linked'
    New-Item -ItemType SymbolicLink -Path (Join-Path $linkedRoot 'assets/fonts') -Target (Join-Path $linkedRoot 'ext-assets/fonts') | Out-Null
    New-Item -ItemType SymbolicLink -Path (Join-Path $linkedRoot 'assets/images') -Target (Join-Path $linkedRoot 'ext-assets/images') | Out-Null
    Copy-VerifiedAssets $linkedRoot (Join-Path $temp 'linked-state.json')
    if (-not (Get-Item -LiteralPath (Join-Path $linkedRoot 'assets/fonts') -Force).LinkType) { throw 'A real asset link must remain a link.' }
    Write-Host 'Windows script regression tests passed.'
} finally { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
