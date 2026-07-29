#!/bin/sh
set -eu

# kuayle-cli installer — downloads the latest release binary.
# kuayle-cli 安装器 — 下载最新 release 二进制。

INSTALL_DIR="${KUAYLE_INSTALL_DIR:-$HOME/.local/bin}"
REPO="uiYzzi/kuayle_cli"
VERSION="${KUAYLE_VERSION:-latest}"

# Detect OS and arch.
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac
case "$OS" in
    linux) TARGET="${ARCH}-unknown-linux-musl" ;;
    darwin) TARGET="${ARCH}-apple-darwin" ;;
    *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

BINARY="kuayle-${TARGET}"
if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${BINARY}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}"
fi

echo "Installing kuayle to ${INSTALL_DIR}..."
mkdir -p "${INSTALL_DIR}"
curl -fsSL "${URL}" -o "${INSTALL_DIR}/kuayle"
chmod +x "${INSTALL_DIR}/kuayle"
echo "✓ kuayle installed to ${INSTALL_DIR}/kuayle"
echo ""
echo "Make sure ${INSTALL_DIR} is in your PATH."
