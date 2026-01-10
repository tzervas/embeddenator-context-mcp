#!/bin/bash
# Installation script for context-mcp MCP server
# This script installs context-mcp globally and configures it for VS Code

set -e

BINARY_NAME="context-mcp"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"

echo "========================================"
echo "  Context-MCP Installation Script"
echo "========================================"
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)     PLATFORM="linux";;
    Darwin*)    PLATFORM="macos";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

echo "Platform detected: $PLATFORM"

# Step 1: Create installation directory
echo ""
echo "[1/5] Creating installation directory..."
mkdir -p "$INSTALL_DIR"
echo "✓ Created: $INSTALL_DIR"

# Step 2: Build or install binary
echo ""
echo "[2/5] Building context-mcp..."
if [ -f "Cargo.toml" ]; then
    # If we're in the source directory, build it
    echo "Building from source..."
    cargo build --release
    cp target/release/context-mcp "$BINARY_PATH"
elif command -v cargo &> /dev/null; then
    # Install from crates.io
    echo "Installing from crates.io..."
    cargo install context-mcp --root "$HOME/.local"
else
    echo "Error: Either run this script from the source directory or install Rust first"
    echo "Visit: https://rustup.rs"
    exit 1
fi

chmod +x "$BINARY_PATH"
echo "✓ Binary installed: $BINARY_PATH"

# Step 3: Configure PATH
echo ""
echo "[3/5] Configuring PATH..."

SHELL_RC=""
if [ -n "$BASH_VERSION" ]; then
    SHELL_RC="$HOME/.bashrc"
elif [ -n "$ZSH_VERSION" ]; then
    SHELL_RC="$HOME/.zshrc"
else
    # Try to detect shell
    CURRENT_SHELL=$(basename "$SHELL")
    case "$CURRENT_SHELL" in
        bash) SHELL_RC="$HOME/.bashrc";;
        zsh)  SHELL_RC="$HOME/.zshrc";;
        fish) SHELL_RC="$HOME/.config/fish/config.fish";;
        *)    SHELL_RC="$HOME/.profile";;
    esac
fi

# Add PATH export if not already present
if ! grep -q "\.local/bin.*PATH" "$SHELL_RC" 2>/dev/null; then
    echo "" >> "$SHELL_RC"
    echo "# Added by context-mcp installer" >> "$SHELL_RC"
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"
    echo "✓ Added PATH to $SHELL_RC"
else
    echo "✓ PATH already configured in $SHELL_RC"
fi

# Make PATH available in current session
export PATH="$HOME/.local/bin:$PATH"

# Step 4: Verify installation
echo ""
echo "[4/5] Verifying installation..."
if "$BINARY_PATH" --version &> /dev/null; then
    VERSION=$("$BINARY_PATH" --version 2>&1 || echo "unknown")
    echo "✓ Installation verified: $VERSION"
else
    echo "✗ Installation verification failed"
    exit 1
fi

# Step 5: Configure VS Code (optional)
echo ""
echo "[5/5] VS Code MCP Configuration"
echo ""
echo "To use context-mcp with VS Code, add this to your MCP settings:"
echo ""
echo "File: ~/.config/Code/User/profiles/<your-profile>/mcp.json"
echo "(or: ~/Library/Application Support/Code/User/profiles/<profile>/mcp.json on macOS)"
echo ""
cat <<'EOF'
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "$HOME/.local/bin/context-mcp",
      "args": ["--stdio"]
    }
  }
}
EOF

echo ""
echo "========================================"
echo "  Installation Complete! ✓"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Restart your terminal or run: source $SHELL_RC"
echo "  2. Test with: context-mcp --help"
echo "  3. Configure VS Code MCP (see above)"
echo "  4. Reload VS Code window after configuration"
echo ""
echo "Documentation:"
echo "  - Usage examples: USAGE_EXAMPLES.md"
echo "  - Test results: ASSESSMENT_REPORT.md"
echo "  - GitHub: https://github.com/tzervas/context-mcp"
echo ""
