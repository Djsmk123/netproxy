#!/bin/bash
# NetProxy Quick Start - One command to rule them all
# Sets up everything: daemon, proxy, CA cert, and optional mocking

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DATA_DIR="$HOME/Library/Application Support/NetProxy"
DAEMON_BIN="$PROJECT_ROOT/target/release/netproxyd"
CA_CERT="$HOME/Desktop/netproxy-ca.pem"
PROXY_PORT="${NETPROXY_PORT:-8899}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

# Banner
echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║     NetProxy Quick Start                       ║"
echo "║     One Command Setup                          ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Step 1: Build if needed
if [ ! -f "$DAEMON_BIN" ]; then
    log_info "Building NetProxy daemon..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin netproxyd
    log_success "Built successfully"
else
    log_success "Daemon binary exists"
fi

# Step 2: Create data directory
log_info "Setting up data directory..."
mkdir -p "$DATA_DIR"
mkdir -p "$DATA_DIR/run"
mkdir -p "$DATA_DIR/fixtures"
log_success "Data directory ready: $DATA_DIR"

# Step 3: Stop existing daemon if running
if [ -f "$DATA_DIR/run/daemon.lock" ]; then
    EXISTING_PID=$(cat "$DATA_DIR/run/daemon.lock" 2>/dev/null || echo "")
    if [ -n "$EXISTING_PID" ] && kill -0 "$EXISTING_PID" 2>/dev/null; then
        log_warn "Stopping existing daemon (PID $EXISTING_PID)..."
        kill "$EXISTING_PID" 2>/dev/null || true
        sleep 1
    fi
fi

# Step 4: Start daemon with autostart
log_info "Starting NetProxy daemon on port $PROXY_PORT..."
cd "$PROJECT_ROOT"
"$DAEMON_BIN" --data-dir "$DATA_DIR" --autostart &
DAEMON_PID=$!
sleep 2

# Verify daemon is running
if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    log_error "Daemon failed to start!"
    exit 1
fi
log_success "Daemon running (PID $DAEMON_PID)"

# Step 5: Export CA certificate
log_info "Exporting CA certificate..."
# Wait for daemon to be ready
sleep 1

# Use MCP tool to export cert (if available) or use direct API
if command -v "$PROJECT_ROOT/target/release/np-mcp" &> /dev/null; then
    # Try using MCP to export (would need to implement this flow)
    # For now, we'll use the control API directly via netcat/socat if available
    # OR we can just check if cert exists in data dir

    # The CA cert should be in $DATA_DIR/ca/ca.pem
    if [ -f "$DATA_DIR/ca/ca.pem" ]; then
        cp "$DATA_DIR/ca/ca.pem" "$CA_CERT"
        log_success "CA certificate exported to $CA_CERT"
    else
        log_warn "CA cert not found in data dir yet, waiting..."
        sleep 2
        if [ -f "$DATA_DIR/ca/ca.pem" ]; then
            cp "$DATA_DIR/ca/ca.pem" "$CA_CERT"
            log_success "CA certificate exported to $CA_CERT"
        else
            log_error "Could not find CA certificate!"
            log_info "Check: $DATA_DIR/ca/ca.pem"
        fi
    fi
else
    log_warn "MCP binary not found, checking for cert manually..."
    if [ -f "$DATA_DIR/ca/ca.pem" ]; then
        cp "$DATA_DIR/ca/ca.pem" "$CA_CERT"
        log_success "CA certificate exported to $CA_CERT"
    fi
fi

# Step 6: Detect active network interface
log_info "Detecting active network interface..."
NETWORK_INTERFACE=""

# Try to detect Wi-Fi first
if networksetup -getinfo "Wi-Fi" &>/dev/null; then
    NETWORK_INTERFACE="Wi-Fi"
elif networksetup -getinfo "Ethernet" &>/dev/null; then
    NETWORK_INTERFACE="Ethernet"
else
    # List all network services and pick the first active one
    NETWORK_INTERFACE=$(networksetup -listallnetworkservices | grep -v "^\*" | head -2 | tail -1)
fi

