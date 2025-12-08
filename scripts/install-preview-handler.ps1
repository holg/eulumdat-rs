# Eulumdat Preview Handler Installation Script
# Supports both x64 and ARM64 Windows
# Run as Administrator

param(
    [switch]$Uninstall,
    [switch]$Debug,
    [ValidateSet("auto", "x64", "arm64")]
    [string]$Architecture = "auto"
)

$ErrorActionPreference = "Continue"

# Detect architecture if auto
if ($Architecture -eq "auto") {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { $Architecture = "x64" }
        "ARM64" { $Architecture = "arm64" }
        "x86"   {
            # Check if running on ARM64 Windows in x86 emulation
            if ($env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
                $Architecture = "arm64"
            } else {
                Write-Host "ERROR: 32-bit Windows is not supported" -ForegroundColor Red
                exit 1
            }
        }
        default {
            Write-Host "ERROR: Unknown architecture: $arch" -ForegroundColor Red
            exit 1
        }
    }
    Write-Host "Detected architecture: $Architecture" -ForegroundColor Cyan
}

# Map to Rust target triple
$targetTriple = switch ($Architecture) {
    "x64" { "x86_64-pc-windows-msvc" }
    "arm64" { "aarch64-pc-windows-msvc" }
}

# Determine build type and paths to search
$buildType = if ($Debug) { "debug" } else { "release" }

# Find the script location and project root
$scriptDir = $PSScriptRoot
if (-not $scriptDir) {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}
$projectRoot = Split-Path -Parent $scriptDir

# Also try UNC path if we're on a network drive
$projectRootUNC = $projectRoot
if ($projectRoot -match "^([A-Z]):(.*)$") {
    $driveLetter = $Matches[1]
    $pathPart = $Matches[2]

    # Try to get UNC path for the drive
    $netUse = net use $driveLetter`: 2>$null | Select-String "Remote"
    if ($netUse -match "Remote\s+(.+)$") {
        $uncRoot = $Matches[1].Trim()
        $projectRootUNC = "$uncRoot$pathPart"
    }
}

# Search for DLL in multiple locations
# Check CARGO_TARGET_DIR first (used when target is redirected), then project target dir
$cargoTargetDir = $env:CARGO_TARGET_DIR
if (-not $cargoTargetDir) {
    # Try common custom target locations
    $cargoTargetDir = "C:\cargo_target"
}

$searchPaths = @(
    # Custom CARGO_TARGET_DIR locations
    (Join-Path $cargoTargetDir "$targetTriple\$buildType\eulumdat_preview.dll"),
    (Join-Path $cargoTargetDir "$buildType\eulumdat_preview.dll"),
    # Project-local target directories
    (Join-Path $projectRoot "target\$targetTriple\$buildType\eulumdat_preview.dll"),
    (Join-Path $projectRoot "target\$buildType\eulumdat_preview.dll"),
    # UNC paths for network drives
    (Join-Path $projectRootUNC "target\$targetTriple\$buildType\eulumdat_preview.dll"),
    (Join-Path $projectRootUNC "target\$buildType\eulumdat_preview.dll")
)

$dllPath = $null
foreach ($path in $searchPaths) {
    if (Test-Path $path -ErrorAction SilentlyContinue) {
        $dllPath = $path
        break
    }
}

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This script must be run as Administrator." -ForegroundColor Red
    Write-Host "Right-click PowerShell and select 'Run as Administrator'" -ForegroundColor Yellow
    exit 1
}

# Check if DLL exists
if (-not $dllPath) {
    Write-Host "ERROR: DLL not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Searched in:" -ForegroundColor Yellow
    foreach ($path in $searchPaths) {
        Write-Host "  - $path" -ForegroundColor Gray
    }
    Write-Host ""
    Write-Host "Please build the project first:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  cd $projectRoot" -ForegroundColor Cyan
    Write-Host "  .\scripts\build-preview-handler.ps1" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Or build manually:" -ForegroundColor Yellow
    Write-Host "  cargo build --release -p eulumdat-preview --target $targetTriple" -ForegroundColor Cyan
    exit 1
}

Write-Host ""
Write-Host "Eulumdat Preview Handler" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan
Write-Host "Architecture: $Architecture ($targetTriple)" -ForegroundColor White
Write-Host "DLL: $dllPath" -ForegroundColor White
Write-Host ""

if ($Uninstall) {
    Write-Host "Unregistering..." -ForegroundColor Yellow

    $result = Start-Process -FilePath "regsvr32.exe" -ArgumentList "/u", "/s", "`"$dllPath`"" -Wait -PassThru -NoNewWindow

    if ($result.ExitCode -eq 0) {
        Write-Host "Successfully unregistered!" -ForegroundColor Green
    } else {
        Write-Host "regsvr32 returned exit code: $($result.ExitCode)" -ForegroundColor Red
        Write-Host "Try manual unregister: regsvr32 /u `"$dllPath`"" -ForegroundColor Yellow
    }
} else {
    Write-Host "Registering..." -ForegroundColor Yellow

    $result = Start-Process -FilePath "regsvr32.exe" -ArgumentList "/s", "`"$dllPath`"" -Wait -PassThru -NoNewWindow

    if ($result.ExitCode -eq 0) {
        Write-Host "Successfully registered!" -ForegroundColor Green
        Write-Host ""
        Write-Host "You can now preview .ldt and .ies files in File Explorer!" -ForegroundColor Green
        Write-Host ""
        Write-Host "Usage:" -ForegroundColor Cyan
        Write-Host "  1. Open File Explorer" -ForegroundColor White
        Write-Host "  2. Press Alt+P to show the preview pane" -ForegroundColor White
        Write-Host "  3. Select a .ldt or .ies file" -ForegroundColor White
    } else {
        Write-Host "regsvr32 returned exit code: $($result.ExitCode)" -ForegroundColor Red
        Write-Host ""
        Write-Host "Try manual register for more details:" -ForegroundColor Yellow
        Write-Host "  regsvr32 `"$dllPath`"" -ForegroundColor Cyan
    }
}

Write-Host ""
Write-Host "Note: You may need to restart Explorer for changes to take effect:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Name explorer -Force; Start-Process explorer" -ForegroundColor Cyan
Write-Host ""
