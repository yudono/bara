#!/usr/bin/env bash
set -e

echo "=========================================="
echo "   ⚡ Installing Bara Compiler & CLI      "
echo "=========================================="

CARGO_BIN="$HOME/.cargo/bin"

if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed. Please install Rust and Cargo first (https://rustup.rs)."
    exit 1
fi

echo "Building and installing Bara binary..."
if cargo install --path . --offline 2>/dev/null; then
    echo "✓ Installed Bara offline successfully."
else
    echo "Installing Bara from local path..."
    cargo install --path .
fi

if [[ ":$PATH:" != *":$CARGO_BIN:"* ]]; then
    echo ""
    echo "Notice: $CARGO_BIN is not in your current PATH."
    echo "To make 'bara' available in all terminal sessions, run:"
    echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
    echo ""
    if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zshrc" ]; then
        if ! grep -q '\.cargo/bin' "$HOME/.zshrc" 2>/dev/null; then
            echo "export PATH=\"\$HOME/.cargo/bin:\$PATH\"" >> "$HOME/.zshrc"
            echo "✓ Added ~/.cargo/bin to ~/.zshrc"
        fi
    fi
fi

echo ""
echo "=========================================="
if command -v bara &> /dev/null; then
    bara --version
    echo "✓ Bara is ready to use!"
else
    "$CARGO_BIN/bara" --version
    echo "✓ Installed to $CARGO_BIN/bara"
    echo "Run 'source ~/.zshrc' or reopen your terminal."
fi
echo "=========================================="
echo "Quickstart:"
echo "  bara init my_app"
echo "  bara check examples/todo_app/main.br"
echo "  bara run examples/cli_algorithms/main.br"
echo "=========================================="
