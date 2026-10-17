#!/bin/bash
# Kill all NetProxy/netproxyd processes

set -e

echo "🔍 Searching for NetProxy processes..."

# Find all netproxyd processes
NETPROXYD_PIDS=$(pgrep -f netproxyd || true)

# Find all NetProxy app processes
NETPROXY_APP_PIDS=$(pgrep -f "NetProxy" | grep -v "netproxyd" || true)

# Count processes
TOTAL_PIDS=$(echo "$NETPROXYD_PIDS $NETPROXY_APP_PIDS" | wc -w | tr -d ' ')

if [ "$TOTAL_PIDS" -eq "0" ]; then
    echo "✅ No NetProxy processes running"
    exit 0
fi

echo "📋 Found $TOTAL_PIDS process(es):"
echo ""

# Show netproxyd processes
if [ -n "$NETPROXYD_PIDS" ]; then
    echo "netproxyd daemons:"
    ps -p $NETPROXYD_PIDS -o pid,ppid,command || true
    echo ""
fi

# Show NetProxy app processes
if [ -n "$NETPROXY_APP_PIDS" ]; then
    echo "NetProxy apps:"
    ps -p $NETPROXY_APP_PIDS -o pid,ppid,command || true
    echo ""
fi

# Ask for confirmation
read -p "❓ Kill all these processes? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Cancelled"
    exit 1
fi

# Kill processes
echo "🔨 Killing processes..."

if [ -n "$NETPROXYD_PIDS" ]; then
    echo "   Killing netproxyd daemons..."
    echo "$NETPROXYD_PIDS" | xargs kill -TERM 2>/dev/null || true
    sleep 1
    # Force kill if still running
    echo "$NETPROXYD_PIDS" | xargs kill -9 2>/dev/null || true
fi

if [ -n "$NETPROXY_APP_PIDS" ]; then
    echo "   Killing NetProxy apps..."
    echo "$NETPROXY_APP_PIDS" | xargs kill -TERM 2>/dev/null || true
    sleep 1
    # Force kill if still running
    echo "$NETPROXY_APP_PIDS" | xargs kill -9 2>/dev/null || true
fi

# Wait a moment for cleanup
sleep 1

# Verify all killed
REMAINING=$(pgrep -f "netproxy" || true)
if [ -z "$REMAINING" ]; then
    echo "✅ All NetProxy processes killed"
else
    echo "⚠️  Some processes may still be running:"
    ps -p $REMAINING -o pid,command || true
fi

# Clean up lock files
echo ""
echo "🧹 Cleaning up lock files..."

LOCK_FILES=$(find ~/Library/Application\ Support/NetProxy -name "daemon.lock" 2>/dev/null || true)
LOCK_FILES="$LOCK_FILES $(find /Users/smkwinner/Desktop/workspace/netproxy/test-data -name "daemon.lock" 2>/dev/null || true)"

if [ -n "$LOCK_FILES" ]; then
    echo "$LOCK_FILES" | while read -r lock; do
        if [ -f "$lock" ]; then
            echo "   Removing: $lock"
            rm -f "$lock"
        fi
    done
    echo "✅ Lock files cleaned"
else
    echo "✅ No lock files to clean"
fi

echo ""
echo "🎉 Done!"
