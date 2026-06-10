#!/bin/sh
set -e

# Determine OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64)
        TARGET="aarch64-apple-darwin"
        ;;
      x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
      *)
        echo "Unsupported macOS architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      *)
        echo "Unsupported Linux architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported operating system: $OS" >&2
    exit 1
    ;;
esac

# Determine version (default: latest)
VERSION="${VERSION:-latest}"

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/doggy8088/subembed/releases/latest/download/subembed-${TARGET}.tar.xz"
else
  # Ensure version starts with 'v' if it doesn't already
  case "$VERSION" in
    v*) ;;
    *) VERSION="v$VERSION" ;;
  esac
  URL="https://github.com/doggy8088/subembed/releases/download/${VERSION}/subembed-${TARGET}.tar.xz"
fi

# Create temporary directory
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading subembed from $URL..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$TMP_DIR/subembed.tar.xz"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TMP_DIR/subembed.tar.xz" "$URL"
else
  echo "Error: curl or wget is required to download subembed." >&2
  exit 1
fi

echo "Extracting..."
tar -xf "$TMP_DIR/subembed.tar.xz" -C "$TMP_DIR"

# Determine installation directory
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
  # If /usr/local/bin is not writable, check if we can write to ~/.local/bin or try sudo
  if [ -d "$HOME/.local/bin" ] && [ -w "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    USE_SUDO=false
  elif [ -d "$HOME/bin" ] && [ -w "$HOME/bin" ]; then
    INSTALL_DIR="$HOME/bin"
    USE_SUDO=false
  else
    USE_SUDO=true
  fi
else
  USE_SUDO=false
fi

echo "Installing subembed to $INSTALL_DIR..."
if [ "$USE_SUDO" = "true" ]; then
  if command -v sudo >/dev/null 2>&1; then
    sudo mv "$TMP_DIR/subembed" "$INSTALL_DIR/subembed"
    sudo chmod +x "$INSTALL_DIR/subembed"
  else
    echo "Error: $INSTALL_DIR is not writable and sudo is not available. Please run the script as root or create $HOME/.local/bin first." >&2
    exit 1
  fi
else
  mkdir -p "$INSTALL_DIR"
  mv "$TMP_DIR/subembed" "$INSTALL_DIR/subembed"
  chmod +x "$INSTALL_DIR/subembed"
fi

echo "subembed installed successfully to $INSTALL_DIR/subembed!"
