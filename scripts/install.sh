#!/usr/bin/env sh
# =============================================================================
#  Dev Manager (`dm`) — oneliner-установщик для Linux / macOS.
#
#  Запуск:
#    curl -fsSL https://raw.githubusercontent.com/Nopass0/dev_manager/main/scripts/install.sh | sh
#
#  Что делает:
#    1. Определяет ОС и архитектуру.
#    2. Скачивает архив dm-<os>-<arch>.tar.gz с последнего релиза GitHub.
#    3. Распаковывает `dm` в ~/.local/bin.
#    4. Добавляет ~/.local/bin в PATH (через ~/.bashrc / ~/.zshrc / ~/.profile),
#       если каталог там ещё не зарегистрирован.
# =============================================================================
set -e

# Замените на ваш owner/repo перед публикацией.
REPO="Nopass0/dev_manager"

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

echo "→ Загрузка dm ($arch-$os) из последнего релиза $REPO…"
ARCHIVE="/tmp/dm-$$.tar.gz"
URL="https://github.com/$REPO/releases/latest/download/dm-$arch-$os.tar.gz"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$ARCHIVE"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$ARCHIVE" "$URL"
else
  echo "✗ Нужен curl или wget." >&2
  exit 1
fi

tar -xzf "$ARCHIVE" -C "$BIN_DIR"
chmod +x "$BIN_DIR/dm"
rm -f "$ARCHIVE"

# Регистрируем ~/.local/bin в PATH, если ещё не зарегистрирован.
register() {
  rc="$1"
  [ -f "$rc" ] || return 0
  grep -q '.local/bin' "$rc" && return 0
  printf '\n# Dev Manager\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$rc"
  echo "✓ PATH обновлён в $rc"
}
register "$HOME/.bashrc"
register "$HOME/.zshrc"
register "$HOME/.profile"

echo "✓ Установлено: $BIN_DIR/dm"
echo "  Перезапустите терминал или выполните:"
echo "    export PATH=\"$HOME/.local/bin:\$PATH\""
"$BIN_DIR/dm" --version 2>/dev/null || true
