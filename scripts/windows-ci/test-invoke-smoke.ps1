[CmdletBinding()]
param()

# This replaces Start-Process with a zero-exit child that creates no report. It
# runs on PowerShell 5.1 and 7 without opening a game window.

$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ("keystone-smoke-test-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $root -Force | Out-Null
try {
    $report = Join-Path $root 'report.json'
    '{"status":"ready","state":"SelectStage","assets_ready":true}' | Set-Content -LiteralPath $report -Encoding utf8
    $fake = Join-Path $root 'exit-zero.exe'
    New-Item -ItemType File -Path $fake -Force | Out-Null

    function Start-Process {
        [pscustomobject]@{
            Id = 1
            Handle = [IntPtr]1
            HasExited = $true
            ExitCode = 0
        } | Add-Member -MemberType ScriptMethod -Name WaitForExit -Value { param($milliseconds) $true } -PassThru
    }

    $threw = $false
    try {
        & (Join-Path $PSScriptRoot 'invoke-smoke.ps1') -ExecutablePath $fake -ReportPath $report -SaveDir (Join-Path $root 'save') -TimeoutSeconds 5
    } catch {
        $threw = $true
    }
    if (-not $threw) { throw 'A stale ready report was accepted after a zero-exit child.' }
    if (Test-Path -LiteralPath $report) { throw 'The stale report was not cleared before launch.' }
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
