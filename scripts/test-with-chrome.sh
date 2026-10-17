#!/bin/bash
# Test NetProxy with Chrome

set -e

cd "$(dirname "$0")/.."

echo "🧪 NetProxy + Chrome Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if daemon is running
if ! pgrep -f "netproxyd --data-dir ./test-data" > /dev/null; then
    echo ""
    echo "⚠️  NetProxy daemon not running!"
    echo "Starting fresh..."
    ./scripts/start-fresh.sh
    echo ""
fi

echo ""
echo "✅ Daemon is running"

# Check database for current flow count
FLOW_COUNT=$(sqlite3 ./test-data/netproxy.db "SELECT COUNT(*) FROM flows;" 2>/dev/null || echo "0")
echo "📊 Current flows in database: $FLOW_COUNT"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🌐 Launching Chrome with Proxy"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Proxy: http://127.0.0.1:8899"
echo "Profile: /tmp/chrome-netproxy (isolated)"
echo "SSL Errors: Ignored (for testing)"
echo ""

# Kill existing Chrome with this profile
pkill -f "user-data-dir=/tmp/chrome-netproxy" 2>/dev/null || true
sleep 1

# Launch Chrome
echo "Opening Chrome..."
open -a "Google Chrome" --args \
  --proxy-server='http://127.0.0.1:8899' \
  --ignore-certificate-errors \
  --user-data-dir=/tmp/chrome-netproxy \
  'https://httpbin.org/json'

echo ""
echo "✅ Chrome launched!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏳ Waiting for requests (10 seconds)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Wait a bit for traffic
sleep 10

# Check again
NEW_FLOW_COUNT=$(sqlite3 ./test-data/netproxy.db "SELECT COUNT(*) FROM flows;" 2>/dev/null || echo "0")
CAPTURED=$((NEW_FLOW_COUNT - FLOW_COUNT))

echo ""
if [ "$CAPTURED" -gt 0 ]; then
    echo "🎉 SUCCESS! Captured $CAPTURED new flow(s)"
    echo ""
    echo "Recent flows:"
    sqlite3 ./test-data/netproxy.db "SELECT method, host, path, status FROM flows ORDER BY flow DESC LIMIT 5;" 2>/dev/null | \
        awk -F'|' '{printf "   %s %s%s → %s\n", $1, $2, $3, $4}'

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✅ NetProxy is working!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Next steps:"
    echo "1. Open NetProxy app: cd app && swift run"
    echo "2. Click 'Start Recording' if not already started"
    echo "3. Select a flow from the list"
    echo "4. Click Response → Body"
    echo "5. JSON Viewer appears! 🎉"
    echo ""
    echo "Test more APIs:"
    echo "  https://api.github.com/users/djsmk123"
    echo "  https://jsonplaceholder.typicode.com/posts/1"
else
    echo "❌ No requests captured"
    echo ""
    echo "Troubleshooting:"
    echo ""
    echo "1. Make sure NetProxy app is running:"
    echo "   cd app && swift run"
    echo ""
    echo "2. Click 'Start Recording' (⌘R) in the app"
    echo ""
    echo "3. Check if port 8899 is listening:"
    echo "   lsof -i :8899"
    echo ""
    echo "4. Try the httpbin link again in Chrome"
    echo ""
    echo "5. Check daemon log:"
    echo "   tail -20 netproxy-daemon.log"
fi

echo ""