if [ -z "$NETWORK_INTERFACE" ]; then
    log_error "Could not detect active network interface!"
    log_info "Please run manually:"
    echo "  networksetup -setwebproxy \"<Interface>\" 127.0.0.1 $PROXY_PORT"
    echo "  networksetup -setsecurewebproxy \"<Interface>\" 127.0.0.1 $PROXY_PORT"
    exit 1
fi

log_success "Detected network interface: $NETWORK_INTERFACE"

# Step 7: Setup macOS system proxy
log_info "Setting up macOS system proxy..."
echo -n "   This will ask for your password: "

sudo networksetup -setwebproxy "$NETWORK_INTERFACE" 127.0.0.1 "$PROXY_PORT"
sudo networksetup -setsecurewebproxy "$NETWORK_INTERFACE" 127.0.0.1 "$PROXY_PORT"

log_success "System proxy configured"

# Step 8: Trust CA certificate
log_info "Installing and trusting CA certificate..."
if [ -f "$CA_CERT" ]; then
    sudo security add-trusted-cert -d trustRoot -k /Library/Keychains/System.keychain "$CA_CERT"
    log_success "CA certificate trusted"
else
    log_error "CA certificate not found at $CA_CERT"
    log_warn "You'll need to trust it manually later"
fi

# Step 9: Create basic mocks.yaml template (optional)
MOCKS_FILE="$DATA_DIR/mocks.yaml"
if [ ! -f "$MOCKS_FILE" ]; then
    log_info "Creating mocks.yaml template..."
    cat > "$MOCKS_FILE" <<'EOF'
version: 1
fixtures_dir: ./fixtures

rules:
  # Example mock rule - uncomment and modify as needed
  # - id: example-mock
  #   priority: 100
  #   enabled: false
  #   description: "Example mock rule"
  #   match:
  #     method: GET
  #     host: api.example.com
  #     path: /v1/users
  #   action:
  #     type: respond
  #     status: 200
  #     headers:
  #       content-type: application/json
  #     body:
  #       inline: '{"mocked": true, "users": []}'

EOF
    log_success "Created mocks.yaml template"
else
    log_success "mocks.yaml already exists"
fi

# Step 10: Create example fixture
EXAMPLE_FIXTURE="$DATA_DIR/fixtures/example.json"
if [ ! -f "$EXAMPLE_FIXTURE" ]; then
    cat > "$EXAMPLE_FIXTURE" <<'EOF'
{
  "mocked": true,
  "message": "This is an example fixture file",
  "timestamp": "2026-10-17T00:00:00Z"
}
EOF
    log_success "Created example fixture"
fi

# Summary
echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║     ✓ NetProxy is Ready!                      ║"
echo "╚════════════════════════════════════════════════╝"
echo ""
log_success "Daemon running on 127.0.0.1:$PROXY_PORT (PID $DAEMON_PID)"
log_success "System proxy configured for: $NETWORK_INTERFACE"
log_success "CA certificate trusted for HTTPS decryption"
log_success "Mock rules ready at: $MOCKS_FILE"
echo ""
echo "📋 Next Steps:"
echo ""
echo "  1. Test the proxy:"
echo "     curl https://httpbin.org/json"
echo ""
echo "  2. View captured traffic:"
echo "     Open NetProxy.app or use MCP tools"
echo ""
echo "  3. Create mock rules:"
echo "     Edit $MOCKS_FILE"
echo "     Then restart: kill $DAEMON_PID && $0"
echo ""
echo "  4. Configure iOS device:"
echo "     ./scripts/setup-ios.sh"
echo ""
echo "🛑 To Stop NetProxy:"
echo ""
echo "  ./scripts/netproxy-down.sh"
echo ""
echo "  Or manually:"
echo "  - Stop daemon: kill $DAEMON_PID"
echo "  - Clear proxy: networksetup -setwebproxystate \"$NETWORK_INTERFACE\" off"
echo "  - Clear HTTPS:  networksetup -setsecurewebproxystate \"$NETWORK_INTERFACE\" off"
echo ""

# Save PID and interface for down script
cat > "$DATA_DIR/run/quickstart.env" <<EOF
DAEMON_PID=$DAEMON_PID
NETWORK_INTERFACE=$NETWORK_INTERFACE
PROXY_PORT=$PROXY_PORT
EOF

log_success "Setup complete! 🎉"
echo ""
