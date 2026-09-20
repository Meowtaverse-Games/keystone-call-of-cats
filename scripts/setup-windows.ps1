#requires -Version 5.1
[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$SkipSmokeTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:WindowsToolchain = 'stable-x86_64-pc-windows-msvc'

function Get-CargoBin {
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    return Join-Path $cargoHome 'bin'
}

function Refresh-ToolPath {
    $parts = @(
        (Get-CargoBin),
        [Environment]::GetEnvironmentVariable('Path', 'Machine'),
        [Environment]::GetEnvironmentVariable('Path', 'User'),
        $env:Path
    ) | Where-Object { $_ }
    $env:Path = $parts -join ';'
}

function Invoke-Native {
    param([Parameter(Mandatory)][string]$FilePath, [string[]]$Arguments = @(), [string]$WorkingDirectory)

    # Process avoids Windows PowerShell 5.1 treating normal native stderr as a
    # terminating error when $ErrorActionPreference is Stop.
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $FilePath
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardOutputEncoding = [Text.Encoding]::UTF8
    $startInfo.StandardErrorEncoding = [Text.Encoding]::UTF8
    if ($WorkingDirectory) { $startInfo.WorkingDirectory = $WorkingDirectory }
    # ArgumentList is unavailable in the .NET Framework used by Windows
    # PowerShell 5.1. Quote the small command lines used by these scripts.
    $quotedArguments = foreach ($argument in $Arguments) {
        if ($argument -notmatch '[\s"]') { $argument } else {
            $escaped = [regex]::Replace($argument, '(\\*)"', '$1$1\\"')
            $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
            '"' + $escaped + '"'
        }
    }
    $startInfo.Arguments = $quotedArguments -join ' '
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { throw "Could not start $FilePath." }
    # Read both streams while the child is alive. This keeps first-run Cargo
    # progress visible and prevents either redirected pipe from filling up.
    $stdoutTask = $process.StandardOutput.ReadLineAsync(); $stderrTask = $process.StandardError.ReadLineAsync()
    $stdoutDone = $false; $stderrDone = $false
    while (-not ($process.HasExited -and $stdoutDone -and $stderrDone)) {
        if (-not $stdoutDone -and $stdoutTask.IsCompleted) {
            $line = $stdoutTask.GetAwaiter().GetResult()
            if ($null -eq $line) { $stdoutDone = $true } else { Write-Output $line; $stdoutTask = $process.StandardOutput.ReadLineAsync() }
        }
        if (-not $stderrDone -and $stderrTask.IsCompleted) {
            $line = $stderrTask.GetAwaiter().GetResult()
            if ($null -eq $line) { $stderrDone = $true } else { Write-Output $line; $stderrTask = $process.StandardError.ReadLineAsync() }
        }
        if (-not (($stdoutTask.IsCompleted -or $stdoutDone) -and ($stderrTask.IsCompleted -or $stderrDone))) { [Threading.Thread]::Sleep(25) }
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "$FilePath failed (exit $($process.ExitCode))." }
}

function Test-InstalledWindowsSdk {
    param([string]$SdkRoot = (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'))
    $libRoot = Join-Path $SdkRoot 'Lib'
    if (-not (Test-Path -LiteralPath $libRoot -PathType Container)) { return $false }
    foreach ($version in Get-ChildItem -LiteralPath $libRoot -Directory -Force) {
        $kernel32 = Join-Path $version.FullName 'um\x64\kernel32.lib'
        $ucrt = Join-Path $version.FullName 'ucrt\x64\ucrt.lib'
        if ((Test-Path -LiteralPath $kernel32 -PathType Leaf) -and (Test-Path -LiteralPath $ucrt -PathType Leaf)) { return $true }
    }
    return $false
}

function Find-CppInstallation {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $path = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -ne 0) { throw 'vswhere failed while checking MSVC.' }
        if ($path) { return @($path)[0] }
    }
    return $null
}

function Install-WingetPackage {
    param([Parameter(Mandatory)][string]$Id, [string]$Override)
    if ($CheckOnly) { throw "Missing dependency: $Id. Run setup again without -CheckOnly." }
    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) { throw 'winget.exe is missing. Install Microsoft App Installer from Microsoft Store, then retry.' }
    $args = @('install', '--id', $Id, '--exact', '--source', 'winget', '--accept-source-agreements', '--accept-package-agreements')
    if ($Override) { $args += @('--override', $Override) }
    Invoke-Native $winget.Source $args
    Refresh-ToolPath
}

