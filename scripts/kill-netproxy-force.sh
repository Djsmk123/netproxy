#!/bin/bash
# Force kill all NetProxy/netproxyd processes (no confirmation)

echo "🔨 Force killing all NetProxy processes..."

# Kill all netproxyd
pkill -9 -f netproxyd 2>/dev/null || true

# Kill all NetProxy apps
pkill -9 -f "NetProxy" 2>/dev/null || true

# Wait a moment
sleep 1

# Clean up lock files
find ~/Library/Application\ Support/NetProxy -name "daemon.lock" -delete 2>/dev/null || true
find /Users/smkwinner/Desktop/workspace/netproxy -name "daemon.lock" -delete 2>/dev/null || true

# Verify
REMAINING=$(pgrep -f "netproxy" || true)
if [ -z "$REMAINING" ]; then
    echo "✅ All NetProxy processes killed"
else
    echo "⚠️  Warning: Some processes still running:"
    ps -p $REMAINING -o pid,command || true
fi
