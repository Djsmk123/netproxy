#!/bin/bash
# Auto-setup NetProxy MCP for Claude Desktop

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLAUDE_CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
MCP_BINARY="$PROJECT_ROOT/target/release/np-mcp"
DATA_DIR="$HOME/Library/Application Support/NetProxy"

echo "🔧 NetProxy MCP Setup for Claude Desktop"
echo "=========================================="
echo ""

# Step 1: Build MCP server
echo "1️⃣ Building MCP server..."
if [ ! -f "$MCP_BINARY" ]; then
    echo "   Building release binary..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin np-mcp
    echo "   ✅ Built successfully!"
else
    echo "   ✅ Binary already exists: $MCP_BINARY"
fi

echo ""

# Step 2: Create Claude config directory
echo "2️⃣ Setting up Claude Desktop configuration..."
CLAUDE_DIR="$(dirname "$CLAUDE_CONFIG")"
if [ ! -d "$CLAUDE_DIR" ]; then
    echo "   Creating directory: $CLAUDE_DIR"
    mkdir -p "$CLAUDE_DIR"
fi

# Step 3: Create or update config
echo ""
if [ -f "$CLAUDE_CONFIG" ]; then
    echo "   ⚠️  Config file already exists!"
    echo "   Location: $CLAUDE_CONFIG"
    echo ""
    echo "   I'll create a backup and add NetProxy configuration."
    echo "   Press Enter to continue, or Ctrl+C to cancel..."
    read

    # Backup existing config
    cp "$CLAUDE_CONFIG" "$CLAUDE_CONFIG.backup.$(date +%Y%m%d_%H%M%S)"
    echo "   ✅ Backup created"

    # Check if netproxy already exists in config
    if grep -q '"netproxy"' "$CLAUDE_CONFIG"; then
        echo "   ⚠️  NetProxy MCP already configured!"
        echo "   Skipping configuration update."
    else
        echo "   Adding NetProxy to existing config..."
        # Use jq if available, otherwise manual edit
        if command -v jq >/dev/null 2>&1; then
            TMP=$(mktemp)
            jq --arg cmd "$MCP_BINARY" --arg datadir "$DATA_DIR" \
                '.mcpServers.netproxy = {
                    "command": $cmd,
                    "args": [],
                    "env": {
                        "NETPROXY_DATA_DIR": $datadir
                    }
                }' "$CLAUDE_CONFIG" > "$TMP"
            mv "$TMP" "$CLAUDE_CONFIG"
            echo "   ✅ Added NetProxy to config!"
        else
            echo "   ⚠️  jq not found - please add manually:"
            echo ""
            show_config
        fi
    fi
else
    echo "   Creating new config file..."
    cat > "$CLAUDE_CONFIG" <<EOF
{
  "mcpServers": {
    "netproxy": {
      "command": "$MCP_BINARY",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "$DATA_DIR"
      }
    }
  }
}
EOF
    echo "   ✅ Config file created!"
fi

echo ""

# Step 4: Verify
echo "3️⃣ Verifying setup..."
echo ""
echo "   Config location: $CLAUDE_CONFIG"
echo "   MCP binary:      $MCP_BINARY"
echo "   Data directory:  $DATA_DIR"
echo ""

if [ -f "$CLAUDE_CONFIG" ]; then
    echo "   ✅ Config file exists"
else
    echo "   ❌ Config file not found!"
    exit 1
fi

if [ -f "$MCP_BINARY" ]; then
    echo "   ✅ MCP binary exists"
    chmod +x "$MCP_BINARY"
else
    echo "   ❌ MCP binary not found!"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Setup Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📝 Next Steps:"
echo ""
echo "1. Restart Claude Desktop (Quit completely and reopen)"
echo "2. Start a new conversation in Claude"
echo "3. Try: 'List all NetProxy MCP tools available'"
echo ""
echo "🎯 Example commands to try:"
echo "  - 'Start NetProxy on port 8899'"
echo "  - 'Set up my Mac to use NetProxy as system proxy'"
echo "  - 'Show me the last 10 captured requests'"
echo "  - 'Create a mock for /api/users that returns test data'"
echo ""
echo "📚 Full documentation:"
echo "  $PROJECT_ROOT/docs/CLAUDE-MCP-SETUP.md"
echo "  $PROJECT_ROOT/docs/MCP-PROXY-SETUP.md"
echo ""
echo "🔍 View your config:"
echo "  cat '$CLAUDE_CONFIG'"
echo ""

function show_config() {
    cat <<'EOF'
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy"
      }
    }
  }
}
EOF
}
