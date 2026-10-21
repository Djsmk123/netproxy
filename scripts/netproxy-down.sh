#!/bin/bash
# NetProxy Quick Stop - Clean shutdown of everything

set -e

DATA_DIR="$HOME/Library/Application Support/NetProxy"
ENV_FILE="$DATA_DIR/run/quickstart.env"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║     NetProxy Shutdown                          ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Load saved environment if available
if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
    log_info "Loaded saved configuration"
else
    log_warn "No saved configuration found, will try to detect..."
    DAEMON_PID=""
    NETWORK_INTERFACE="Wi-Fi"
    PROXY_PORT="8899"
fi

# Step 1: Stop daemon
log_info "Stopping NetProxy daemon..."

# Try saved PID first
if [ -n "$DAEMON_PID" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    kill "$DAEMON_PID"
    log_success "Stopped daemon (PID $DAEMON_PID)"
else
    # Try to find by lock file
    if [ -f "$DATA_DIR/run/daemon.lock" ]; then
        PID=$(cat "$DATA_DIR/run/daemon.lock" 2>/dev/null || echo "")
        if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
            kill "$PID"
            log_success "Stopped daemon (PID $PID)"
        else
            log_warn "Daemon not running"
        fi
    else
        # Try finding netproxyd process
        PID=$(pgrep -f "netproxyd" || echo "")
        if [ -n "$PID" ]; then
            kill "$PID"
            log_success "Stopped daemon (PID $PID)"
        else
            log_warn "No daemon process found"
        fi
    fi
fi

# Step 2: Detect network interface if not saved
if [ -z "$NETWORK_INTERFACE" ]; then
    if networksetup -getinfo "Wi-Fi" &>/dev/null; then
        NETWORK_INTERFACE="Wi-Fi"
    elif networksetup -getinfo "Ethernet" &>/dev/null; then
        NETWORK_INTERFACE="Ethernet"
    else
        NETWORK_INTERFACE=$(networksetup -listallnetworkservices | grep -v "^\*" | head -2 | tail -1)
    fi
fi

# Step 3: Clear system proxy
log_info "Clearing macOS system proxy..."
if [ -n "$NETWORK_INTERFACE" ]; then
    sudo networksetup -setwebproxystate "$NETWORK_INTERFACE" off 2>/dev/null || true
    sudo networksetup -setsecurewebproxystate "$NETWORK_INTERFACE" off 2>/dev/null || true
    log_success "System proxy cleared for: $NETWORK_INTERFACE"
else
    log_warn "Could not detect network interface, proxy may still be set"
    echo "  Run manually: networksetup -setwebproxystate \"<Interface>\" off"
fi

# Step 4: Clean up environment file
if [ -f "$ENV_FILE" ]; then
    rm -f "$ENV_FILE"
fi

# Summary
echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║     ✓ NetProxy Stopped                         ║"
echo "╚════════════════════════════════════════════════╝"
echo ""
log_success "Daemon stopped"
log_success "System proxy disabled"
echo ""
log_info "Note: CA certificate is still trusted in System Keychain"
log_info "To remove it, open Keychain Access and delete 'NetProxy CA'"
echo ""
log_info "Data directory preserved: $DATA_DIR"
log_info "To remove all data: rm -rf '$DATA_DIR'"
echo ""
log_success "Shutdown complete!"
echo ""
