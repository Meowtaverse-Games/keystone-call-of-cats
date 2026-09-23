# Compatible with a one-shot artifact worker. It deliberately has no network,
# runner registration, or credential handling: the caller extracts the artifact
# and supplies paths in SASARA_JOB_PAYLOAD.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

if (-not $env:SASARA_JOB_PAYLOAD -or -not $env:SASARA_JOB_OUTPUT) {
    throw 'SASARA_JOB_PAYLOAD and SASARA_JOB_OUTPUT are required.'
}

$payloadFile = Join-Path $env:SASARA_JOB_PAYLOAD 'smoke-job.json'
if (-not (Test-Path -LiteralPath $payloadFile -PathType Leaf)) {
    throw "Job payload missing: $payloadFile"
}
$payload = Get-Content -LiteralPath $payloadFile -Raw | ConvertFrom-Json
if (-not $payload.artifact_dir) {
    throw 'SASARA_JOB_PAYLOAD must include artifact_dir.'
}

$artifactDir = Join-Path $env:SASARA_JOB_PAYLOAD ([string]$payload.artifact_dir)
$executablePath = if ($payload.executable_path) {
    Join-Path $env:SASARA_JOB_PAYLOAD ([string]$payload.executable_path)
} else {
    Join-Path $artifactDir 'keystone-cc.exe'
}
$timeoutSeconds = if ($payload.timeout_seconds) { [int]$payload.timeout_seconds } else { 120 }
$outputDirectory = [string]$env:SASARA_JOB_OUTPUT
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$outputPath = Join-Path $outputDirectory 'result.json'

$reportPath = Join-Path $outputDirectory 'smoke-result.json'
$saveDir = Join-Path $outputDirectory 'smoke-save'
$logPath = Join-Path $outputDirectory 'smoke.log'

try {
    $result = & (Join-Path $env:SASARA_JOB_PAYLOAD 'invoke-smoke.ps1') `
        -ExecutablePath $executablePath -ReportPath $reportPath -SaveDir $saveDir `
        -TimeoutSeconds $timeoutSeconds -LogPath $logPath
    $result | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $outputPath -Encoding utf8
} catch {
    @{ status = 'failed'; error = $_.Exception.Message } |
        ConvertTo-Json | Set-Content -LiteralPath $outputPath -Encoding utf8
    throw
}
