#!/bin/bash
set -e

echo "==> Installing chmer ..."

INSTALL_DIR="/usr/local/bin"

# Create install dir if missing
sudo mkdir -p "$INSTALL_DIR"

# Copy chmer script
sudo cp chmer "$INSTALL_DIR/chmer"
sudo chmod +x "$INSTALL_DIR/chmer"

# Install Stockfish if not present
if ! command -v stockfish &>/dev/null; then
    echo "==> Stockfish not found, installing..."
    if command -v apt &>/dev/null; then
        sudo apt update && sudo apt install -y stockfish
    elif command -v dnf &>/dev/null; then
        sudo dnf install -y stockfish
    elif command -v pacman &>/dev/null; then
        sudo pacman -Sy --noconfirm stockfish
    else
        echo "⚠️ Unsupported package manager. Please install Stockfish manually."
    fi
else
    echo "==> Stockfish already installed."
fi

echo "✅ chmer installed successfully!"
