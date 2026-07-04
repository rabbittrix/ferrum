#!/usr/bin/env bash
# Ferrum Linux installer — builds CLI, adds to PATH, optional GUI deps hint.
set -euo pipefail

FERRUM_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${FERRUM_INSTALL_DIR:-/usr/local/bin}"
USE_USER_BIN=false

if [[ "${1:-}" == "--user" ]]; then
  USE_USER_BIN=true
  INSTALL_DIR="${HOME}/.local/bin"
fi

echo "==> Building Ferrum CLI (release)…"
cd "$FERRUM_ROOT"
cargo build --release -p ferrum-cli

BIN="$FERRUM_ROOT/target/release/ferrum"
if [[ ! -f "$BIN" ]]; then
  echo "Build failed: $BIN not found" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
echo "==> Installing ferrum to $INSTALL_DIR"
if [[ -w "$INSTALL_DIR" ]]; then
  cp "$BIN" "$INSTALL_DIR/ferrum"
  chmod +x "$INSTALL_DIR/ferrum"
else
  sudo cp "$BIN" "$INSTALL_DIR/ferrum"
  sudo chmod +x "$INSTALL_DIR/ferrum"
fi

path_line="export PATH=\"$INSTALL_DIR:\$PATH\""
shell_rc=""
case "${SHELL:-}" in
  */zsh) shell_rc="${HOME}/.zshrc" ;;
  *) shell_rc="${HOME}/.bashrc" ;;
esac

if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
  if [[ -f "$shell_rc" ]] && ! grep -Fq "$INSTALL_DIR" "$shell_rc" 2>/dev/null; then
    echo "" >> "$shell_rc"
    echo "# Ferrum CLI" >> "$shell_rc"
    echo "$path_line" >> "$shell_rc"
    echo "==> Added PATH entry to $shell_rc"
  else
    echo "==> Add to your shell profile: $path_line"
  fi
fi

if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null && ! pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
  echo ""
  echo "⚠ GUI build deps missing. Ubuntu/Debian:"
  echo "  sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev"
fi

echo ""
echo "✓ Ferrum installed. Run: ferrum doctor"
echo "  GUI build (on Linux): cd ferrum-gui && npm install && npm run tauri:build"
