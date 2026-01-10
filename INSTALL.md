# Installation Guide

## Quick Start

### One-Line Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/tzervas/context-mcp/main/install.sh | bash
```

Or download and run:

```bash
wget https://raw.githubusercontent.com/tzervas/context-mcp/main/install.sh
chmod +x install.sh
./install.sh
```

### From Source

```bash
git clone https://github.com/tzervas/context-mcp.git
cd context-mcp
./install.sh
```

### From Crates.io

```bash
cargo install context-mcp
```

---

## VS Code MCP Configuration

After installation, configure VS Code to use context-mcp:

### 1. Locate Your MCP Configuration File

- **Linux**: `~/.config/Code/User/profiles/<profile>/mcp.json`
- **macOS**: `~/Library/Application Support/Code/User/profiles/<profile>/mcp.json`
- **Windows**: `%APPDATA%\Code\User\profiles\<profile>\mcp.json`

### 2. Add Context-MCP Server

Edit your `mcp.json` file to include:

```json
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "/home/<username>/.local/bin/context-mcp",
      "args": ["--stdio"]
    }
  }
}
```

**Important**: Replace `/home/<username>` with your actual home directory path, or use the full path from `which context-mcp`.

### 3. Reload VS Code

- Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS)
- Type "Developer: Reload Window"
- Press Enter

---

## Configuration Options

### Basic Configuration (Default)

```json
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "/home/<username>/.local/bin/context-mcp",
      "args": ["--stdio"]
    }
  }
}
```

### With Persistent Storage

```json
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "/home/<username>/.local/bin/context-mcp",
      "args": [
        "--stdio",
        "--persist",
        "--storage-path", "/home/<username>/.context-mcp/data"
      ]
    }
  }
}
```

### High Capacity Configuration

```json
{
  "servers": {
    "context-mcp": {
      "type": "stdio",
      "command": "/home/<username>/.local/bin/context-mcp",
      "args": [
        "--stdio",
        "--cache-size", "10000",
        "--threads", "0",
        "--persist"
      ]
    }
  }
}
```

### Available Options

| Option | Description | Default |
|--------|-------------|---------|
| `--stdio` | Use stdio transport (required for VS Code) | - |
| `--host <HOST>` | Server host (HTTP mode only) | 127.0.0.1 |
| `--port <PORT>` | Server port (HTTP mode only) | 3000 |
| `--storage-path <PATH>` | Path for persistent storage | - |
| `--cache-size <SIZE>` | Memory cache size | 1000 |
| `--persist` | Enable disk persistence | false |
| `--threads <N>` | Number of RAG threads (0 = auto) | 0 |
| `--no-decay` | Disable temporal decay scoring | false |

---

## Verification

### Test Installation

```bash
# Check version
context-mcp --version

# Show help
context-mcp --help

# Run test server (Ctrl+C to stop)
context-mcp --stdio
```

### Verify VS Code Integration

1. Open VS Code
2. Check if context-mcp tools are available in MCP tools menu
3. Try storing a test context
4. Query it back to verify functionality

### Run Comprehensive Tests

```bash
cd context-mcp
python3 test_mcp_server.py
```

This will run all 23 tests and show performance benchmarks.

---

## Troubleshooting

### Binary Not Found

```bash
# Check if installed
ls -lh ~/.local/bin/context-mcp

# Check PATH
echo $PATH | grep ".local/bin"

# Add to PATH manually
export PATH="$HOME/.local/bin:$PATH"

# Make permanent
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### VS Code Not Detecting Server

1. Verify binary path: `which context-mcp`
2. Update `mcp.json` with the full path
3. Reload VS Code window
4. Check VS Code developer console for errors

### Permission Denied

```bash
# Make binary executable
chmod +x ~/.local/bin/context-mcp
```

### Server Not Starting

Check for errors:
```bash
# Run manually to see error messages
~/.local/bin/context-mcp --stdio
```

Common issues:
- Missing dependencies (reinstall Rust/cargo)
- Port already in use (for HTTP mode)
- Incorrect PATH configuration

---

## Uninstallation

```bash
# Remove binary
rm ~/.local/bin/context-mcp

# Remove from shell RC file (if added)
# Edit ~/.bashrc or ~/.zshrc and remove context-mcp PATH export

# Remove VS Code configuration
# Edit mcp.json and remove context-mcp server entry

# Remove persistent data (if enabled)
rm -rf ~/.context-mcp
```

---

## Platform-Specific Notes

### Linux

- Default install location: `~/.local/bin`
- Shell config: `~/.bashrc` or `~/.zshrc`
- VS Code config: `~/.config/Code/User/profiles/<profile>/mcp.json`

### macOS

- Default install location: `~/.local/bin`
- Shell config: `~/.zshrc` (Catalina+) or `~/.bash_profile`
- VS Code config: `~/Library/Application Support/Code/User/profiles/<profile>/mcp.json`

### Windows

- Use WSL2 (recommended) or native Windows build
- Binary location: `%USERPROFILE%\.local\bin\context-mcp.exe`
- VS Code config: `%APPDATA%\Code\User\profiles\<profile>\mcp.json`

---

## Next Steps

After installation:

1. **Read Usage Examples**: See [USAGE_EXAMPLES.md](USAGE_EXAMPLES.md)
2. **Review Capabilities**: See [README.md](README.md)
3. **Check Performance**: See [ASSESSMENT_REPORT.md](ASSESSMENT_REPORT.md)
4. **Start Using**: Store and retrieve contexts through VS Code

---

## Support

- **Issues**: https://github.com/tzervas/context-mcp/issues
- **Documentation**: https://docs.rs/context-mcp
- **Discussions**: https://github.com/tzervas/context-mcp/discussions
