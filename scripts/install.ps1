# =============================================================================
#  Dev Manager (`dm`) — oneliner-установщик для Windows (PowerShell).
#
#  Запуск:
#    iwr -useb https://raw.githubusercontent.com/your-org/dev_manager/main/scripts/install.ps1 | iex
#
#  Что делает:
#    1. Скачивает dm-<arch>-pc-windows-msvc.zip с последнего релиза GitHub.
#    2. Распаковывает dm.exe в %LOCALAPPDATA%\Programs\dm.
#    3. Добавляет каталог в пользовательский PATH (постоянно + текущая сессия),
#       если его там ещё нет.
# =============================================================================
$ErrorActionPreference = 'Stop'

# Замените на ваш owner/repo перед публикацией.
$Repo = "your-org/dev_manager"

$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\dm'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Arch = if ([Environment]::Is64BitOperatingSystem) { 'x86_64' } else { 'x86' }
$Url  = "https://github.com/$Repo/releases/latest/download/dm-$Arch-pc-windows-msvc.zip"
$Zip  = Join-Path $env:TEMP "dm-install.zip"

Write-Host "→ Загрузка dm ($Arch) из последнего релиза $Repo…"
Invoke-WebRequest -Uri $Url -OutFile $Zip -UseBasicParsing
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
Remove-Item $Zip

# Регистрируем в PATH (пользователь), если ещё нет.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $InstallDir) {
    $new = if ($userPath) { "$userPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    # Обновляем PATH текущей сессии, чтобы dm стал доступен сразу.
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "✓ PATH обновлён."
} else {
    Write-Host "• PATH уже содержит $InstallDir."
}

$Exe = Join-Path $InstallDir 'dm.exe'
Write-Host "✓ Установлено: $Exe"
Write-Host "  Перезапустите терминал, чтобы команда 'dm' стала доступна."
& $Exe --version
