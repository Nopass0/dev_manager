//! Генерация oneliner-скриптов установки из GitHub Releases.
//!
//! Эти же скрипты лежат как статические файлы в `scripts/install.sh` и
//! `scripts/install.ps1`. Здесь — их программные версии (например, для команды
//! `dm install --print-script`), чтобы не рассинхронизировать логику.

/// Имя owner/repo по умолчанию для ссылки на GitHub Releases.
///
/// Замените на реальные значения перед публикацией. Скрипты ниже подставляют
/// это значение в URL загрузки.
pub const DEFAULT_REPO: &str = "your-org/dev_manager";

/// Возвращает bash-скрипт установки (Linux/macOS).
///
/// Скрипт:
/// 1. Определяет архитектуру (x86_64/aarch64) и ОС.
/// 2. Скачивает архив `dm-<os>-<arch>.tar.gz` с последнего релиза.
/// 3. Распаковывает `dm` в `~/.local/bin`.
/// 4. Добавляет каталог в PATH через `~/.bashrc`/`~/.zshrc`.
pub fn bash_installer(repo: &str) -> String {
    format!(r#"#!/usr/bin/env sh
# Dev Manager (dm) — автоматический установщик для Linux/macOS.
# Запуск: curl -fsSL https://raw.githubusercontent.com/{repo}/main/scripts/install.sh | sh
set -e

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)  os="unknown-linux-musl" ;;
  Darwin*) os="apple-darwin" ;;
  *) echo "Неподдерживаемая ОС: $OS" >&2; exit 1 ;;
esac
case "$ARCH" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *) echo "Неподдерживаемая архитектура: $ARCH" >&2; exit 1 ;;
esac

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

echo "Загрузка dm ($arch-$os) из последнего релиза {repo}..."
ARCHIVE="/tmp/dm-$$.tar.gz"
# Берём ссылку на архив из latest release (формат имени файла: dm-<os>-<arch>.tar.gz).
URL="https://github.com/{repo}/releases/latest/download/dm-$arch-$os.tar.gz"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$ARCHIVE"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$ARCHIVE" "$URL"
else
  echo "Нужен curl или wget." >&2; exit 1
fi

tar -xzf "$ARCHIVE" -C "$BIN_DIR"
chmod +x "$BIN_DIR/dm"
rm -f "$ARCHIVE"

# Регистрируем ~/.local/bin в PATH, если ещё не зарегистрирован.
register() {{
  rc="$1"
  [ -f "$rc" ] || return 0
  grep -q '.local/bin' "$rc" && return 0
  printf '\n# Dev Manager\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$rc"
  echo "PATH обновлён в $rc"
}}
register "$HOME/.bashrc"
register "$HOME/.zshrc"
register "$HOME/.profile"

echo "Установлено: $BIN_DIR/dm"
echo "Перезапустите терминал или выполните: export PATH=\"$HOME/.local/bin:$PATH\""
"$BIN_DIR/dm" --version || true
"#)
}

/// Возвращает PowerShell-скрипт установки (Windows).
///
/// Скрипт:
/// 1. Скачивает `dm-x86_64-pc-windows-msvc.zip`.
/// 2. Распаковывает в `%LOCALAPPDATA%\Programs\dm`.
/// 3. Регистрирует каталог в пользовательском PATH (постоянно + текущая сессия).
pub fn powershell_installer(repo: &str) -> String {
    format!(r#"# Dev Manager (dm) — установщик для Windows (PowerShell).
# Запуск: iwr -useb https://raw.githubusercontent.com/{repo}/main/scripts/install.ps1 | iex
$ErrorActionPreference = 'Stop'

$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\dm'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Arch = if ([Environment]::Is64BitOperatingSystem) {{ 'x86_64' }} else {{ 'x86' }}
$Url = "https://github.com/{repo}/releases/latest/download/dm-$Arch-pc-windows-msvc.zip"
$Zip = Join-Path $env:TEMP "dm-install.zip"

Write-Host "Загрузка dm ($Arch) из последнего релиза {repo}..."
Invoke-WebRequest -Uri $Url -OutFile $Zip -UseBasicParsing
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
Remove-Item $Zip

# Регистрируем в PATH (пользователь), если ещё нет.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $InstallDir) {{
    $new = if ($userPath) {{ "$userPath;$InstallDir" }} else {{ $InstallDir }}
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    # Обновляем PATH текущей сессии.
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "PATH обновлён."
}} else {{
    Write-Host "PATH уже содержит $InstallDir."
}}

$Exe = Join-Path $InstallDir 'dm.exe'
Write-Host "Установлено: $Exe"
& $Exe --version
"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_installer_has_required_pieces() {
        let s = bash_installer("org/repo");
        assert!(s.contains("set -e"));
        assert!(s.contains("org/repo"));
        assert!(s.contains(".local/bin"));
        assert!(s.contains("tar -xzf"));
    }

    #[test]
    fn powershell_installer_has_required_pieces() {
        let s = powershell_installer("org/repo");
        assert!(s.contains("Expand-Archive"));
        assert!(s.contains("SetEnvironmentVariable"));
        assert!(s.contains("org/repo"));
        assert!(s.contains("Programs\\dm"));
    }
}
