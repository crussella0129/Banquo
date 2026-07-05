<#
.SYNOPSIS
    Build Banquo in release mode and install it as a standalone application.

.DESCRIPTION
    Banquo is a real terminal emulator — once installed you launch it like any
    other app, with no console window and no `cargo run` from the source tree.

    This script:
      1. Builds the optimized release binary (`cargo build --release`).
      2. Copies `banquo.exe` to a stable install location.
      3. Creates a Start-menu (and optional Desktop) shortcut.

    If the build fails, the script aborts before installing anything.

.PARAMETER InstallDir
    Where to copy banquo.exe. Default: %LOCALAPPDATA%\Banquo.

.PARAMETER Desktop
    Also create a Desktop shortcut.

.PARAMETER AddToPath
    Add the install directory to the user PATH so `banquo` works from any shell.

.EXAMPLE
    .\install.ps1
    .\install.ps1 -Desktop -AddToPath
#>
[CmdletBinding()]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Banquo'),
    [switch]$Desktop,
    [switch]$AddToPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = $PSScriptRoot

Write-Host 'Banquo installer' -ForegroundColor Cyan
Write-Host '================'

# 1. Build (release). Abort the whole install if this fails.
Write-Host "`n[1/4] Building release binary (cargo build --release)..." -ForegroundColor Yellow
Push-Location $repoRoot
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed (exit $LASTEXITCODE). Aborting - nothing was installed."
    }
}
finally {
    Pop-Location
}

$builtExe = Join-Path $repoRoot 'target\release\banquo.exe'
if (-not (Test-Path $builtExe)) {
    throw "Build reported success but $builtExe is missing. Aborting."
}

# 2. Copy to the install directory.
Write-Host "`n[2/4] Installing to $InstallDir ..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$targetExe = Join-Path $InstallDir 'banquo.exe'
Copy-Item -Path $builtExe -Destination $targetExe -Force
Write-Host "  Copied banquo.exe -> $targetExe"

# 3. Create shortcut(s) via WScript.Shell (plain COM automation - no unsafe code).
Write-Host "`n[3/4] Creating shortcut(s)..." -ForegroundColor Yellow
$wsh = New-Object -ComObject WScript.Shell

function New-BanquoShortcut([string]$LinkPath) {
    $sc = $wsh.CreateShortcut($LinkPath)
    $sc.TargetPath = $targetExe
    $sc.WorkingDirectory = $InstallDir
    $sc.Description = 'Banquo - a most beautiful terminal'
    $sc.Save()
    Write-Host "  Shortcut: $LinkPath"
}

$startMenuDir = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
New-BanquoShortcut (Join-Path $startMenuDir 'Banquo.lnk')

if ($Desktop) {
    New-BanquoShortcut (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Banquo.lnk')
}

# Optional: put the install dir on the user PATH so `banquo` works everywhere.
if ($AddToPath) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($userPath -split ';') -notcontains $InstallDir) {
        $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$userPath;$InstallDir" }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Host "  Added $InstallDir to user PATH (restart shells to pick it up)."
    }
    else {
        Write-Host "  $InstallDir already on user PATH."
    }
}

# 4. Bootstrap default configuration if missing
Write-Host "`n[4/4] Setting up default configuration..." -ForegroundColor Yellow
$configDir = Join-Path $env:APPDATA 'banquo'
$configFile = Join-Path $configDir 'banquo.toml'

if (-not (Test-Path $configFile)) {
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
    $defaultTheme = Join-Path $repoRoot 'configs\zircon.toml'
    if (Test-Path $defaultTheme) {
        Copy-Item -Path $defaultTheme -Destination $configFile -Force
        Write-Host "  Created default config at $configFile (using Zircon theme)"
    } else {
        Write-Host "  Warning: $defaultTheme not found, skipping default config setup." -ForegroundColor Red
    }
} else {
    Write-Host "  Existing config found at $configFile, preserving."
}

Write-Host "`nDone. Launch Banquo from the Start menu, or run:" -ForegroundColor Green
Write-Host "  $targetExe"