function Invoke-SetupWindows {
    if ($env:OS -ne 'Windows_NT' -or [Environment]::Is64BitOperatingSystem -eq $false) {
        throw 'This setup currently supports Windows 11 x64 only. Run it in Windows PowerShell, not WSL.'
    }
    Refresh-ToolPath
    if (-not (Get-Command git.exe -ErrorAction SilentlyContinue)) { Install-WingetPackage 'Git.Git' }

    $cppInstallation = Find-CppInstallation
    if (-not $cppInstallation) {
        if ($CheckOnly) { throw 'MSVC C++ Build Tools are missing. Run setup again without -CheckOnly.' }
        Write-Host 'Installing Visual Studio Build Tools requires a UAC confirmation. A reboot may be requested; this script never reboots Windows.'
        Install-WingetPackage 'Microsoft.VisualStudio.2022.BuildTools' '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
        $cppInstallation = Find-CppInstallation
        if (-not $cppInstallation) { throw 'MSVC C++ Build Tools are not ready. Restart Windows if requested, then rerun setup.' }
    }
    if (-not (Test-InstalledWindowsSdk)) {
        throw 'A complete x64 Windows SDK is missing (kernel32.lib and ucrt.lib must be in the same SDK version). In Visual Studio Installer choose Modify for Build Tools, add "Desktop development with C++" and a Windows 10/11 SDK, then rerun setup.'
    }

    if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) { Install-WingetPackage 'Rustlang.Rustup' }
    $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if (-not $rustup) { throw "rustup.exe was installed but is not on PATH. Open a new PowerShell, or set CARGO_HOME then rerun. Expected: $(Get-CargoBin)" }
    if (-not $CheckOnly) { Invoke-Native $rustup.Source @('toolchain', 'install', $script:WindowsToolchain, '--profile', 'minimal') }
    $version = & $rustup.Source run $script:WindowsToolchain rustc --version
    if ($LASTEXITCODE -ne 0) { throw 'The Rust MSVC toolchain is unavailable. Rerun setup without -CheckOnly.' }
    Write-Output $version
    if ($version -notmatch '^rustc (\d+\.\d+\.\d+)' -or [version]$Matches[1] -lt [version]'1.95.0') { throw 'This project requires Rust 1.95.0 or newer.' }
    Invoke-Native (Get-Command git.exe).Source @('--version')
    Invoke-Native $rustup.Source @('run', $script:WindowsToolchain, 'cargo', '--version')
    if (-not $SkipSmokeTest) {
        $probeDir = Join-Path $env:TEMP ('keystone-cc-probe-' + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $probeDir | Out-Null
        try {
            $source = Join-Path $probeDir 'probe.rs'; $exe = Join-Path $probeDir 'probe.exe'
            Set-Content -LiteralPath $source -Encoding ASCII -Value 'fn main() { println!("MSVC compile: OK"); }'
            Invoke-Native $rustup.Source @('run', $script:WindowsToolchain, 'rustc', $source, '-o', $exe)
            Invoke-Native $exe
        } finally { Remove-Item -LiteralPath $probeDir -Recurse -Force -ErrorAction SilentlyContinue }
    }
    Write-Host 'Toolchain ready. Game build and runtime checks are separate.'
}

if ($MyInvocation.InvocationName -ne '.') { Invoke-SetupWindows }
