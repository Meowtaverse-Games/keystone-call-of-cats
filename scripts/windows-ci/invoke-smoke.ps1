[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,
    [Parameter(Mandatory = $true)]
    [string]$ReportPath,
    [Parameter(Mandatory = $true)]
    [string]$SaveDir,
    [int]$TimeoutSeconds = 120,
    [string]$LogPath
)

$ErrorActionPreference = 'Stop'

function Stop-ProcessTree {
    param([int]$ProcessId)

    Get-CimInstance Win32_Process -Filter "ParentProcessId = $ProcessId" -ErrorAction SilentlyContinue |
        ForEach-Object { Stop-ProcessTree -ProcessId $_.ProcessId }
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}

if ($TimeoutSeconds -lt 1 -or $TimeoutSeconds -gt 300) {
    throw 'TimeoutSeconds must be between 1 and 300.'
}
if (-not (Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) {
    throw "Executable not found: $ExecutablePath"
}

$reportDirectory = Split-Path -Parent $ReportPath
if ($reportDirectory) { New-Item -ItemType Directory -Path $reportDirectory -Force | Out-Null }
New-Item -ItemType Directory -Path $SaveDir -Force | Out-Null
if (-not $LogPath) { $LogPath = Join-Path $reportDirectory 'smoke.log' }
$errorLogPath = "$LogPath.stderr"

$env:KEYSTONE_CI_SAVE_DIR = $SaveDir
$arguments = "--ci-smoke --ci-smoke-report `"$ReportPath`""
$process = Start-Process -FilePath $ExecutablePath -ArgumentList $arguments -PassThru -NoNewWindow `
    -RedirectStandardOutput $LogPath -RedirectStandardError $errorLogPath

try {
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Stop-ProcessTree -ProcessId $process.Id
        throw "Smoke launch exceeded ${TimeoutSeconds}s and was stopped."
    }

    if ($process.ExitCode -ne 0) {
        throw "Smoke launch exited with code $($process.ExitCode)."
    }
    if (-not (Test-Path -LiteralPath $ReportPath -PathType Leaf)) {
        throw "Smoke launch did not create report: $ReportPath"
    }

    $result = Get-Content -LiteralPath $ReportPath -Raw | ConvertFrom-Json
    if ($result.status -ne 'ready' -or $result.state -ne 'SelectStage' -or $result.assets_ready -ne $true) {
        throw 'Smoke report did not confirm assets ready and SelectStage.'
    }
    $result
} finally {
    if (-not $process.HasExited) {
        Stop-ProcessTree -ProcessId $process.Id
    }
}
