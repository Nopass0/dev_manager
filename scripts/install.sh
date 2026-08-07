#!/usr/bin/env sh
set -e
REPO="Nopass0/dev_manager"
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
  Linux*)  os="unknown-linux-musl" ;;
  Darwin*) os="apple-darwin" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac
case "$ARCH" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
echo "Downloading dm ($arch-$os) from $REPO..."
ARCHIVE="/tmp/dm-$$.tar.gz"
URL="https://github.com/$REPO/releases/latest/download/dm-$arch-$os.tar.gz"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$ARCHIVE"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$ARCHIVE" "$URL"
else
  echo "curl or wget required." >&2; exit 1
fi
tar -xzf "$ARCHIVE" -C "$BIN_DIR"
chmod +x "$BIN_DIR/dm"
rm -f "$ARCHIVE"
register() {
  rc="$1"
  [ -f "$rc" ] || return 0
  grep -q '.local/bin' "$rc" && return 0
  printf '\n# Dev Manager\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$rc"
  echo "PATH updated in $rc"
}
register "$HOME/.bashrc"
register "$HOME/.zshrc"
register "$HOME/.profile"
echo "Installed: $BIN_DIR/dm"
echo "Restart your terminal or run: export PATH=\"$HOME/.local/bin:\$PATH\""
"$BIN_DIR/dm" --version 2>/dev/null || true
