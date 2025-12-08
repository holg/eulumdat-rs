# Build Eulumdat Preview Handler for Windows
# Builds for both x64 and ARM64 architectures

param(
    [switch]$Debug,
    [switch]$X64Only,
    [switch]$Arm64Only
)

$ErrorActionPreference = "Continue"

$projectRoot = Join-Path $PSScriptRoot ".."
Push-Location $projectRoot

try {
    $buildType = if ($Debug) { "" } else { "--release" }
    $buildTypeLabel = if ($Debug) { "debug" } else { "release" }

    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " Eulumdat Preview Handler Build Script" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""

    # Check for cargo
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Host "ERROR: cargo not found. Please install Rust from https://rustup.rs" -ForegroundColor Red
        exit 1
    }

    # Check for rustup
    $rustup = Get-Command rustup -ErrorAction SilentlyContinue
    if (-not $rustup) {
        Write-Host "WARNING: rustup not found. Cannot auto-install targets." -ForegroundColor Yellow
    }

    # Determine which architectures to build
    $buildX64 = -not $Arm64Only
    $buildArm64 = -not $X64Only

    $success = $true

    # Build for x64
    if ($buildX64) {
        Write-Host "Building for x64 (x86_64-pc-windows-msvc)..." -ForegroundColor Yellow

        # Ensure target is installed (suppress all output)
        if ($rustup) {
            & rustup target add x86_64-pc-windows-msvc 2>&1 | Out-Null
        }

        $args = @("build", "-p", "eulumdat-preview", "--target", "x86_64-pc-windows-msvc")
        if (-not $Debug) { $args += "--release" }

        Write-Host "  cargo $($args -join ' ')" -ForegroundColor Gray

        & cargo @args

        if ($LASTEXITCODE -eq 0) {
            $dllPath = "target\x86_64-pc-windows-msvc\$buildTypeLabel\eulumdat_preview.dll"
            if (Test-Path $dllPath) {
                $size = (Get-Item $dllPath).Length / 1MB
                Write-Host "  OK: $dllPath ($([math]::Round($size, 2)) MB)" -ForegroundColor Green
            } else {
                Write-Host "  Built but DLL not found at expected path" -ForegroundColor Yellow
            }
        } else {
            Write-Host "  FAILED: Build returned exit code $LASTEXITCODE" -ForegroundColor Red
            $success = $false
        }
        Write-Host ""
    }

    # Build for ARM64
    if ($buildArm64) {
        Write-Host "Building for ARM64 (aarch64-pc-windows-msvc)..." -ForegroundColor Yellow

        # Ensure target is installed (suppress all output)
        if ($rustup) {
            & rustup target add aarch64-pc-windows-msvc 2>&1 | Out-Null
        }

        $args = @("build", "-p", "eulumdat-preview", "--target", "aarch64-pc-windows-msvc")
        if (-not $Debug) { $args += "--release" }

        Write-Host "  cargo $($args -join ' ')" -ForegroundColor Gray

        & cargo @args

        if ($LASTEXITCODE -eq 0) {
            $dllPath = "target\aarch64-pc-windows-msvc\$buildTypeLabel\eulumdat_preview.dll"
            if (Test-Path $dllPath) {
                $size = (Get-Item $dllPath).Length / 1MB
                Write-Host "  OK: $dllPath ($([math]::Round($size, 2)) MB)" -ForegroundColor Green
            } else {
                Write-Host "  Built but DLL not found at expected path" -ForegroundColor Yellow
            }
        } else {
            Write-Host "  FAILED: Build returned exit code $LASTEXITCODE" -ForegroundColor Red
            $success = $false
        }
        Write-Host ""
    }

    # Summary
    if ($success) {
        Write-Host "========================================" -ForegroundColor Green
        Write-Host " Build Complete!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "To install, run as Administrator:" -ForegroundColor Cyan
        Write-Host "  .\scripts\install-preview-handler.ps1" -ForegroundColor White
        Write-Host ""
        Write-Host "Or specify architecture:" -ForegroundColor Cyan
        Write-Host "  .\scripts\install-preview-handler.ps1 -Architecture x64" -ForegroundColor White
        Write-Host "  .\scripts\install-preview-handler.ps1 -Architecture arm64" -ForegroundColor White
    } else {
        Write-Host "========================================" -ForegroundColor Red
        Write-Host " Build Failed!" -ForegroundColor Red
        Write-Host "========================================" -ForegroundColor Red
        Write-Host ""
        Write-Host "Check the error messages above." -ForegroundColor Yellow
        exit 1
    }

} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
    Write-Host $_.ScriptStackTrace -ForegroundColor Gray
    exit 1
} finally {
    Pop-Location
}
