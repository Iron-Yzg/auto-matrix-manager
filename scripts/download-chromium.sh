#!/bin/bash
# Download and setup embedded Chromium for browser automation
# 下载并设置内置 Chromium 浏览器

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHROMIUM_DIR="$SCRIPT_DIR/.chromium"

echo "Setting up embedded Chromium..."

# Create directory
mkdir -p "$CHROMIUM_DIR"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Darwin)
        CHROMIUM_URL="https://storage.googleapis.com/chromium-browser-snapshots/Mac/LAST_CHANGE"
        CHROMIUM_ZIP="$CHROMIUM_DIR/chrome-mac.zip"
        echo "Downloading Chromium for macOS..."
        # Download chromium
        curl -L -o "$CHROMIUM_ZIP" "https://storage.googleapis.com/chromium-browser-snapshots/Mac_x64/LAST_CHANGE"
        # For production, use official Google Chrome or download from other source
        echo "For macOS, we recommend installing Google Chrome from https://chrome.google.com"
        echo "Or install chromium: brew install chromium"
        rm -rf "$CHROMIUM_ZIP"
        ;;
    Linux)
        CHROMIUM_URL="https://download-chromium.appspot.com/dl/Linux_x64"
        CHROMIUM_ZIP="$CHROMIUM_DIR/chrome-linux.zip"
        echo "Downloading Chromium for Linux..."
        curl -L -o "$CHROMIUM_ZIP" "$CHROMIUM_URL"
        unzip -o "$CHROMIUM_ZIP" -d "$CHROMIUM_DIR"
        rm -f "$CHROMIUM_ZIP"
        chmod +x "$CHROMIUM_DIR/chrome-linux/chrome"
        echo "Chromium installed at: $CHROMIUM_DIR/chrome-linux/chrome"
        ;;
    MINGW*|CYGWIN*|MSYS*)
        echo "Windows detected. Please install Google Chrome from https://chrome.google.com"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Chromium setup complete!"
