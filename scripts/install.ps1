# =============================================================================
#  Dev Manager (dm) — Windows installer (PowerShell)
#
#  Usage:
#    iwr -useb https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.ps1 | iex
#
#  What it does:
#    1. Downloads dm-<arch>-pc-windows-msvc.zip from the latest GitHub release.
#    2. Extracts dm.exe to %LOCALAPPDATA%\Programs\dm.
#    3. Adds the directory to the user PATH (persistent + current session).
# =============================================================================
$ErrorActionPreference = 'Stop'

$Repo = "Nopass0/dev_manager"

$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\dm'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x86_64' } else { 'x86' }
$Url  = "https://github.com/$Repo/releases/latest/download/dm-$Arch-pc-windows-msvc.zip"
$Zip  = Join-Path $env:TEMP "dm-install.zip"

Write-Host "Downloading dm ($Arch) from $Repo..."
Invoke-WebRequest -Uri $Url -OutFile $Zip -UseBasicParsing
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
Remove-Item $Zip

# Add to user PATH if not already there.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $InstallDir) {
    $new = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "PATH updated."
} else {
    Write-Host "PATH already contains $InstallDir."
}

$Exe = Join-Path $InstallDir 'dm.exe'
Write-Host "Installed: $Exe"
Write-Host "Restart your terminal for 'dm' command to become available."
& $Exe --version
