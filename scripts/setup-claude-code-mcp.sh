#!/bin/bash
# Auto-setup NetProxy MCP for Claude Code CLI

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MCP_BINARY="$PROJECT_ROOT/target/release/np-mcp"
DATA_DIR="$HOME/Library/Application Support/NetProxy"

# Config locations
PROJECT_CONFIG="$PROJECT_ROOT/.claude/settings.json"
USER_CONFIG="$HOME/.claude/settings.json"

echo "🔧 NetProxy MCP Setup for Claude Code"
echo "======================================="
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

# Step 2: Choose config location
echo "2️⃣ Choose configuration scope:"
echo ""
echo "   A) Project-level (recommended for development)"
echo "      - Only active in this NetProxy project"
echo "      - Uses test-data directory"
echo "      - Can be committed to git"
echo ""
echo "   B) User-level (for global use)"
echo "      - Active in all Claude Code sessions"
echo "      - Uses production data directory"
echo "      - Not committed to git"
echo ""
echo -n "   Choose [A/B] (default: A): "
read -r CHOICE

CHOICE=${CHOICE:-A}

case "$CHOICE" in
    [Aa]*)
        CONFIG_FILE="$PROJECT_CONFIG"
        CONFIG_TYPE="project"
        USE_DATA_DIR="$PROJECT_ROOT/test-data"
        ;;
    [Bb]*)
        CONFIG_FILE="$USER_CONFIG"
        CONFIG_TYPE="user"
        USE_DATA_DIR="$DATA_DIR"
        ;;
    *)
        echo "   Invalid choice. Using project-level."
        CONFIG_FILE="$PROJECT_CONFIG"
        CONFIG_TYPE="project"
        USE_DATA_DIR="$PROJECT_ROOT/test-data"
        ;;
esac

echo ""
echo "   Selected: $CONFIG_TYPE-level configuration"
echo "   Config file: $CONFIG_FILE"
echo "   Data directory: $USE_DATA_DIR"

echo ""

# Step 3: Create config directory
CONFIG_DIR="$(dirname "$CONFIG_FILE")"
if [ ! -d "$CONFIG_DIR" ]; then
    echo "   Creating directory: $CONFIG_DIR"
    mkdir -p "$CONFIG_DIR"
fi

# Step 4: Create or update config
echo ""
echo "3️⃣ Configuring Claude Code..."

if [ -f "$CONFIG_FILE" ]; then
    echo "   ⚠️  Config file already exists!"
    echo "   Location: $CONFIG_FILE"
    echo ""

    # Check if netproxy already exists
    if grep -q '"netproxy"' "$CONFIG_FILE" 2>/dev/null; then
        echo "   ⚠️  NetProxy MCP already configured!"
        echo ""
        echo "   Current configuration:"
        cat "$CONFIG_FILE"
        echo ""
        echo "   Do you want to update it? [y/N]: "
        read -r UPDATE
        if [[ ! "$UPDATE" =~ ^[Yy]$ ]]; then
            echo "   Skipping configuration update."
            exit 0
        fi

        # Backup
        cp "$CONFIG_FILE" "$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
        echo "   ✅ Backup created"
    fi

    # Merge with existing config
    if command -v jq >/dev/null 2>&1; then
        echo "   Updating configuration with jq..."
        TMP=$(mktemp)

        # Use workspace folder variable for project config
        if [ "$CONFIG_TYPE" = "project" ]; then
            CMD_PATH="\${workspaceFolder}/target/release/np-mcp"
            DATA_PATH="\${workspaceFolder}/test-data"
        else
            CMD_PATH="$MCP_BINARY"
            DATA_PATH="$USE_DATA_DIR"
        fi

        jq --arg cmd "$CMD_PATH" --arg datadir "$DATA_PATH" \
            '.mcpServers.netproxy = {
                "command": $cmd,
                "args": [],
                "env": {
                    "NETPROXY_DATA_DIR": $datadir
                }
            }' "$CONFIG_FILE" > "$TMP"
        mv "$TMP" "$CONFIG_FILE"
        echo "   ✅ Configuration updated!"
    else
        echo "   ⚠️  jq not found - creating new config"
        create_new_config
    fi
else
    echo "   Creating new configuration file..."
    create_new_config
    echo "   ✅ Configuration created!"
fi

echo ""

# Step 5: Verify
echo "4️⃣ Verifying setup..."
echo ""
echo "   Config file:    $CONFIG_FILE"
echo "   MCP binary:     $MCP_BINARY"
echo "   Data directory: $USE_DATA_DIR"
echo ""

if [ -f "$CONFIG_FILE" ]; then
    echo "   ✅ Config file exists"
    echo ""
    echo "   Current configuration:"
    cat "$CONFIG_FILE"
else
    echo "   ❌ Config file not found!"
    exit 1
fi

if [ -f "$MCP_BINARY" ]; then
    echo ""
    echo "   ✅ MCP binary exists"
    chmod +x "$MCP_BINARY"
else
    echo ""
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
if [ "$CONFIG_TYPE" = "project" ]; then
    echo "1. Navigate to this project:"
    echo "   cd $PROJECT_ROOT"
    echo ""
fi
echo "2. Start Claude Code:"
echo "   claude"
echo ""
echo "3. Test MCP tools:"
echo '   > "List all NetProxy MCP tools available"'
echo ""
echo "🎯 Example commands to try:"
echo '  > "Start NetProxy on port 8899"'
echo '  > "Set up my Mac to use NetProxy as system proxy"'
echo '  > "Show me the last 10 captured requests"'
echo '  > "Create a mock for /api/users"'
echo ""
echo "📚 Documentation:"
echo "  $PROJECT_ROOT/docs/CLAUDE-CODE-MCP-SETUP.md"
echo "  $PROJECT_ROOT/docs/MCP-PROXY-SETUP.md"
echo ""
echo "🔍 View your config:"
echo "  cat '$CONFIG_FILE'"
echo ""

function create_new_config() {
    # Use workspace folder variable for project config
    if [ "$CONFIG_TYPE" = "project" ]; then
        CMD_PATH="\${workspaceFolder}/target/release/np-mcp"
        DATA_PATH="\${workspaceFolder}/test-data"

        cat > "$CONFIG_FILE" <<EOF
{
  "mcpServers": {
    "netproxy": {
      "command": "$CMD_PATH",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "$DATA_PATH"
      }
    }
  },
  "permissions": {
    "allow": [
      "mcp:netproxy:*"
    ]
  }
}
EOF
    else
        cat > "$CONFIG_FILE" <<EOF
{
  "mcpServers": {
    "netproxy": {
      "command": "$MCP_BINARY",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "$USE_DATA_DIR"
      }
    }
  }
}
EOF
    fi
}
