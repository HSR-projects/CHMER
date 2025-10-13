#!/bin/bash
set -e

INSTALL_DIR="/usr/local/bin"
TARGET="$INSTALL_DIR/chmer"

echo "==> Uninstalling chmer ..."

# Remove chmer binary
if [ -f "$TARGET" ]; then
    sudo rm -f "$TARGET"
    echo "Removed $TARGET"
else
    echo "⚠️ chmer not found in $INSTALL_DIR"
fi

# Optionally remove Stockfish (ask user first)
if command -v stockfish &>/dev/null; then
    read -p "Do you also want to remove Stockfish? (y/N): " CONFIRM
    if [[ "$CONFIRM" =~ ^[Yy]$ ]]; then
        if command -v apt &>/dev/null; then
            sudo apt remove -y stockfish
        elif command -v dnf &>/dev/null; then
            sudo dnf remove -y stockfish
        elif command -v pacman &>/dev/null; then
            sudo pacman -Rns --noconfirm stockfish
        else
            echo "⚠️ Unsupported package manager. Please remove Stockfish manually."
        fi
    else
        echo "Stockfish left installed."
    fi
else
    echo "Stockfish not installed or already removed."
fi

echo "✅ Uninstall complete!"
