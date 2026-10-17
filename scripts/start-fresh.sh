#!/bin/bash
# Start NetProxy fresh (kills existing processes, starts new daemon)

set -e

cd "$(dirname "$0")/.."

echo "🚀 Starting NetProxy Fresh"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Kill existing processes
echo ""
echo "1️⃣  Cleaning up existing processes..."
./scripts/kill-netproxy-force.sh

# Build latest
echo ""
echo "2️⃣  Building latest release..."
cargo build --release --package netproxyd --quiet

# Start daemon
echo ""
echo "3️⃣  Starting daemon..."
DATA_DIR="${1:-./test-data}"
mkdir -p "$DATA_DIR"

nohup ./target/release/netproxyd --data-dir "$DATA_DIR" > netproxy-daemon.log 2>&1 &
DAEMON_PID=$!

echo "   Daemon PID: $DAEMON_PID"
echo "   Data directory: $DATA_DIR"
echo "   Log file: netproxy-daemon.log"

# Wait for startup
echo ""
echo "4️⃣  Waiting for daemon to start..."
sleep 2

# Check if running
if kill -0 $DAEMON_PID 2>/dev/null; then
    echo "   ✅ Daemon is running"

    # Show control info
    if [ -f "$DATA_DIR/run/control.sock" ]; then
        echo ""
        echo "📋 Control Info:"
        echo "   Socket: $DATA_DIR/run/control.sock"
        if [ -f "$DATA_DIR/run/control.token" ]; then
            TOKEN=$(cat "$DATA_DIR/run/control.token")
            echo "   Token: ${TOKEN:0:16}..."
        fi
    fi
else
    echo "   ❌ Daemon failed to start"
    echo ""
    echo "Log output:"
    tail -20 netproxy-daemon.log
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ NetProxy is ready!"
echo ""
echo "Next steps:"
echo "  • Run SwiftUI app: cd app && swift run"
echo "  • Or use Control API via: $DATA_DIR/run/control.sock"
echo ""
echo "To stop:"
echo "  ./scripts/kill-netproxy.sh"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
